use std::ops::Range;
use std::sync::Arc;

use cgmath::{InnerSpace, SquareMatrix};
use float_ord::FloatOrd;
use hyperdraw::{GraphicsState, NdEuclidCamera, NdEuclidPuzzleRenderer};
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperprefs::{AnimationPreferences, Preferences};
use hyperpuzzle::prelude::*;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::{GizmoHoverState, NdEuclidPuzzleHoverState, PuzzleViewInput};
use crate::styles::PuzzleStyleStates;
use crate::{PuzzleSimulation, ReplayEvent};

/// Extra state for a view of an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidViewState {
    /// N-dimensional Euclidean puzzle geometry.
    pub geom: Arc<NdEuclidPuzzleGeometry>,

    /// Puzzle renderer.
    pub renderer: NdEuclidPuzzleRenderer,
    /// Camera defining how to view the puzzle.
    pub camera: NdEuclidCamera,

    /// Latest screen-space cursor position.
    pub cursor_pos: Option<cgmath::Point2<f32>>,
    /// Cursor drag state.
    pub drag_state: Option<DragState>,

    /// What puzzle geometry the cursor is hovering over. This is frozen during
    /// a drag.
    pub puzzle_hover_state: Option<NdEuclidPuzzleHoverState>,
    /// What twist gizmo the cursor is hovering over. This is frozen during a
    /// drag.
    pub gizmo_hover_state: Option<GizmoHoverState>,
}
impl NdEuclidViewState {
    /// Constructs a fresh state.
    ///
    /// Returns `None` if `puzzle` is not an N-dimensional Euclidean puzzle.
    pub fn new(
        gfx: &Arc<GraphicsState>,
        prefs: &mut Preferences,
        puzzle: &Arc<Puzzle>,
    ) -> Option<Self> {
        let geom = puzzle
            .ui_data
            .downcast_ref::<NdEuclidPuzzleUiData>()?
            .geom();
        let renderer = NdEuclidPuzzleRenderer::new(gfx, puzzle)?;

        let view_preset = prefs
            .perspective_view_presets(PerspectiveDim::from_ndim(geom.ndim()))
            .load_last_loaded_or_default(hyperprefs::DEFAULT_PRESET_NAME);

        Some(Self {
            renderer,
            camera: NdEuclidCamera {
                view_preset,
                target_size: [1, 1],
                rot: Motor::ident(geom.ndim()),
                zoom: 0.5,
            },

            geom,

            cursor_pos: None,
            drag_state: None,

            puzzle_hover_state: None,
            gizmo_hover_state: None,
        })
    }

    /// Resets the camera.
    pub fn reset_camera(&mut self) {
        self.camera.rot = Motor::ident(self.camera.rot.ndim());
    }

    /// Returns what the cursor was hovering over.
    pub fn puzzle_hover_state(&self) -> Option<NdEuclidPuzzleHoverState> {
        self.puzzle_hover_state.clone()
    }

    /// Returns the hovered twist gizmo element.
    pub fn gizmo_hover_state(&self) -> Option<GizmoHoverState> {
        self.gizmo_hover_state
    }

    /// Updates the puzzle view for a frame. This method is idempotent.
    pub fn update(
        &mut self,
        input: PuzzleViewInput,
        prefs: &Preferences,
        animation_prefs: &AnimationPreferences,
        sim: &Mutex<PuzzleSimulation>,
        styles: &PuzzleStyleStates,
    ) {
        let PuzzleViewInput {
            ndc_cursor_pos,
            target_size,
            is_dragging: _,
            exceeded_twist_drag_threshold,
            hover_mode: _,
        } = input;

        // Convert NDC to screen space.
        let cursor_pos = (|| {
            // IIFE to mimic try_block
            let [ndc_x, ndc_y] = ndc_cursor_pos?;
            let s = self.camera.xy_scale().ok()?;
            Some(cgmath::point2(ndc_x / s.x, ndc_y / s.y))
        })();
        // Update cursor position.
        let cursor_delta = Option::zip(cursor_pos, self.cursor_pos).map(|(old, new)| new - old);
        self.cursor_pos = cursor_pos;

        // Update drag state.
        if let Some(drag_state) = &mut self.drag_state {
            let mut sim = sim.lock();
            let puzzle = sim.puzzle();
            let nd_euclid = sim.nd_euclid();
            let ndim = match nd_euclid {
                Some(nd_euclid) => nd_euclid.geom.ndim(),
                None => 2,
            };

            match drag_state {
                // Update camera.
                DragState::ViewRot { z_axis } => {
                    if let Some(mut delta) = cursor_delta {
                        if *z_axis > 2 {
                            delta = -delta;
                        }
                        if *z_axis < ndim {
                            let cgmath::Vector2 { x: dx, y: dy } = delta;
                            self.camera.rot =
                                pga::Motor::from_angle_in_axis_plane(0, *z_axis, dx as _)
                                    * pga::Motor::from_angle_in_axis_plane(1, *z_axis, dy as _)
                                    * &self.camera.rot;
                        }
                    }
                }

                // Initialize partial twist state.
                DragState::PreTwist => {
                    if exceeded_twist_drag_threshold && ndim == 3 {
                        // IIFE to mimic try_block
                        let best_grip = (|| {
                            let hov = self.puzzle_hover_state()?;
                            let parallel_drag_delta = self.parallel_drag_delta()?;
                            let target = hov.normal_3d().cross_product_3d(&parallel_drag_delta);
                            sim.nd_euclid()?
                                .geom
                                .axis_vectors
                                .iter()
                                .filter_map(|(axis, axis_vector)| {
                                    // TODO: canoncalize axis based on layer mask
                                    let layers = puzzle.min_drag_layer_mask(axis, hov.piece)?;
                                    let score = target.dot(axis_vector.normalize()?).abs();
                                    if !APPROX.is_pos(score) {
                                        return None;
                                    }
                                    Some((axis, layers, score))
                                })
                                // TODO: handle multiple good matches, maybe?
                                .max_by_key(|(_, _, score)| FloatOrd(*score))
                                .map(|(axis, layers, _)| (axis, layers))
                        })();
                        if let Some((axis, layers)) = best_grip {
                            sim.begin_nd_euclid_partial_twist(ndim, axis, layers);
                            self.drag_state = Some(DragState::Twist);
                        } else {
                            log::trace!("canceling partial twist");
                            self.drag_state = Some(DragState::Canceled);
                        }
                    }
                }

                // Update partial twist state.
                DragState::Twist => {
                    // IIFE to mimic try_block
                    let partial_twist = (|| {
                        let hov = self.puzzle_hover_state()?;
                        let mut parallel_drag_delta = self.parallel_drag_delta()?;
                        let axis = nd_euclid?.partial_twist_drag_state.as_ref()?.axis;
                        let axis_vector = &nd_euclid?.geom.axis_vectors[axis];
                        let drag_origin = Point::ORIGIN; // TODO: change for multi-origin puzzles
                        if prefs.interaction.scale_twist_drag_by_radius {
                            parallel_drag_delta = parallel_drag_delta
                                / (hov.position.rejected_from(axis_vector)? - drag_origin).mag();
                        }
                        Some((hov.normal_3d(), parallel_drag_delta))
                    })();
                    if let Some((surface_normal, parallel_drag_delta)) = partial_twist {
                        sim.update_nd_euclid_partial_twist(
                            surface_normal,
                            parallel_drag_delta,
                            animation_prefs,
                        );
                    }
                }

                DragState::Canceled => (),
            }
        } else {
            // Update hover states, only when not in the middle of a drag.
            // IIFE to mimic try_block
            self.puzzle_hover_state = (|| {
                let vertex_3d_positions = self.renderer.puzzle_vertex_3d_positions.get()?;
                self.compute_sticker_hover_state(&vertex_3d_positions, prefs, styles, sim)
            })();
            self.gizmo_hover_state = (|| {
                let vertex_3d_positions = self.renderer.gizmo_vertex_3d_positions.get()?;
                self.compute_gizmo_hover_state(&vertex_3d_positions)
            })();
        }

        // Update camera.
        self.camera.target_size = target_size;
    }

    /// Returns the camera to use for drawing for one frame.
    pub fn transient_camera(&self, sim: &Mutex<PuzzleSimulation>) -> NdEuclidCamera {
        let mut cam = self.camera.clone();

        if let Some(t) = sim.lock().special_anim_t() {
            use std::f32::consts::{PI, TAU};

            let ndim = self.geom.ndim();
            let angle = (t * TAU) as Float;

            // Adjust view angle.
            let mut offset = pga::Motor::from_angle_in_axis_plane(0, 2, angle);
            let mut meta_offset = pga::Motor::from_angle_in_axis_plane(0, 1, -1.0);
            if ndim >= 4 {
                offset *= pga::Motor::from_angle_in_axis_plane(1, 3, angle);
                meta_offset *= pga::Motor::from_angle_in_axis_plane(2, 3, 1.0);
            }
            if ndim != 4 {
                meta_offset *= pga::Motor::from_angle_in_axis_plane(0, 1, 0.5 * angle);
            }
            offset = meta_offset.transform(&offset);
            cam.rot = offset * cam.rot;

            // Adjust piece explode
            let control_amount = 1.0 - (2.0 * t - 1.0).powf(12.0);
            let new_piece_explode = (t * PI).sin()
                * match ndim {
                    3 => 0.8,
                    _ => 0.0,
                };
            cam.view_preset.value.piece_explode = hypermath::util::lerp(
                cam.view_preset.value.piece_explode,
                new_piece_explode,
                control_amount,
            );

            if ndim >= 4 {
                let new_sticker_shrink = (t * PI).sin() * 0.5;
                cam.view_preset.value.sticker_shrink = hypermath::util::lerp(
                    cam.view_preset.value.sticker_shrink,
                    new_sticker_shrink,
                    control_amount,
                );
            }
        }

        cam
    }

    /// Computes the new puzzle hover state using the latest cursor position.
    #[must_use]
    fn compute_sticker_hover_state(
        &self,
        vertex_3d_positions: &[cgmath::Vector4<f32>],
        prefs: &Preferences,
        styles: &PuzzleStyleStates,
        sim: &Mutex<PuzzleSimulation>,
    ) -> Option<NdEuclidPuzzleHoverState> {
        let sim = sim.lock();
        let puzzle = sim.puzzle_type();

        let cursor_pos = self.cursor_pos?;

        let interactable_pieces = styles.interactable_pieces(prefs);

        let sticker_tri_ranges = self
            .geom
            .mesh
            .sticker_triangle_ranges
            .iter()
            .map(|(sticker, tri_range)| (puzzle.stickers[sticker].piece, Some(sticker), tri_range));

        let empty_internals_list = PerPiece::new();
        let internals_tri_ranges = if self.camera.prefs().show_internals {
            &self.geom.mesh.piece_internals_triangle_ranges
        } else {
            &empty_internals_list
        }
        .iter()
        .map(|(piece, tri_range)| (piece, None, tri_range));

        itertools::chain(sticker_tri_ranges, internals_tri_ranges)
            .filter(|(piece, _sticker, _tri_range)| interactable_pieces.contains(*piece))
            .flat_map(|(piece, sticker, tri_range)| {
                self.puzzle_triangle_hovers(
                    &sim,
                    cursor_pos,
                    piece,
                    sticker,
                    tri_range,
                    vertex_3d_positions,
                )
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    /// Computes the new gizmo hover state using the latest cursor position.
    #[must_use]
    fn compute_gizmo_hover_state(
        &self,
        vertex_3d_positions: &[cgmath::Vector4<f32>],
    ) -> Option<GizmoHoverState> {
        let cursor_pos = self.cursor_pos?;

        let gizmo_tri_ranges = self.geom.mesh.gizmo_triangle_ranges.iter();

        gizmo_tri_ranges
            .flat_map(|(gizmo, tri_range)| {
                self.gizmo_triangle_hover(cursor_pos, gizmo, tri_range, vertex_3d_positions)
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    /// Applies a twist to the puzzle based on the current mouse position.
    pub fn do_click_twist(&self, sim: &mut PuzzleSimulation, layers: LayerMask, direction: Sign) {
        let puzzle = sim.puzzle_type();

        if let Some(hov) = &self.gizmo_hover_state {
            let Ok(&target) = self.geom.gizmo_twists.get(hov.gizmo_face) else {
                return;
            };
            let transform = match direction {
                Sign::Neg => {
                    let Ok(twist_info) = puzzle.twists.twists.get(target) else {
                        return;
                    };
                    twist_info.reverse
                }
                Sign::Pos => target,
            };
            let twist = LayeredTwist { layers, transform };

            sim.do_event(ReplayEvent::GizmoClick {
                time: Some(hyperpuzzle::Timestamp::now()),
                layers,
                target,
                reverse: direction == Sign::Neg,
            });
            sim.do_event(ReplayEvent::Twists(smallvec![twist]));
        }
    }

    /// Completes a mouse drag.
    pub fn confirm_drag(&mut self, sim: &Mutex<PuzzleSimulation>) {
        if let Some(drag) = self.drag_state.take() {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => sim.lock().confirm_partial_twist(),
                DragState::Canceled => (),
            }
        }
    }
    /// Cancels a mouse drag.
    pub fn cancel_drag(&mut self, sim: &Mutex<PuzzleSimulation>) {
        if let Some(drag) = self.drag_state.replace(DragState::Canceled) {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => sim.lock().cancel_partial_twist(),
                DragState::Canceled => (),
            }
        }
    }

    /// Returns an approximation of the 3D vector along which the mouse has been
    /// dragged. This may return `None` even while a drag is happening.
    ///
    /// This is supposed to be parallel to the sticker face that the mouse
    /// initially clicked on.
    pub fn drag_delta_3d(&self) -> Option<[Point; 2]> {
        // TODO: where does this method want to live? does it want to exist at all?
        let a = self.puzzle_hover_state()?.position;
        let b = &a + self.parallel_drag_delta()?;
        Some([a, b])
    }

    /// Returns the triangles on the puzzle that contain the screen-space point
    /// `cursor_pos`.
    ///
    /// # Panics
    ///
    /// Panics if the puzzle backend isn't supported.
    fn puzzle_triangle_hovers<'a>(
        &'a self,
        puzzle_state: &'a PuzzleSimulation,
        cursor_pos: cgmath::Point2<f32>,
        piece: Piece,
        sticker: Option<Sticker>,
        tri_range: &'a Range<u32>,
        puzzle_vertex_3d_positions: &'a [cgmath::Vector4<f32>],
    ) -> impl 'a + Iterator<Item = NdEuclidPuzzleHoverState> {
        let mesh = &self.geom.mesh;
        let piece_transform = &puzzle_state
            .unwrap_render_data::<NdEuclidPuzzleStateRenderData>()
            .piece_transforms[piece];
        mesh.triangles[tri_range.start as usize..tri_range.end as usize]
            .iter()
            .filter_map(move |&vertex_ids| {
                let tri_verts @ [a, b, c] =
                    vertex_ids.map(|i| puzzle_vertex_3d_positions[i as usize]);
                // If the cursor isn't hovering the triangle, then
                // `triangle_hover_barycentric_coordinates()` returns `None`.
                let (barycentric_coords @ [qa, qb, qc], backface) =
                    crate::util::triangle_hover_barycentric_coordinates(cursor_pos, tri_verts)?;

                let [pa, pb, pc] = vertex_ids.map(|i| mesh.vertex_position(i));
                let position =
                    piece_transform.transform_point(pa * qa as _ + pb * qb as _ + pc * qc as _);

                let [ua, ub, uc] = vertex_ids.map(|i| mesh.u_tangent(i as _));
                let [va, vb, vc] = vertex_ids.map(|i| mesh.v_tangent(i as _));
                let u_tangent =
                    piece_transform.transform_vector(ua * qa as _ + ub * qb as _ + uc * qc as _);
                let v_tangent =
                    piece_transform.transform_vector(va * qa as _ + vb * qb as _ + vc * qc as _);

                Some(NdEuclidPuzzleHoverState {
                    cursor_pos,
                    z: qa * a.z + qb * b.z + qc * c.z,

                    piece,
                    sticker,

                    vertex_ids,
                    barycentric_coords,
                    backface,

                    position,
                    u_tangent,
                    v_tangent,
                })
            })
    }

    /// Returns the triangle on a gizmo face that contains the screen-space
    /// point `cursor_pos`, or `None` if there is none.
    fn gizmo_triangle_hover(
        &self,
        cursor_pos: cgmath::Point2<f32>,
        gizmo_face: GizmoFace,
        tri_range: &Range<u32>,
        gizmo_vertex_3d_positions: &[cgmath::Vector4<f32>],
    ) -> Option<GizmoHoverState> {
        let mesh = &self.geom.mesh;
        mesh.triangles[tri_range.start as usize..tri_range.end as usize]
            .iter()
            .find_map(move |&vertex_ids| {
                let tri_verts @ [a, b, c] =
                    vertex_ids.map(|i| gizmo_vertex_3d_positions[i as usize]);
                // If the cursor isn't hovering the triangle, then
                // `triangle_hover_barycentric_coordinates()` returns `None`.
                let (_barycentric_coords @ [qa, qb, qc], backface) =
                    crate::util::triangle_hover_barycentric_coordinates(cursor_pos, tri_verts)?;

                Some(GizmoHoverState {
                    z: qa * a.z + qb * b.z + qc * c.z,

                    gizmo_face,

                    backface,
                })
            })
    }

    fn parallel_drag_delta(&self) -> Option<Vector> {
        let initial_hover_state = self.puzzle_hover_state()?;

        let screen_space_delta = self.cursor_pos? - initial_hover_state.cursor_pos;
        let delta_2d = screen_space_delta;
        // Get the 3D position where the drag started.
        let drag_start = initial_hover_state.position.clone();
        // Get the tangent vectors at that position.
        let [u, v] = [
            &initial_hover_state.u_tangent,
            &initial_hover_state.v_tangent,
        ];
        // Project the tangent vectors onto the screen.
        let u_2d = self.camera.project_vector_to_screen_space(&drag_start, u)?;
        let v_2d = self.camera.project_vector_to_screen_space(&drag_start, v)?;
        // Convert the drag delta into the basis formed using the projected
        // tangent vectors, then use that to reconstruct the 3D vector.
        let screen_to_uv = cgmath::Matrix2::from_cols(u_2d, v_2d).invert()?;
        let delta_uv = screen_to_uv * delta_2d;
        let delta_3d = u * delta_uv.x as _ + v * delta_uv.y as _;

        Some(match delta_3d.normalize() {
            Some(v) => v * delta_2d.magnitude() as _ * crate::TWIST_DRAG_SPEED as _,
            None => vector![],
        })
    }
}

/// State of a mouse drag for an N-dimensional Euclidean puzzle.
#[derive(Debug, Copy, Clone)]
pub enum DragState {
    /// Rotating the camera.
    ViewRot {
        /// Which axis to exchange with X and Y.
        z_axis: u8,
    },
    /// Clicked and dragged on a piece. Once the user has dragged enough to
    /// determine a direction, the drag state will change to
    /// [`DragState::Twist`].
    PreTwist,
    /// Dragging a piece to twist.
    Twist,
    /// Drag canceled; ignore drag inputs until the mouse button is released
    /// again.
    Canceled,
}

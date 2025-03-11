use std::ops::Range;
use std::sync::Arc;

use cgmath::{InnerSpace, SquareMatrix};
use float_ord::FloatOrd;
use hyperdraw::{Camera, GfxEffectParams, GraphicsState, NdEuclidPuzzleRenderer};
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperprefs::{
    AnimationPreferences, ColorScheme, FilterPreset, FilterPresetName, FilterPresetRef, FilterRule,
    FilterSeqPreset, InterpolateFn, ModifiedPreset, Preferences, PresetRef,
    PuzzleFilterPreferences, PuzzleViewPreferencesSet,
};
use hyperpuzzle_core::{
    Axis, GizmoFace, LayerMask, LayeredTwist, NdEuclidPuzzleGeometry,
    NdEuclidPuzzleStateRenderData, PerPiece, Piece, PieceMask, Puzzle, Sticker,
};
use parking_lot::Mutex;
use smallvec::smallvec;

use super::ReplayEvent;
use super::simulation::PuzzleSimulation;
use super::styles::*;

/// View into a puzzle simulation, which has its own piece filters.
#[derive(Debug)]
pub struct PuzzleView {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub sim: Arc<Mutex<PuzzleSimulation>>,

    /// Puzzle renderer.
    pub renderer: NdEuclidPuzzleRenderer,

    /// Camera defining how to view the puzzle.
    pub camera: Camera,

    /// Current color scheme.
    pub colors: ModifiedPreset<ColorScheme>,
    /// Color scheme to apply for only the current frame.
    ///
    /// This is used to preview a change to a color scheme (particularly when
    /// hovering over UI elements that change the sticker colors when clicked).
    pub temp_colors: Option<ColorScheme>,
    /// Computed piece styles based on the filters state.
    pub styles: PuzzleStyleStates,
    /// Piece filters state.
    pub filters: PuzzleFiltersState,

    /// Latest screen-space cursor position.
    cursor_pos: Option<cgmath::Point2<f32>>,
    /// What puzzle geometry the cursor is hovering over. This is frozen during
    /// a drag.
    puzzle_hover_state: Option<PuzzleHoverState>,
    /// What twist gizmo the cursor is hovering over. This is frozen during a
    /// drag.
    gizmo_hover_state: Option<GizmoHoverState>,
    /// Axis whose twist gizmo should be highlighted for only the current frame.
    pub temp_gizmo_highlight: Option<Axis>,

    /// Whether to show the piece being hovered. This is updated every frame.
    pub show_puzzle_hover: bool,
    /// Whether to show the twist gizmo facet being hovered. This is updated
    /// every frame.
    pub show_gizmo_hover: bool,

    /// Cursor drag state.
    drag_state: Option<DragState>,
}
impl PuzzleView {
    /// Constructs a new puzzle view with an existing simulation.
    pub fn new(
        gfx: &Arc<GraphicsState>,
        sim: &Arc<Mutex<PuzzleSimulation>>,
        prefs: &mut Preferences,
    ) -> Self {
        let puz = Arc::clone(sim.lock().puzzle_type());
        let view_preset = prefs[PuzzleViewPreferencesSet::from_ndim(puz.ndim())]
            .load_last_loaded(hyperprefs::DEFAULT_PRESET_NAME);
        let colors = prefs
            .color_schemes
            .get_mut(&puz.colors)
            .schemes
            .load_last_loaded(hyperprefs::DEFAULT_PRESET_NAME);

        let simulation = sim.lock();
        let puzzle = simulation.puzzle_type();

        Self {
            sim: Arc::clone(sim),

            renderer: NdEuclidPuzzleRenderer::new(gfx, puzzle),

            camera: Camera {
                view_preset,
                target_size: [1, 1],
                rot: Motor::ident(puzzle.ndim()),
                zoom: 0.5,
            },

            colors,
            temp_colors: None,
            styles: PuzzleStyleStates::new(puzzle.pieces.len()),
            filters: PuzzleFiltersState::new(prefs.first_custom_style()),

            show_puzzle_hover: false,
            show_gizmo_hover: false,
            temp_gizmo_highlight: None,

            cursor_pos: None,
            puzzle_hover_state: None,
            gizmo_hover_state: None,
            drag_state: None,
        }
    }

    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(self.sim.lock().puzzle_type())
    }

    /// Returns what the cursor was hovering over.
    pub fn puzzle_hover_state(&self) -> Option<PuzzleHoverState> {
        self.puzzle_hover_state.clone()
    }
    /// Returns the hovered piece.
    fn hovered_piece(&self) -> Option<Piece> {
        Some(self.puzzle_hover_state.as_ref()?.piece)
    }

    /// Returns the hovered twist gizmo element.
    pub fn gizmo_hover_state(&self) -> Option<GizmoHoverState> {
        self.gizmo_hover_state.clone()
    }

    /// Sets the mouse drag state.
    pub fn set_drag_state(&mut self, new_drag_state: DragState) {
        self.confirm_drag();
        self.drag_state = Some(new_drag_state);
    }
    /// Returns the mouse drag state.
    pub fn drag_state(&self) -> Option<DragState> {
        self.drag_state
    }
    /// Completes a mouse drag.
    pub fn confirm_drag(&mut self) {
        if let Some(drag) = self.drag_state.take() {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => self.sim.lock().confirm_partial_twist(),
                DragState::Canceled => (),
            }
        }
    }
    /// Cancels a mouse drag.
    pub fn cancel_drag(&mut self) {
        if let Some(drag) = self.drag_state.replace(DragState::Canceled) {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => self.sim.lock().cancel_partial_twist(),
                DragState::Canceled => (),
            }
        }
    }
    /// Returns an approximation of the 3D vector along which the mouse has been
    /// dragged. This may return `None` even while a drag is happening.
    ///
    /// This is supposed to be parallel to the sticker face that the mouse
    /// initially clicked on.
    pub fn drag_delta_3d(&self) -> Option<[Vector; 2]> {
        // TODO: where does this method want to live? does it want to exist at all?
        let a = self.puzzle_hover_state()?.position;
        let b = &a + self.parallel_drag_delta()?;
        Some([a, b])
    }

    /// Updates the current piece filters.
    fn notify_filters_changed(&mut self) {
        let all_pieces = PieceMask::new_full(self.puzzle().pieces.len());

        let fallback_style = self.filters.fallback_style().clone();
        self.styles.set_base_styles(&all_pieces, fallback_style);

        let main_rules = self.filters.iter_active_rules();
        let fallback_rules = self
            .filters
            .combined_fallback_preset
            .iter()
            .flat_map(|f| &f.rules);
        let rules = itertools::chain(main_rules, fallback_rules);

        // Iterate in an order such that later rules override earlier ones.
        for rule in rules.rev() {
            let pieces = rule.set.eval(&self.puzzle());
            self.styles.set_base_styles(&pieces, rule.style.clone());
        }

        for rule in self.filters.iter_active_rules().rev() {
            let pieces = rule.set.eval(&self.puzzle());
            self.styles.set_base_styles(&pieces, rule.style.clone());
        }
    }

    /// Updates the puzzle view for a frame. This method is idempotent.
    pub fn update(
        &mut self,
        input: PuzzleViewInput,
        prefs: &Preferences,
        animation_prefs: &AnimationPreferences,
    ) {
        if self.filters.changed {
            self.filters.changed = false;
            self.notify_filters_changed();
        }

        let PuzzleViewInput {
            cursor_pos,
            target_size,
            puzzle_vertex_3d_positions,
            gizmo_vertex_3d_positions,
            exceeded_twist_drag_threshold,
            hover_mode,
        } = input;

        self.show_puzzle_hover = hover_mode == Some(HoverMode::Piece)
            && self.drag_state.is_none()
            && self.sim.lock().partial_twist().is_none()
            && !self.sim.lock().has_twist_anim_queued();
        self.show_gizmo_hover =
            hover_mode == Some(HoverMode::TwistGizmo) && self.drag_state.is_none();

        let ndim = self.puzzle().ndim();

        // Update cursor position.
        let cursor_delta = Option::zip(cursor_pos, self.cursor_pos).map(|(old, new)| new - old);
        self.cursor_pos = cursor_pos;

        // Update drag state.
        if let Some(drag_state) = &mut self.drag_state {
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
                                pga::Motor::from_angle_in_axis_plane(ndim, 0, *z_axis, dx as _)
                                    * pga::Motor::from_angle_in_axis_plane(
                                        ndim, 1, *z_axis, dy as _,
                                    )
                                    * &self.camera.rot;
                        }
                    }
                }

                // Initialize partial twist state.
                DragState::PreTwist => {
                    if exceeded_twist_drag_threshold && ndim == 3 {
                        let mut sim = self.sim.lock();
                        let puzzle = sim.puzzle();
                        // IIFE to mimic try_block
                        let best_grip = (|| {
                            let hov = self.puzzle_hover_state()?;
                            let parallel_drag_delta = self.parallel_drag_delta()?;
                            let target = hov.normal_3d().cross_product_3d(&parallel_drag_delta);
                            puzzle
                                .ty()
                                .ui_data
                                .downcast_ref::<NdEuclidPuzzleGeometry>()?
                                .axis_vectors
                                .iter()
                                .filter_map(|(axis, axis_vector)| {
                                    // TODO: canoncalize axis based on layer mask
                                    let layers = puzzle.min_drag_layer_mask(axis, hov.piece)?;
                                    let score = target.dot(axis_vector.normalize()?).abs();
                                    if !is_approx_positive(&score) {
                                        return None;
                                    }
                                    Some((axis, layers, score))
                                })
                                // TODO: handle multiple good matches, maybe?
                                .max_by_key(|(_, _, score)| FloatOrd(*score))
                                .map(|(axis, layers, _)| (axis, layers))
                        })();
                        if let Some((axis, layers)) = best_grip {
                            sim.begin_partial_twist(axis, layers);
                            self.drag_state = Some(DragState::Twist);
                        } else {
                            log::trace!("canceling partial twist");
                            self.drag_state = Some(DragState::Canceled);
                        }
                    }
                }

                // Update partial twist state.
                DragState::Twist => {
                    (|| {
                        // IIFE to mimic try_block
                        let hov = self.puzzle_hover_state()?;
                        let mut parallel_drag_delta = self.parallel_drag_delta()?;
                        let mut sim = self.sim.lock();
                        let axis = sim.partial_twist().as_ref()?.axis;
                        let puzzle = sim.puzzle_type();
                        let geom = puzzle.ui_data.downcast_ref::<NdEuclidPuzzleGeometry>()?;
                        let axis_vector = &geom.axis_vectors[axis];
                        if prefs.interaction.scale_twist_drag_by_radius {
                            parallel_drag_delta = parallel_drag_delta
                                / hov.position.rejected_from(axis_vector)?.mag();
                        }
                        sim.update_partial_twist(
                            hov.normal_3d(),
                            parallel_drag_delta,
                            animation_prefs,
                        );
                        Some(())
                    })();
                }

                DragState::Canceled => (),
            }
        } else {
            // Update hover states, only when not in the middle of a drag.
            self.puzzle_hover_state = puzzle_vertex_3d_positions.and_then(|vertex_3d_positions| {
                self.compute_sticker_hover_state(&vertex_3d_positions, prefs)
            });
            self.gizmo_hover_state = gizmo_vertex_3d_positions.and_then(|vertex_3d_positions| {
                self.compute_gizmo_hover_state(&vertex_3d_positions)
            });
        }

        // Update hovered piece.
        self.styles
            .set_hovered_piece(self.hovered_piece().filter(|_| self.show_puzzle_hover));

        // Update blocking state.
        {
            let puzzle = self.puzzle();
            let sim = self.sim.lock();
            let anim = sim.blocking_pieces_anim();
            let amt = anim.blocking_amount(animation_prefs);
            let pieces = PieceMask::from_iter(puzzle.pieces.len(), anim.pieces().iter().copied());
            self.styles.set_blocking_pieces(pieces, amt);
        }

        // Update camera.
        self.camera.target_size = target_size;
    }

    /// Returns the camera to use for drawing for one frame.
    pub fn transient_camera(&self) -> Camera {
        let mut cam = self.camera.clone();

        if let Some(t) = self.special_anim_t() {
            use std::f32::consts::{PI, TAU};

            let ndim = self.puzzle().ndim();
            let angle = (t * TAU) as Float;

            // Adjust view angle.
            let mut offset = pga::Motor::from_angle_in_axis_plane(ndim, 0, 2, angle);
            let mut meta_offset = pga::Motor::from_angle_in_axis_plane(ndim, 0, 1, -1.0);
            if ndim >= 4 {
                offset *= pga::Motor::from_angle_in_axis_plane(ndim, 1, 3, angle);
                meta_offset *= pga::Motor::from_angle_in_axis_plane(ndim, 2, 3, 1.0);
                // meta_offset *= pga::Motor::from_angle_in_axis_plane(ndim, 0,
                // 3, 0.25 * angle);
            }
            if ndim != 4 {
                meta_offset *= pga::Motor::from_angle_in_axis_plane(ndim, 0, 1, 0.5 * angle);
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

    /// Returns the effects to use for drawing for one frame.
    pub fn effects(&self) -> GfxEffectParams {
        if let Some(t) = self.special_anim_t() {
            use std::f32::consts::PI;

            let amount = (t * PI).sin();

            GfxEffectParams {
                chromatic_abberation: [amount / 4.0, amount / 6.0],
            }
        } else {
            GfxEffectParams::default()
        }
    }

    fn special_anim_t(&self) -> Option<f32> {
        self.sim
            .lock()
            .special_anim()
            .get()
            .map(|t| InterpolateFn::Cosine.interpolate(t))
    }

    /// Computes the new puzzle hover state using the latest cursor position.
    #[must_use]
    fn compute_sticker_hover_state(
        &self,
        vertex_3d_positions: &[cgmath::Vector4<f32>],
        prefs: &Preferences,
    ) -> Option<PuzzleHoverState> {
        let puzzle = self.puzzle();
        let geom = puzzle.ui_data.downcast_ref::<NdEuclidPuzzleGeometry>()?;

        let cursor_pos = self.cursor_pos?;

        let interactable_pieces = self.styles.interactable_pieces(prefs);

        let sticker_tri_ranges = geom
            .mesh
            .sticker_triangle_ranges
            .iter()
            .map(|(sticker, tri_range)| (puzzle.stickers[sticker].piece, Some(sticker), tri_range));

        let empty_internals_list = PerPiece::new();
        let internals_tri_ranges = if self.camera.prefs().show_internals {
            &geom.mesh.piece_internals_triangle_ranges
        } else {
            &empty_internals_list
        }
        .iter()
        .map(|(piece, tri_range)| (piece, None, tri_range));

        let sim = self.sim.lock();

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
        let puzzle = self.puzzle();
        let geom = puzzle.ui_data.downcast_ref::<NdEuclidPuzzleGeometry>()?;

        let cursor_pos = self.cursor_pos?;

        let gizmo_tri_ranges = geom.mesh.gizmo_triangle_ranges.iter();

        gizmo_tri_ranges
            .flat_map(|(gizmo, tri_range)| {
                self.gizmo_triangle_hover(cursor_pos, gizmo, tri_range, vertex_3d_positions)
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    /// Resets the camera.
    pub fn reset_camera(&mut self) {
        self.camera.rot = Motor::ident(self.puzzle().ndim());
    }

    /// Applies a twist to the puzzle based on the current mouse position.
    pub fn do_click_twist(&self, layers: LayerMask, direction: Sign) {
        let mut state = self.sim.lock();
        let puzzle = state.puzzle_type();
        let ndim = puzzle.ndim();

        let Some(geom) = puzzle.ui_data.downcast_ref::<NdEuclidPuzzleGeometry>() else {
            return;
        };

        if let Some(hov) = &self.gizmo_hover_state {
            let Ok(&target) = geom.gizmo_twists.get(hov.gizmo_face) else {
                return;
            };
            let transform = match direction {
                Sign::Neg => {
                    let Ok(twist_info) = puzzle.twists.get(target) else {
                        return;
                    };
                    twist_info.reverse
                }
                Sign::Pos => target,
            };
            let twist = LayeredTwist { layers, transform };

            state.do_event(ReplayEvent::GizmoClick {
                layers,
                target,
                reverse: direction == Sign::Neg,
            });
            state.do_event(ReplayEvent::Twists(smallvec![twist]));
        } else if let Some(hov) = &self.puzzle_hover_state {
            if puzzle.ndim() == 3 {
                // Only do a move if we are hovering a sticker.
                if hov.sticker.is_none() {
                    return;
                }

                // Find the axis aligned with the normal vector of this
                // sticker.
                let [u, v] = [&hov.u_tangent, &hov.v_tangent];
                let target_vector = Vector::cross_product_3d(u, v);
                // TODO: should axis vectors already be normalized?
                let Some(axis) = geom.axis_vectors.find(|_, axis_vector| {
                    axis_vector
                        .normalize()
                        .is_some_and(|v| approx_eq(&v, &target_vector))
                }) else {
                    return;
                };

                // Find the twist that turns the least in the correct direction.
                // TODO: search only twists on `axis`
                let candidates = puzzle.twists.iter_filter(|_, info| info.axis == axis);

                // Aim for a 180 degree counterclockwise rotation around the axis.
                let target = match hov.backface {
                    false => Motor::from_normalized_vector_product(ndim, v, u),
                    true => Motor::from_normalized_vector_product(ndim, u, v),
                };
                let best_twist = candidates.min_by_key(|&twist| {
                    // `score` ranges from -1 to +1. If it's a positive number,
                    // then the twist goes in the desired direction; if it's
                    // negative, then it goes in the other direction. `score` is
                    // larger if the twist travels through a larger angle:
                    // - no rotation = 0
                    // - 180-degree rotation = ±1
                    let score = Motor::dot(&geom.twist_transforms[twist], &target);
                    (Sign::from(score) * direction, FloatOrd(score.abs()))
                });
                if let Some(transform) = best_twist {
                    let twist = LayeredTwist { layers, transform };
                    state.do_event(ReplayEvent::DragTwist);
                    state.do_event(ReplayEvent::Twists(smallvec![twist]));
                }
            }
        }
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
    ) -> impl 'a + Iterator<Item = PuzzleHoverState> {
        let geom = puzzle_state
            .puzzle_type()
            .ui_data
            .downcast_ref::<NdEuclidPuzzleGeometry>()
            .expect("unexpected type for PuzzleTypeGpuData");
        let mesh = &geom.mesh;
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

                Some(PuzzleHoverState {
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
        let puzzle_state = self.sim.lock();
        let geom = puzzle_state
            .puzzle_type()
            .ui_data
            .downcast_ref::<NdEuclidPuzzleGeometry>()?;
        let mesh = &geom.mesh;
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

    /// Returns the color value for a given puzzle color, ignoring temporary
    /// per-frame overrides.
    pub fn get_rgb_color(
        &self,
        color: hyperpuzzle_core::Color,
        prefs: &Preferences,
    ) -> Option<hyperpuzzle_core::Rgb> {
        let default_color = self.colors.value.get_index(color.0 as usize)?.1;
        prefs.color_palette.get(default_color)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PuzzleHoverState {
    /// Screen-space cursor coordinates within the puzzle view.
    pub cursor_pos: cgmath::Point2<f32>,
    /// Screen-space Z coordinate.
    z: f32,

    /// Piece being hovered.
    pub piece: Piece,
    /// Sticker being hovered. If this is `None`, then an internal facet of the
    /// piece is being hovered.
    pub sticker: Option<Sticker>,

    /// IDs of the vertices of the hovered triangle.
    vertex_ids: [u32; 3],
    /// Barycentric coordinates on the hovered triangle.
    barycentric_coords: [f32; 3],
    /// Whether the backface of the surface is being hovered (as opposed to the
    /// frontface). This primarily matters in 3D, where stickers are oriented.
    pub backface: bool,

    /// Exact hovered location on the surface of the puzzle, in puzzle space,
    /// after undoing geometry modifications such as sticker shrink and piece
    /// explode.
    pub position: Vector,
    /// First tangent vector of the hovered surface, in puzzle space.
    pub u_tangent: Vector,
    /// Second tangent vector of the hovered surface, in puzzle space.
    pub v_tangent: Vector,
}
impl PuzzleHoverState {
    /// Returns the normal vector to the hovered surface, which is only valid in
    /// 3D.
    pub fn normal_3d(&self) -> Vector {
        self.u_tangent.cross_product_3d(&self.v_tangent)
    }
}

/// Hovered twist gizmo element.
#[derive(Debug, Clone, PartialEq)]
pub struct GizmoHoverState {
    /// Screen-space Z coordinate.
    pub z: f32,

    /// Gizmo face being hovered.
    pub gizmo_face: GizmoFace,

    /// Whether the backface of the gizmo is being hovered (as opposed to the
    /// frontface).
    ///
    /// TODO: check that this is correct -- I'm not sure the gizmo mesh
    ///       construction checks face orientation
    pub backface: bool,
}

/// State of a mouse drag.
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

/// Input data for a puzzle view for one frame.
pub struct PuzzleViewInput {
    /// Position of the cursor on the puzzle view, in screen space.
    pub cursor_pos: Option<cgmath::Point2<f32>>,
    /// Size of the target to draw to.
    pub target_size: [u32; 2],
    /// 3D positions of vertices in the puzzle mesh.
    pub puzzle_vertex_3d_positions: Option<Arc<Vec<cgmath::Vector4<f32>>>>,
    /// 3D positions of vertices in the twist gizmos mesh.
    pub gizmo_vertex_3d_positions: Option<Arc<Vec<cgmath::Vector4<f32>>>>,
    /// Whether the cursor has been dragged enough to begin a drag twist, if
    /// that's the type of drag happening.
    pub exceeded_twist_drag_threshold: bool,
    /// What the mouse can hover over.
    pub hover_mode: Option<HoverMode>,
}

/// Which kind of objects the user may interact with by hovering with the mouse.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum HoverMode {
    /// Pieces of the puzzle.
    #[default]
    Piece,
    /// Twist gizmos.
    TwistGizmo,
}

/// Piece filters state for a puzzle view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleFiltersState {
    /// Reference to the saved filter preset, if any.
    pub base: Option<FilterPresetRef>,
    /// Filter preset data.
    pub current: FilterSeqPreset,
    /// Combination of all fallback rules to apply to pieces not specified by
    /// the current preset.
    pub combined_fallback_preset: Option<FilterPreset>,
    /// For each rule: whether it is active. Inactive rules are ignored when
    /// displaying the puzzle.
    pub active_rules: Vec<bool>,

    /// Whether the piece filters have changed since the last frame.
    changed: bool,
}
impl PuzzleFiltersState {
    /// Returns a new empty filters state with no rules and no fallback style.
    pub fn new_empty() -> Self {
        Self {
            base: None,
            current: FilterSeqPreset::new_empty(),
            combined_fallback_preset: None,
            active_rules: vec![],
            changed: true,
        }
    }

    /// Returns a new default filters state with a single rule (to show all
    /// pieces in default state) and an optional fallback style.
    pub fn new(fallback_style: Option<PresetRef>) -> Self {
        Self {
            base: None,
            current: FilterSeqPreset::new_with_single_rule(fallback_style),
            combined_fallback_preset: None,
            active_rules: vec![],
            changed: true,
        }
    }

    /// Iterates over active rules, skipping inactive ones.
    pub fn iter_active_rules(&self) -> impl DoubleEndedIterator<Item = &FilterRule> {
        self.current
            .inner
            .rules
            .iter()
            .enumerate()
            .filter(|(i, _rule)| *self.active_rules.get(*i).unwrap_or(&true))
            .map(|(_i, rule)| rule)
    }

    /// Loads a filter preset, overwriting the current state completely.
    pub fn load_preset(
        &mut self,
        filter_prefs: &PuzzleFilterPreferences,
        name: Option<&FilterPresetName>,
    ) {
        // IIFE to mimic try_block
        match (|| Some((name?, filter_prefs.get(name?)?)))() {
            Some((name, current)) => {
                let preset_ref = filter_prefs.new_ref(name);
                let fallback = filter_prefs.combined_fallback_preset(&preset_ref.name());

                *self = Self {
                    base: Some(preset_ref),
                    current,
                    combined_fallback_preset: fallback,
                    active_rules: vec![],
                    changed: true,
                };
            }
            None => {
                *self = Self {
                    base: None,
                    current: std::mem::take(&mut self.current),
                    combined_fallback_preset: None,
                    active_rules: std::mem::take(&mut self.active_rules),
                    changed: true,
                }
            }
        }
    }
    /// Reloads the current filter preset, overwriting the current state
    /// completely.
    pub fn reload(&mut self, filter_prefs: &PuzzleFilterPreferences) {
        let name = self.base.as_ref().map(|r| r.name());
        self.load_preset(filter_prefs, name.as_ref());
    }

    /// Updates the combined fallback preset.
    pub fn update_combined_fallback_preset(&mut self, filter_prefs: &PuzzleFilterPreferences) {
        if let Some(base) = &self.base {
            let new_fallback = filter_prefs.combined_fallback_preset(&base.name());
            if new_fallback != self.combined_fallback_preset {
                self.combined_fallback_preset = new_fallback;
                self.changed = true;
            }
        } else if self.combined_fallback_preset.is_some() {
            self.combined_fallback_preset = None;
            self.changed = true;
        }
    }

    /// Returns the ultimate fallback style.
    fn fallback_style(&self) -> &Option<PresetRef> {
        match &self.combined_fallback_preset {
            Some(p) => &p.fallback_style,
            None => &self.current.inner.fallback_style,
        }
    }

    /// Marks the filters as having changed, indicating that the puzzle view
    /// should recompute piece styles on the next frame.
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }
}

use std::ops::Range;
use std::sync::Arc;

use cgmath::InnerSpace;
use cgmath::SquareMatrix;
use float_ord::FloatOrd;
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperpuzzle::PieceMask;
use hyperpuzzle::{LayerMask, PerPiece, Piece, Puzzle, Sticker};
use parking_lot::Mutex;

use super::simulation::PuzzleSimulation;
use super::styles::*;
use super::Camera;
use crate::preferences::{Preferences, ViewPreferences};

/// View into a puzzle simulation, which has its own piece filters.
#[derive(Debug)]
pub struct PuzzleView {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub sim: Arc<Mutex<PuzzleSimulation>>,

    pub camera: Camera,

    pub styles: PuzzleStyleStates,

    /// Latest screen-space cursor position.
    cursor_pos: Option<cgmath::Point2<f32>>,
    /// What the cursor is hovering over. This is frozen during a drag.
    hover_state: Option<HoverState>,
    /// Cursor drag state.
    drag_state: Option<DragState>,
}
impl PuzzleView {
    pub(crate) fn new(puzzle_simulation: &Arc<Mutex<PuzzleSimulation>>) -> Self {
        let simulation = puzzle_simulation.lock();
        let puzzle = simulation.puzzle_type();
        Self {
            sim: Arc::clone(puzzle_simulation),

            camera: Camera {
                prefs: ViewPreferences::default(),
                target_size: [1, 1],
                rot: Motor::ident(puzzle.ndim()),
                zoom: 0.5,
            },

            styles: PuzzleStyleStates::new(puzzle.pieces.len()),

            cursor_pos: None,
            hover_state: None,
            drag_state: None,
        }
    }

    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.sim.lock().puzzle_type())
    }

    /// Returns what the cursor was hovering over.
    pub fn hover_state(&self) -> Option<HoverState> {
        self.hover_state.clone()
    }
    /// Returns the hovered piece.
    fn hovered_piece(&self) -> Option<Piece> {
        Some(self.hover_state.as_ref()?.piece)
    }

    pub fn set_drag_state(&mut self, new_drag_state: DragState) {
        self.confirm_drag();
        match new_drag_state {
            DragState::ViewRot { .. } => (),
            DragState::PreTwist => (),
            DragState::Twist => (),
        }
        self.drag_state = Some(new_drag_state);
    }
    pub fn drag_state(&self) -> Option<DragState> {
        self.drag_state
    }
    pub fn confirm_drag(&mut self) {
        if let Some(drag) = self.drag_state.take() {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => self.sim.lock().confirm_partial_twist(),
            }
        }
    }
    pub fn cancel_drag(&mut self) {
        if let Some(drag) = self.drag_state.take() {
            match drag {
                DragState::ViewRot { .. } => (),
                DragState::PreTwist => (),
                DragState::Twist => self.sim.lock().cancel_partial_twist(),
            }
        }
    }
    pub fn drag_delta_3d(&self) -> Option<[Vector; 2]> {
        // TODO: where does this method want to live? does it want to exist at all?
        let a = self.hover_state()?.position;
        let b = &a + self.parallel_drag_delta()?;
        Some([a, b])
    }

    /// Updates the puzzle view for a frame. This method is idempotent.
    pub fn update(&mut self, input: PuzzleViewInput<'_>) {
        let PuzzleViewInput {
            cursor_pos,
            target_size,
            vertex_3d_positions,
            prefs,
            exceeded_twist_drag_threshold,
        } = input;

        let ndim = self.puzzle().ndim();

        // Update cursor position.
        let cursor_delta = Option::zip(cursor_pos, self.cursor_pos).map(|(old, new)| new - old);
        self.cursor_pos = cursor_pos;

        // Update drag state.
        if let Some(drag_state) = &mut self.drag_state {
            match drag_state {
                // Update camera.
                DragState::ViewRot { z_axis } => {
                    if let Some(delta) = cursor_delta {
                        let cgmath::Vector2 { x: dx, y: dy } = delta;
                        self.camera.rot =
                            pga::Motor::from_angle_in_axis_plane(ndim, 0, *z_axis, dx as _)
                                * pga::Motor::from_angle_in_axis_plane(ndim, 1, *z_axis, dy as _)
                                * &self.camera.rot;
                    }
                }

                // Initialize partial twist state.
                DragState::PreTwist => {
                    if exceeded_twist_drag_threshold {
                        match ndim {
                            3 => {
                                let mut sim = self.sim.lock();
                                let puzzle = sim.puzzle();
                                // IIFE to mimic try_block
                                let best_grip = (|| {
                                    let hov = self.hover_state()?;
                                    let parallel_drag_delta = self.parallel_drag_delta()?;
                                    let target =
                                        hov.normal_3d().cross_product_3d(&parallel_drag_delta);
                                    puzzle
                                        .ty()
                                        .axes
                                        .iter()
                                        .filter_map(|(axis, info)| {
                                            let layers = puzzle.min_layer_mask(axis, hov.piece)?;
                                            let score = target.dot(info.vector.normalize()?).abs();
                                            if !is_approx_positive(&score) {
                                                return None;
                                            }
                                            Some((axis, layers, score))
                                        })
                                        .max_by_key(|(_, _, score)| FloatOrd(*score))
                                        .map(|(axis, layers, _)| (axis, layers))
                                })();
                                if let Some((axis, layers)) = best_grip {
                                    sim.begin_partial_twist(axis, layers);
                                    self.drag_state = Some(DragState::Twist);
                                } else {
                                    log::trace!("canceling partial twist");
                                    self.drag_state = None;
                                }
                            }
                            _ => (),
                        }
                    }
                }

                // Update partial twist state.
                DragState::Twist => {
                    (|| {
                        // IIFE to mimic try_block
                        let hov = self.hover_state()?;
                        let parallel_drag_delta = self.parallel_drag_delta()?;
                        self.sim
                            .lock()
                            .update_partial_twist(hov.normal_3d(), parallel_drag_delta);
                        Some(())
                    })();
                }
            }
        } else {
            // Update hover state, only when not in the middle of a drag.
            let old_hovered_piece = self.hovered_piece();
            self.hover_state = vertex_3d_positions.and_then(|vertex_3d_positions| {
                self.compute_hover_state(&vertex_3d_positions, prefs)
            });
            let new_hovered_piece = self.hovered_piece();
            if old_hovered_piece != new_hovered_piece {
                self.styles.set_hovered_piece(new_hovered_piece);
            }
        }

        // Update blocking state.
        {
            let puzzle = self.puzzle();
            let sim = self.sim.lock();
            let anim = sim.blocking_pieces_anim();
            let amt = anim.blocking_amount(&prefs.interaction);
            let pieces = PieceMask::from_iter(puzzle.pieces.len(), anim.pieces().iter().copied());
            self.styles.set_blocking_pieces(pieces, amt);
        }

        // Update camera.
        {
            self.camera.prefs = prefs.view(&self.puzzle()).clone();
            self.camera.target_size = target_size;
        }
    }

    /// Computes the new hover state from the pixel position of the cursor.
    #[must_use]
    fn compute_hover_state(
        &self,
        vertex_3d_positions: &[cgmath::Vector4<f32>],
        prefs: &Preferences,
    ) -> Option<HoverState> {
        let puzzle = self.puzzle();

        let cursor_pos = self.cursor_pos?;

        let interactable_pieces = self.styles.interactable_pieces(&prefs.styles);

        let sticker_tri_ranges = puzzle
            .mesh
            .sticker_triangle_ranges
            .iter()
            .map(|(sticker, tri_range)| (puzzle.stickers[sticker].piece, Some(sticker), tri_range));

        let empty_internals_list = PerPiece::new();
        let internals_tri_ranges = if prefs.view(&puzzle).show_internals {
            &puzzle.mesh.piece_internals_triangle_ranges
        } else {
            &empty_internals_list
        }
        .iter()
        .map(|(piece, tri_range)| (piece, None, tri_range));

        itertools::chain(sticker_tri_ranges, internals_tri_ranges)
            .filter(|(piece, _sticker, _tri_range)| interactable_pieces.contains(*piece))
            .filter_map(|(piece, sticker, tri_range)| {
                self.triangle_hover(cursor_pos, piece, sticker, tri_range, vertex_3d_positions)
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    pub(crate) fn reset_camera(&mut self) {
        self.camera.rot = Motor::ident(self.puzzle().ndim());
    }

    pub(crate) fn do_sticker_click(&self, direction: Sign) {
        let mut state = self.sim.lock();
        let puzzle = state.puzzle_type();
        let ndim = puzzle.ndim();

        if let Some(hov) = &self.hover_state {
            if puzzle.ndim() == 3 {
                // Only do a move if we are hovering a sticker.
                if hov.sticker.is_none() {
                    return;
                }

                // Find the axis aligned with the normal vector of this
                // sticker.
                let [u, v] = [&hov.u_tangent, &hov.v_tangent];
                let target_vector = Vector::cross_product_3d(&u, &v);
                // TODO: this assumes that the axis vectors are normalized,
                //       which they are, but is that assumption documented or
                //       enforced anywhere? it feels a little sus.
                let Some(axis) = puzzle.axes.find(|_, axis_info| {
                    axis_info
                        .vector
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
                    false => Motor::from_normalized_vector_product(ndim, &u, &v),
                    true => Motor::from_normalized_vector_product(ndim, &v, &u),
                };
                let best_twist = candidates.min_by_key(|&twist| {
                    // `score` ranges from -1 to +1. If it's a positive number,
                    // then the twist goes in the desired direction; if it's
                    // negative, then it goes in the other direction. `score` is
                    // larger if the twist travels through a larger angle:
                    // - no rotation = 0
                    // - 180-degree rotation = ±1
                    let score = Motor::dot(&puzzle.twists[twist].transform, &target);
                    (Sign::from(score) * direction, FloatOrd(score.abs()))
                });
                if let Some(twist) = best_twist {
                    state.do_twist(twist, LayerMask(1));
                }
            }
        } else if puzzle.ndim() == 4 {
            // TODO: 4D click controls
        }
    }

    /// Returns the nearest triangle on a sticker that contain the screen-space
    /// point `p`.
    fn triangle_hover<'a>(
        &self,
        cursor_pos: cgmath::Point2<f32>,
        piece: Piece,
        sticker: Option<Sticker>,
        tri_range: &Range<u32>,
        vertex_3d_positions: &'a [cgmath::Vector4<f32>],
    ) -> Option<HoverState> {
        let puzzle_state = self.sim.lock();
        let piece_transform = &puzzle_state.piece_transforms()[piece];
        let mesh = &puzzle_state.puzzle_type().mesh;
        mesh.triangles[tri_range.start as usize..tri_range.end as usize]
            .iter()
            .filter_map(move |&vertex_ids| {
                let tri_verts @ [a, b, c] = vertex_ids.map(|i| vertex_3d_positions[i as usize]);
                let (barycentric_coords @ [qa, qb, qc], backface) =
                    triangle_hover_barycentric_coordinates(cursor_pos, tri_verts)?;

                let [pa, pb, pc] = vertex_ids.map(|i| mesh.vertex_position(i));
                let position =
                    piece_transform.transform_point(pa * qa as _ + pb * qb as _ + pc * qc as _);

                let [ua, ub, uc] = vertex_ids.map(|i| mesh.u_tangent(i as _));
                let [va, vb, vc] = vertex_ids.map(|i| mesh.v_tangent(i as _));
                let u_tangent =
                    piece_transform.transform_vector(ua * qa as _ + ub * qb as _ + uc * qc as _);
                let v_tangent =
                    piece_transform.transform_vector(va * qa as _ + vb * qb as _ + vc * qc as _);

                Some(HoverState {
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
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    fn parallel_drag_delta(&self) -> Option<Vector> {
        let initial_hover_state = self.hover_state()?;

        let screen_space_delta = self.cursor_pos? - initial_hover_state.cursor_pos;
        let delta_2d = screen_space_delta * crate::TWIST_DRAG_SPEED;
        // Get the 3D position where the drag started.
        let drag_start = initial_hover_state.position.clone();
        // Get the tangent vectors at that position.
        let [u, v] = [
            &initial_hover_state.u_tangent,
            &initial_hover_state.v_tangent,
        ];
        // Project the tangent vectors onto the screen.
        let u_2d = self.camera.project_vector(&drag_start, u)?;
        let v_2d = self.camera.project_vector(&drag_start, v)?;
        // Convert the drag delta into the basis formed using the projected
        // tangent vectors, then use that to reconstruct the 3D vector.
        let screen_to_uv = cgmath::Matrix2::from_cols(u_2d, v_2d).invert()?;
        let delta_uv = screen_to_uv * delta_2d;
        let delta_3d = u * delta_uv.x as _ + v * delta_uv.y as _;

        Some(match delta_3d.normalize() {
            Some(v) => v * delta_2d.magnitude() as _,
            None => vector![],
        })
    }
}

/// Returns the perspective-correct barycentric coordinates for the point `p` in
/// triangle `tri`, and a boolean indicating whether the triangle's backface is
/// visible (as opposed to its frontface). The Z coordinate is ignored; only X,
/// Y, and W are used.
///
/// Returns `None` if the point is not in the triangle.
///
/// This method uses the math described at
/// <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/visibility-problem-depth-buffer-depth-interpolation.html>
fn triangle_hover_barycentric_coordinates(
    p: cgmath::Point2<f32>,
    tri: [cgmath::Vector4<f32>; 3],
) -> Option<([f32; 3], bool)> {
    // If any vertex is culled, skip it.
    if tri.iter().any(|p| p.w == 0.0) {
        return None;
    }

    let mut tri_2d = tri.map(|p| cgmath::point2(p.x / p.w, p.y / p.w));

    // Ensure the triangle is counterclockwise.
    let mut total_area = triangle_area_2x(tri_2d);
    let rev = total_area < 0.0;
    if rev {
        tri_2d.reverse();
        total_area = -total_area;
    }

    // Compute the barycentric coordinates in screen space.
    let [a, b, c] = tri_2d;
    let recip_total_area = total_area.recip();
    let qa = triangle_area_2x([p, b, c]) * recip_total_area;
    let qb = triangle_area_2x([a, p, c]) * recip_total_area;
    let qc = triangle_area_2x([a, b, p]) * recip_total_area;
    // If the point is inside the triangle ...
    let [ra, rb, _rc] = (qa > 0.0 && qb > 0.0 && qc > 0.0).then(|| {
        let [a, b, c] = tri;
        // ... then compute the perspective-correct W value
        let w = qa * a.w + qb * b.w + qc * c.w;
        // ... and use that to compute perspective-correct barycentric
        //     coordinates.
        let mut out = [qa * w / a.w, qb * w / b.w, qc * w / c.w];
        if rev {
            out.reverse();
        }
        out
    })?;

    // Ensure that the barycentric coordinates add to *exactly* one.
    Some(([ra, rb, 1.0 - ra - rb], rev))
}

fn triangle_area_2x([a, b, c]: [cgmath::Point2<f32>; 3]) -> f32 {
    cgmath::Matrix2::from_cols(b - a, b - c).determinant()
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoverState {
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
impl HoverState {
    /// Returns the normal vector to the hovered surface, which is only valid in
    /// 3D.
    pub fn normal_3d(&self) -> Vector {
        self.u_tangent.cross_product_3d(&self.v_tangent)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DragState {
    /// Rotating the camera.
    ViewRot { z_axis: u8 },
    /// Clicked and dragged on a piece. Once the user has dragged enough to
    /// determine a direction, the drag state will change to
    /// [`DragState::Twist`].
    PreTwist,
    /// Dragging a piece to twist.
    Twist,
}

pub struct PuzzleViewInput<'a> {
    /// Position of the cursor on the puzzle view, in screen space.
    pub cursor_pos: Option<cgmath::Point2<f32>>,
    /// Size of the target to draw to.
    pub target_size: [u32; 2],
    /// 3D positions of vertices in the puzzle mesh.
    pub vertex_3d_positions: Option<Arc<Vec<cgmath::Vector4<f32>>>>,
    /// User preferences.
    pub prefs: &'a Preferences,
    /// Whether the cursor has been dragged enough to begin a drag twist, if
    /// that's the type of drag happening.
    pub exceeded_twist_drag_threshold: bool,
}

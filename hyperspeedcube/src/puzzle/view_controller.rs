use std::ops::Range;
use std::sync::Arc;

use bitvec::prelude::*;
use cgmath::SquareMatrix;
use float_ord::FloatOrd;
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperpuzzle::{LayerMask, PerPiece, Piece, Puzzle, Sticker};
use parking_lot::Mutex;

use super::controller::PuzzleController;
use super::styles::*;
use super::Camera;
use crate::preferences::{Preferences, ViewPreferences};

/// View into a puzzle simulation, which has its own piece filters.
#[derive(Debug)]
pub struct PuzzleViewController {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub state: Arc<Mutex<PuzzleController>>,

    pub camera: Camera,

    pub styles: PuzzleStyleStates,

    hover_state: Option<HoverState>,
}
impl PuzzleViewController {
    pub(crate) fn new(puzzle: &Arc<Mutex<PuzzleController>>) -> Self {
        let puzzle_controller = puzzle.lock();
        let puzzle_type = puzzle_controller.puzzle_type();
        Self {
            state: Arc::clone(puzzle),

            camera: Camera {
                prefs: ViewPreferences::default(),
                target_size: [1, 1],
                rot: Motor::ident(puzzle_type.ndim()),
                zoom: 0.5,
            },

            styles: PuzzleStyleStates::new(puzzle_type.pieces.len()),
            hover_state: None,
        }
    }

    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.state.lock().puzzle_type())
    }

    pub fn hover_state(&self) -> Option<HoverState> {
        self.hover_state.clone()
    }

    /// Sets the hover state.
    pub(crate) fn set_hover_state(&mut self, new_hover_state: Option<HoverState>) {
        if let Some(old_hover_state) = self.hover_state.take() {
            let mut piece_set: BitBox<u64> = bitbox![u64, Lsb0; 0; self.puzzle().pieces.len()];
            piece_set.set(old_hover_state.piece.0 as usize, true);
            self.styles
                .set_piece_states(&piece_set, |s| PieceStyleState {
                    hovered_piece: false,
                    ..s
                });
        }

        self.hover_state = new_hover_state;
        if let Some(new_hover_state) = &self.hover_state {
            let mut piece_set = bitbox![u64, Lsb0; 0; self.puzzle().pieces.len()];
            piece_set.set(new_hover_state.piece.0 as usize, true);
            self.styles
                .set_piece_states(&piece_set, |s| PieceStyleState {
                    hovered_piece: true,
                    ..s
                });
        }
    }

    /// Computes the new hover state.
    #[must_use]
    pub(crate) fn compute_hover_state(
        &self,
        screen_space_mouse_pos: cgmath::Point2<f32>,
        vertex_3d_positions: &[cgmath::Vector4<f32>],
        prefs: &Preferences,
    ) -> Option<HoverState> {
        let puzzle = self.puzzle();

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
            .filter(|(piece, _sticker, _tri_range)| interactable_pieces[piece.0 as usize])
            .filter_map(|(piece, sticker, tri_range)| {
                self.triangle_hover(
                    screen_space_mouse_pos,
                    piece,
                    sticker,
                    tri_range,
                    &vertex_3d_positions,
                )
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    /// Updates piece styles based on the puzzle controller state.
    pub(crate) fn update_styles(&mut self, prefs: &Preferences) {
        let state = self.state.lock();

        // TODO: maybe optimize this better?
        // TODO: make crate-wide function for f32<->u8 conversion
        let blocking_amount =
            (state.blocking_pieces().blocking_amount(&prefs.interaction) * 255.0) as u8;
        let mut pieces = bitbox![u64, Lsb0; 0; state.puzzle_type().pieces.len()];
        for piece in state.blocking_pieces().pieces() {
            pieces.set(piece.0 as usize, true);
        }
        self.styles.set_piece_states_with_opposite(
            &pieces,
            |style| PieceStyleState {
                blocking_amount,
                ..style
            },
            |style| PieceStyleState {
                blocking_amount: 0,
                ..style
            },
        );
    }

    pub(crate) fn reset_camera(&mut self) {
        self.camera.rot = Motor::ident(self.puzzle().ndim());
    }

    pub(crate) fn do_sticker_click(&self, direction: Sign) {
        let mut state = self.state.lock();
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
        p: cgmath::Point2<f32>,
        piece: Piece,
        sticker: Option<Sticker>,
        tri_range: &Range<u32>,
        vertex_3d_positions: &'a [cgmath::Vector4<f32>],
    ) -> Option<HoverState> {
        let puzzle_state = self.state.lock();
        let piece_transform = &puzzle_state.piece_transforms()[piece];
        let mesh = &puzzle_state.puzzle_type().mesh;
        mesh.triangles[tri_range.start as usize..tri_range.end as usize]
            .iter()
            .filter_map(move |&vertex_ids| {
                let tri_verts @ [a, b, c] = vertex_ids.map(|i| vertex_3d_positions[i as usize]);
                let (barycentric_coords @ [qa, qb, qc], backface) =
                    triangle_hover_barycentric_coordinates(p, tri_verts)?;

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
                    piece,
                    sticker,
                    z: qa * a.z + qb * b.z + qc * c.z,
                    backface,
                    vertex_ids,
                    barycentric_coords,

                    position,
                    u_tangent,
                    v_tangent,
                })
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
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
    pub piece: Piece,
    pub sticker: Option<Sticker>,
    /// Screen-space Z coordinate.
    z: f32,
    /// Whether the triangle is being hovered from behind.
    pub backface: bool,
    vertex_ids: [u32; 3],
    barycentric_coords: [f32; 3],

    /// Position of the cursor on the surface of the sticker, in puzzle space.
    pub position: Vector,
    /// First tangent vector of the surface on the sticker at the cursor, in
    /// puzzle space.
    pub u_tangent: Vector,
    /// Second tangent vector of the surface on the sticker at the cursor, in
    /// puzzle space.
    pub v_tangent: Vector,
}

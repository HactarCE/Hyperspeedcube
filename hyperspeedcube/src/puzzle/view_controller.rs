use std::ops::Range;
use std::sync::Arc;

use bitvec::prelude::*;
use cgmath::SquareMatrix;
use hypermath::pga::*;
use hypermath::prelude::*;
use hyperpuzzle::{LayerMask, PerPiece, Piece, Puzzle, Sticker};
use parking_lot::Mutex;

use super::controller::PuzzleController;
use super::styles::*;
use crate::preferences::Preferences;

#[derive(Debug)]
pub struct PuzzleViewController {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub state: Arc<Mutex<PuzzleController>>,

    pub rot: Motor,
    pub zoom: f32,

    pub styles: PuzzleStyleStates,
    hover_state: Option<HoverState>,
}
impl PuzzleViewController {
    pub(crate) fn new(puzzle: &Arc<Mutex<PuzzleController>>) -> Self {
        let puzzle_controller = puzzle.lock();
        let puzzle_type = puzzle_controller.puzzle_type();
        Self {
            state: Arc::clone(puzzle),

            rot: Motor::ident(puzzle_type.ndim()),
            zoom: 0.5,

            styles: PuzzleStyleStates::new(puzzle_type.pieces.len()),
            hover_state: None,
        }
    }

    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.state.lock().puzzle_type())
    }

    pub fn hover_state(&self) -> Option<&HoverState> {
        self.hover_state.as_ref()
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

        // On my machine, parallelizing this using rayon made it ~2x faster
        // (~900µs -> ~400µs) for 3^5, and a bit slower for smaller puzzles. It
        // may be worth parallelizing in the future if it turns out to be taking
        // a lot of time for large puzzles, especially on slow hardware.
        //
        // Another option is to compute it asynchronously while other
        // computations happen on another thread, although I'm not sure what
        // other expensive computations would need to happen every frame.

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
            .flat_map(|(piece, sticker, tri_range)| {
                triangle_hovers(
                    screen_space_mouse_pos,
                    &puzzle,
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
        self.rot = Motor::ident(self.puzzle().ndim());
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
                let u = Blade::from_vector(ndim, &hov.u_tangent);
                let v = Blade::from_vector(ndim, &hov.v_tangent);
                let Some(target_vector) =
                    Blade::cross_product_3d(&u, &v).and_then(|b| b.to_vector())
                else {
                    return;
                };
                // TODO: this assumes that the axis vectors are normalized,
                //       which they are, but is that assumption documented or
                //       enforced anywhere? it feels a little sus.
                let Some(axis) = puzzle
                    .axes
                    .find(|_, axis_info| approx_eq(&axis_info.vector, &target_vector))
                else {
                    return;
                };

                // Find the twist that turns the least in the correct direction.
                // TODO: search only twists on `axis`
                let candidates = puzzle
                    .twists
                    .iter_filter(|twist, twist_info| twist_info.axis == axis);

                // First, can we get the bivector to have the correct sign?
                // for twist in candidates {
                //     println!(
                //         "{} ... {}",
                //         puzzle.twists[twist].name,
                //         puzzle.twist_transform(twist).mv().dot(bivector.mv())
                //     );
                // }
                let best_twist = candidates.min_by_key(|&twist| {
                    // `score` ranges from -1 to +1. If it's a positive number,
                    // then the twist goes in the desired direction; if it's
                    // negative, then it goes in the other direction. `score` is
                    // larger if the twist travels through a larger angle:
                    // - no rotation = 0
                    // - 180-degree rotation = ±1

                    todo!("oops! didn't impl")
                    // let score = puzzle.twists[twist].transform.mv().dot(bivector.mv());
                    // (-Sign::from(score) * direction, FloatOrd(score.abs()))
                });
                if let Some(twist) = best_twist {
                    state.do_twist(twist, LayerMask(1));
                }
            }
        } else if puzzle.ndim() == 4 {
            // TODO: 4D click controls
        }
    }
}

/// Returns data about triangles that contain the screen-space point `p`.
fn triangle_hovers<'a>(
    p: cgmath::Point2<f32>,
    puzzle: &'a Puzzle,
    piece: Piece,
    sticker: Option<Sticker>,
    tri_range: &Range<u32>,
    vertex_3d_positions: &'a [cgmath::Vector4<f32>],
) -> impl 'a + Iterator<Item = HoverState> {
    puzzle.mesh.triangles[tri_range.start as usize..tri_range.end as usize]
        .iter()
        .filter_map(move |&vertex_ids| {
            let tri_verts @ [a, b, c] = vertex_ids.map(|i| vertex_3d_positions[i as usize]);
            let [qa, qb, qc] = triangle_hover_barycentric_coordinates(p, tri_verts)?;
            let [ua, ub, uc] = vertex_ids.map(|i| puzzle.mesh.u_tangent(i as _));
            let [va, vb, vc] = vertex_ids.map(|i| puzzle.mesh.v_tangent(i as _));
            Some(HoverState {
                piece,
                sticker,
                z: qa * a.z + qb * b.z + qc * c.z,
                u_tangent: ua * qa as _ + ub * qb as _ + uc * qc as _,
                v_tangent: va * qa as _ + vb * qb as _ + vc * qc as _,
            })
        })
}

/// Returns the perspective-correct barycentric coordinates for the point `p` in
/// triangle `tri`. The Z coordinate is ignored; only X, Y, and W are used.
///
/// Returns `None` if the point is not in the triangle.
///
/// This method uses the math described at
/// <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/visibility-problem-depth-buffer-depth-interpolation.html>
fn triangle_hover_barycentric_coordinates(
    p: cgmath::Point2<f32>,
    tri: [cgmath::Vector4<f32>; 3],
) -> Option<[f32; 3]> {
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
    (qa > 0.0 && qb > 0.0 && qc > 0.0).then(|| {
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
    })
}

fn triangle_area_2x([a, b, c]: [cgmath::Point2<f32>; 3]) -> f32 {
    cgmath::Matrix2::from_cols(b - a, b - c).determinant()
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoverState {
    piece: Piece,
    sticker: Option<Sticker>,
    z: f32,
    u_tangent: Vector,
    v_tangent: Vector,
}

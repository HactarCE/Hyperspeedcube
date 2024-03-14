use std::{ops::Range, sync::Arc};

use bitvec::prelude::*;
use cgmath::SquareMatrix;
use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, Piece, Puzzle, Sticker};
use parking_lot::Mutex;

use crate::preferences::Preferences;

use super::{controller::PuzzleController, styles::*};

#[derive(Debug)]
pub struct PuzzleViewController {
    /// Puzzle state. This is wrapped in an `Arc<Mutex<T>>` so that multiple
    /// puzzle views can access the same state.
    pub state: Arc<Mutex<PuzzleController>>,

    pub rot: Isometry,
    pub zoom: f32,

    pub styles: PuzzleStyleStates,
    hover_state: Option<HoverState>,
}
impl PuzzleViewController {
    pub(crate) fn new(puzzle: &Arc<Puzzle>) -> Self {
        Self::with_state(&Arc::new(Mutex::new(PuzzleController::new(puzzle))))
    }
    pub(crate) fn with_state(puzzle: &Arc<Mutex<PuzzleController>>) -> Self {
        Self {
            state: Arc::clone(puzzle),

            rot: Isometry::ident(),
            zoom: 0.5,

            styles: PuzzleStyleStates::new(puzzle.lock().puzzle.pieces.len()),
            hover_state: None,
        }
    }

    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.state.lock().puzzle)
    }

    pub fn hover_state(&self) -> Option<HoverState> {
        self.hover_state
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
            Some(HoverState {
                piece,
                sticker,
                z: qa * a.z + qb * b.z + qc * c.z,
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct HoverState {
    piece: Piece,
    sticker: Option<Sticker>,
    z: f32,
}

//! Puzzle geometry generation.

use cgmath::*;
use itertools::Itertools;

use super::PolygonVertex;
use crate::preferences::Preferences;
use crate::puzzle::*;
use ndpuzzle::util::IterCyclicPairsExt;

const OUTLINE_SCALE: f32 = 1.0 / 512.0;
const OUTLINE_WEDGE_VERTS_PER_RADIAN: f32 = 3.0;

pub(super) fn make_puzzle_mesh(
    puzzle: &mut PuzzleController,
    prefs: &Preferences,
    sticker_geometries: &[ProjectedStickerGeometry],
) -> (Vec<PolygonVertex>, Vec<u32>, Vec<[f32; 4]>) {
    // Triangulate polygons and combine the whole puzzle into one mesh.
    let mut verts = vec![];
    let mut indices = vec![];
    let mut polygon_colors = vec![];

    let facet_colors = &prefs.colors.facet_colors_list(puzzle.ty());

    for geom in sticker_geometries {
        let sticker_info = puzzle.ty().info(geom.sticker);

        let visual_state = puzzle.visual_piece_state(sticker_info.piece);

        // Determine sticker alpha.
        let alpha = visual_state.opacity(prefs);

        // Determine sticker fill color.
        let sticker_color = egui::Rgba::from(if prefs.colors.blindfold {
            prefs.colors.blind_sticker
        } else {
            facet_colors[puzzle.ty().info(geom.sticker).color.0 as usize]
        })
        .multiply(alpha);

        // Generate polygon vertices.
        for polygon in &*geom.front_polygons {
            let polygon_id = polygon_colors.len();
            let base = verts.len() as u32;
            polygon_colors.push([
                sticker_color.r() * polygon.illumination,
                sticker_color.g() * polygon.illumination,
                sticker_color.b() * polygon.illumination,
                sticker_color.a(),
            ]);
            verts.extend(polygon.verts.iter().map(|v| PolygonVertex {
                pos: [v.x, v.y, v.z],
                polygon_id: polygon_id as i32,
            }));
            let n = polygon.verts.len() as u32;
            indices.extend((2..n).flat_map(|i| [base, base + i - 1, base + i]));
        }
    }

    (verts, indices, polygon_colors)
}

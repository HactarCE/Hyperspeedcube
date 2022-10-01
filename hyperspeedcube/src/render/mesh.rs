//! Puzzle geometry generation.

use cgmath::*;
use itertools::Itertools;

use super::RgbaVertex;
use crate::preferences::Preferences;
use crate::puzzle::*;
use crate::util::IterCyclicPairsExt;

const OUTLINE_SCALE: f32 = 1.0 / 512.0;
const OUTLINE_WEDGE_VERTS_PER_RADIAN: f32 = 3.0;

pub(super) fn make_puzzle_mesh(
    puzzle: &mut PuzzleController,
    prefs: &Preferences,
    sticker_geometries: &[ProjectedStickerGeometry],
) -> (Vec<RgbaVertex>, Vec<u32>) {
    // Triangulate polygons and combine the whole puzzle into one mesh.
    let mut verts = vec![];
    let mut indices = vec![];

    // We already did depth sorting, so the GPU doesn't need to know the real
    // depth values. It just needs some value between 0 and 1 that increases
    // nearer to the camera. It's easy enough to start at 0.5 and do integer
    // incrementation for each sticker to get the next-largest `f32` value.
    let mut z = 0.5_f32;

    let face_colors = &prefs.colors.face_colors_list(puzzle.ty());

    for geom in sticker_geometries {
        let sticker_info = puzzle.ty().info(geom.sticker);

        let visual_state = puzzle.visual_piece_state(sticker_info.piece);

        // Determine sticker alpha.
        let alpha = visual_state.opacity(prefs);

        // Determine sticker fill color.
        let sticker_color = egui::Rgba::from(if prefs.colors.blindfold {
            prefs.colors.blind_face
        } else {
            face_colors[puzzle.ty().info(geom.sticker).color.0 as usize]
        })
        .multiply(alpha);

        // Determine outline appearance.
        let outline_color = visual_state
            .outline_color(prefs, puzzle.selection().contains(&geom.sticker))
            .multiply(alpha);
        let outline_size = visual_state.outline_size(prefs);

        // Generate outline vertices.
        if outline_size > 0.0 {
            let mut outlines = vec![];
            for polygon in &*geom.front_polygons {
                for (a, b) in polygon
                    .verts
                    .iter()
                    .map(|p| cgmath::point2(p.x, p.y))
                    .cyclic_pairs()
                {
                    // O(n) lookup using `.contains()` is fine because we'll
                    // never have more than 10 or so entries anyway.
                    if !outlines.contains(&[a, b]) && !outlines.contains(&[b, a]) {
                        outlines.push([a, b]);
                    }
                }
            }
            generate_outline_geometry(
                &mut verts,
                &mut indices,
                &outlines,
                outline_size,
                |Point2 { x, y }| RgbaVertex {
                    pos: [x, y, z],
                    color: outline_color.to_array(),
                },
            );
        }

        // Generate face vertices.
        for polygon in &*geom.front_polygons {
            let base = verts.len() as u32;
            verts.extend(polygon.verts.iter().map(|v| RgbaVertex {
                pos: [v.x, v.y, z],
                color: [
                    sticker_color.r() * polygon.illumination,
                    sticker_color.g() * polygon.illumination,
                    sticker_color.b() * polygon.illumination,
                    sticker_color.a(),
                ],
            }));
            let n = polygon.verts.len() as u32;
            indices.extend((2..n).flat_map(|i| [base, base + i - 1, base + i]));
        }

        // Increase the Z value very slightly. If this scares you, click this
        // link and try increasing the significand: https://float.exposed/0x3f000000
        z = f32::from_bits(z.to_bits() + 1);
    }

    (verts, indices)
}

fn generate_outline_geometry(
    verts_out: &mut Vec<RgbaVertex>,
    indices_out: &mut Vec<u32>,
    lines: &[[Point2<f32>; 2]],
    outline_size: f32,
    make_vert: impl Copy + Fn(Point2<f32>) -> RgbaVertex,
) {
    let outline_radius = outline_size * OUTLINE_SCALE;

    let mut unique_line_ends: Vec<Point2<f32>> = vec![];

    // Generate simple lines.
    for &[a, b] in lines {
        let base = verts_out.len() as u32;

        if !unique_line_ends.contains(&a) {
            unique_line_ends.push(a);
        }
        if !unique_line_ends.contains(&b) {
            unique_line_ends.push(b);
        }

        // Compute a vector parallel to the line.
        let parallel = b - a;
        // Rotate that 90 degrees counterclockwise to get the normal
        // vector of the line, and normalize it to the desired radius.
        let normal = cgmath::vec2(-parallel.y, parallel.x).normalize_to(outline_radius);
        verts_out.extend_from_slice(&[
            make_vert(a - normal),
            make_vert(a + normal),
            make_vert(b - normal),
            make_vert(b + normal),
        ]);
        indices_out.extend_from_slice(&[0, 1, 2, 3, 2, 1].map(|i| base + i));
    }

    // Generate line joins.
    for p in unique_line_ends {
        let max_angle_pair = {
            lines
                .iter()
                // For each edge where `p` is an endpoint, get the other
                // endpoint.
                .filter_map(|&[a, b]| match () {
                    _ if a == p => Some(b),
                    _ if b == p => Some(a),
                    _ => None,
                })
                // Get the angle of the edge incident to `p`.
                .map(|q| Rad::atan2(q.y - p.y, q.x - p.x))
                // Sort the angles counterclockwise.
                .sorted_by(|l, r| f32::total_cmp(&l.0, &r.0))
                // Compute the counterclockwise difference between each pair of adjacent angles.
                .cyclic_pairs()
                .map(|(a, b)| (a, (b - a).normalize()))
                // Find the pair of angles with the largest counterclockwise difference.
                .max_by(|(_, diff1), (_, diff2)| f32::total_cmp(&diff1.0, &diff2.0))
                // And it must be greater than 180 degrees.
                .filter(|&(_, diff)| diff > Rad::turn_div_2())
        };

        // If such a pair exists, then add a circular wedge to fill in the
        // gap. (Only one wedge will ever be needed for a given vertex.)
        if let Some((a, diff)) = max_angle_pair {
            let base = verts_out.len() as u32;
            verts_out.push(make_vert(p));

            let diff = diff - Rad::turn_div_2();
            let n = 2 + (diff.0 * OUTLINE_WEDGE_VERTS_PER_RADIAN).round() as usize;
            let rot = Matrix2::from_angle(diff / (n - 1) as f32);

            // Yes, `initial` is intentionally rotated an extra 90 degrees
            // counterclockwise because of the wedge shape we're trying to make.
            let initial = cgmath::vec2(-a.sin(), a.cos()) * outline_radius;

            verts_out.extend(
                std::iter::successors(Some(initial), |p| Some(rot * p))
                    .map(|offset| p + offset)
                    .map(make_vert)
                    .take(n),
            );
            indices_out.extend((1..n as u32).flat_map(|i| [base, base + i, base + i + 1]));
        }
    }
}

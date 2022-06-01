//! Puzzle geometry generation.

use cgmath::*;
use std::ops::{Add, Mul};

use super::*;
use crate::preferences::ViewPreferences;

const OUTLINE_SCALE: f32 = 1.0 / 512.0;
const OUTLINE_WEDGE_VERTS_PER_RADIAN: f32 = 3.0;

pub(super) fn generate_puzzle_geometry(app: &mut App) -> Vec<ProjectedStickerGeometry> {
    let prefs = &app.prefs;
    let puzzle = &app.puzzle;
    let view_prefs = &prefs.view[puzzle.ty()];

    let mut sticker_geometry_params = StickerGeometryParams::new(view_prefs);
    let light_params = LightParams::new(view_prefs);

    // Project stickers.
    let mut sticker_geometries: Vec<ProjectedStickerGeometry> = vec![];
    for piece in puzzle.displayed().pieces() {
        sticker_geometry_params.model_transform = puzzle.model_transform_for_piece(*piece);

        for sticker in piece.stickers() {
            let is_hovered = if prefs.interaction.highlight_piece_on_hover {
                // Is this piece hovered?
                app.hovered_sticker
                    .map(|s| s.piece() == *piece)
                    .unwrap_or(false)
            } else {
                // Is this sticker hovered?
                app.hovered_sticker.map(|s| s == sticker).unwrap_or(false)
            };

            // Determine opacity.
            let alpha = if is_hovered {
                prefs.colors.hovered_opacity
            } else {
                // Interpolate sticker alpha during a twist.
                let start_alpha = puzzle.sticker_alpha(sticker);
                let (end_alpha, t) = match puzzle.sticker_animation(sticker) {
                    Some((sticker, t)) => (puzzle.sticker_alpha(sticker), t),
                    None => (start_alpha, 0.0),
                };
                mix(start_alpha, end_alpha, t)
            };

            // Determine fill color.
            let fill_color = match prefs.colors.blindfold {
                false => match puzzle.displayed().get_sticker_color(sticker) {
                    Ok(face) => prefs.colors[face],
                    Err(_) => prefs.colors.blind_face, // fallback
                },
                true => prefs.colors.blind_face,
            };
            let mut fill_color = egui::Rgba::from(fill_color).to_array();
            fill_color[3] *= alpha;

            // Determine outline color and size.
            let (outline_color, outline_size) = if is_hovered {
                (prefs.outlines.hovered_color, prefs.outlines.hovered_size)
            } else if puzzle.selection().has_sticker(sticker) {
                (prefs.outlines.default_color, prefs.outlines.default_size)
            } else {
                (prefs.outlines.hidden_color, prefs.outlines.hidden_size)
            };
            let mut outline_color = egui::Rgba::from(outline_color).to_array();
            outline_color[3] *= alpha;

            // Compute geometry, including vertex positions before 3D
            // perspective projection.
            let sticker_geom = match sticker.geometry(sticker_geometry_params) {
                Some(s) => s,
                None => continue, // behind camera; skip this sticker
            };

            // Compute vertex positions after 3D perspective projection.
            let projected_verts = match sticker_geom
                .verts
                .iter()
                .map(|&v| sticker_geometry_params.project_3d(v))
                .collect::<Option<Vec<_>>>()
            {
                Some(s) => s,
                None => continue, // behind camera; skip this sticker
            };

            let mut projected_front_polygons = vec![];
            let mut projected_back_polygons = vec![];
            let mut outlines = vec![];

            for indices in &sticker_geom.polygon_indices {
                let projected_normal = polygon_normal_from_indices(&projected_verts, indices);
                if projected_normal.z > 0.0 {
                    // This polygon is front-facing.
                    let normal = polygon_normal_from_indices(&sticker_geom.verts, indices);
                    let color = light_params.compute_color(fill_color, normal);
                    projected_front_polygons.push(polygon_from_indices(
                        &projected_verts,
                        indices,
                        color,
                    ));

                    // Add outline edges.
                    if outline_size > 0.0 {
                        for (&a, &b) in indices.iter().cyclic_pairs() {
                            let edge = if a < b { [a, b] } else { [b, a] };
                            // O(n) lookup using `.contains()` is fine because we'll
                            // never have more than 10 or so entries anyway.
                            if !outlines.contains(&edge) {
                                outlines.push(edge);
                            }
                        }
                    }
                } else {
                    // This polygon is back-facing.
                    projected_back_polygons.push(polygon_from_indices(
                        &projected_verts,
                        indices,
                        fill_color,
                    ));
                }
            }

            let (min_bound, max_bound) = util::min_and_max_bound(&projected_verts);

            sticker_geometries.push(ProjectedStickerGeometry {
                sticker_id: sticker.id(),

                verts: projected_verts.into_boxed_slice(),
                min_bound,
                max_bound,

                front_polygons: projected_front_polygons.into_boxed_slice(),
                back_polygons: projected_back_polygons.into_boxed_slice(),

                outlines: outlines.into_boxed_slice(),
                outline_color,
                outline_size,
            });
        }
    }

    // Sort stickers by depth.
    sort::sort_by_depth(&mut sticker_geometries);

    sticker_geometries
}

pub(super) fn triangulate_puzzle_geometry(
    sticker_geometries: &[ProjectedStickerGeometry],
) -> (Vec<RgbaVertex>, Vec<u16>) {
    // Triangulate polygons and combine the whole puzzle into one mesh.
    let mut verts = vec![];
    let mut indices = vec![];
    // We already did depth sorting, so the GPU doesn't need to know the real
    // depth values. It just needs some value between 0 and 1 that increases
    // nearer to the camera. It's easy enough to start at 0.5 and do integer
    // incrementation for each sticker to get the next-largest `f32` value.
    let mut z = 0.5_f32;
    for sticker in sticker_geometries {
        // Generate outline vertices.
        if !sticker.outlines.is_empty() {
            generate_outline_geometry(&mut verts, &mut indices, sticker, |Point2 { x, y }| {
                RgbaVertex {
                    pos: [x, y, z],
                    color: sticker.outline_color,
                }
            });
        }

        // Generate face vertices.
        for polygon in &*sticker.front_polygons {
            let base = verts.len() as u16;
            verts.extend(polygon.verts.iter().map(|v| RgbaVertex {
                pos: [v.x, v.y, z],
                color: polygon.color,
            }));
            let n = polygon.verts.len() as u16;
            indices.extend((2..n).flat_map(|i| [base, base + i - 1, base + i]));
        }

        // Increase the Z value very slightly. If this scares you, click this
        // link and try increasing the significand: https://float.exposed/0x3f000000
        z = f32::from_bits(z.to_bits() + 1);
    }

    (verts, indices)
}

struct LightParams {
    light_vector: Vector3<f32>,
    directional_light_factor: f32,
    ambient_light_factor: f32,
}
impl LightParams {
    fn new(view_prefs: &ViewPreferences) -> Self {
        let light_vector = Matrix3::from_angle_y(Deg(view_prefs.light_yaw))
        * Matrix3::from_angle_x(Deg(-view_prefs.light_pitch)) // pitch>0 means light comes from above
        * Vector3::unit_z();
        let directional_light_factor = view_prefs.light_intensity;
        let ambient_light_factor = 1.0 - view_prefs.light_intensity; // TODO: make ambient light configurable
        Self {
            light_vector,
            directional_light_factor,
            ambient_light_factor,
        }
    }
    fn compute_color(&self, mut color: [f32; 4], normal: Vector3<f32>) -> [f32; 4] {
        let light_multiplier = (self.light_vector.dot(normal.normalize()) * 0.5 + 0.5)
            * self.directional_light_factor
            + self.ambient_light_factor;
        color[0] *= light_multiplier;
        color[1] *= light_multiplier;
        color[2] *= light_multiplier;
        color
    }
}

fn generate_outline_geometry(
    verts: &mut Vec<RgbaVertex>,
    indices: &mut Vec<u16>,
    projected_sticker: &ProjectedStickerGeometry,
    make_vert: impl Copy + Fn(Point2<f32>) -> RgbaVertex,
) {
    let outline_radius = projected_sticker.outline_size * OUTLINE_SCALE;

    // Generate simple lines.
    for &[i, j] in &*projected_sticker.outlines {
        let base = verts.len() as u16;

        let a = projected_sticker.verts[i as usize];
        let b = projected_sticker.verts[j as usize];
        let a = cgmath::point2(a.x, a.y);
        let b = cgmath::point2(b.x, b.y);
        // Compute a vector parallel to the line.
        let parallel = b - a;
        // Rotate that 90 degrees counterclockwise to get the normal
        // vector of the line, and normalize it to the desired radius.
        let normal = cgmath::vec2(-parallel.y, parallel.x).normalize_to(outline_radius);
        verts.extend_from_slice(&[
            make_vert(a - normal),
            make_vert(a + normal),
            make_vert(b - normal),
            make_vert(b + normal),
        ]);
        indices.extend_from_slice(&[base + 0, base + 1, base + 2, base + 3, base + 2, base + 1]);
    }

    // Generate line joins.
    for (i, p) in projected_sticker.verts.iter().enumerate() {
        let p = cgmath::point2(p.x, p.y);
        let max_angle_pair = {
            projected_sticker
                .outlines
                .iter()
                // For each edge, where `p` is an endpoint, get the other
                // endpoint.
                .filter_map(|&[a, b]| match () {
                    _ if a == i as u16 => Some(b),
                    _ if b == i as u16 => Some(a),
                    _ => None,
                })
                .map(|j| projected_sticker.verts[j as usize])
                // Get the angle of the edge incident to `p`.
                .map(|q| Rad::atan2(q.y - p.y, q.x - p.x))
                // Sort the angles counterclockwise.
                .sorted_by(|l, r| f32_total_cmp(&l.0, &r.0))
                // Compute the counterclockwise difference between each pair of adjacent angles.
                .cyclic_pairs()
                .map(|(a, b)| (a, (b - a).normalize()))
                // Find the pair of angles with the largest counterclockwise difference.
                .max_by(|(_, diff1), (_, diff2)| f32_total_cmp(&diff1.0, &diff2.0))
                // And it must be greater than 180 degrees.
                .filter(|&(_, diff)| diff > Rad::turn_div_2())
        };

        // If such a pair exists, then add a circular wedge to fill in the
        // gap. (Only one wedge will ever be needed for a given vertex.)
        if let Some((a, diff)) = max_angle_pair {
            let base = verts.len() as u16;
            verts.push(make_vert(p));

            let diff = diff - Rad::turn_div_2();
            let n = 2 + (diff.0 * OUTLINE_WEDGE_VERTS_PER_RADIAN).round() as usize;
            let rot = Matrix2::from_angle(diff / (n - 1) as f32);

            // Yes, `initial` is intentionally rotated an extra 90 degrees
            // counterclockwise because of the wedge shape we're trying to make.
            let initial = cgmath::vec2(-a.sin(), a.cos()) * outline_radius;

            verts.extend(
                std::iter::successors(Some(initial), |p| Some(rot * p))
                    .map(|offset| p + offset)
                    .map(make_vert)
                    .take(n),
            );
            indices.extend((1..n as u16).flat_map(|i| [base, base + i, base + i + 1]));
        }
    }
}

fn polygon_from_indices(verts: &[Point3<f32>], indices: &[u16], color: [f32; 4]) -> Polygon {
    let verts: SmallVec<_> = indices.iter().map(|&i| verts[i as usize]).collect();
    let normal = polygon_normal_from_indices(&verts, &[0, 1, 2]);
    let (min_bound, max_bound) = util::min_and_max_bound(&verts);

    Polygon {
        verts,
        min_bound,
        max_bound,
        normal,

        color,
    }
}

fn polygon_normal_from_indices(verts: &[Point3<f32>], indices: &[u16]) -> Vector3<f32> {
    let a = verts[indices[0] as usize];
    let b = verts[indices[1] as usize];
    let c = verts[indices[2] as usize];
    (c - a).cross(b - a)
}

fn mix<T>(a: T, b: T, t: f32) -> <T::Output as Add>::Output
where
    T: Mul<f32>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

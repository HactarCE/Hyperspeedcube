use cgmath::SquareMatrix;
use float_ord::FloatOrd;
use rand::seq::SliceRandom;
use rand::Rng;

pub const INVALID_STR: &str = "<invalid>";

/// Returns the perspective-correct barycentric coordinates for the point `p` in
/// triangle `tri`, and a boolean indicating whether the triangle's backface is
/// visible (as opposed to its frontface). The Z coordinate is ignored; only X,
/// Y, and W are used.
///
/// Returns `None` if the point is not in the triangle.
///
/// This function uses the math described at
/// <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/visibility-problem-depth-buffer-depth-interpolation.html>
pub fn triangle_hover_barycentric_coordinates(
    hover_pos: cgmath::Point2<f32>,
    tri: [cgmath::Vector4<f32>; 3],
) -> Option<([f32; 3], bool)> {
    // If any vertex is culled, skip the whole triangle.
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
    let h = hover_pos;
    let recip_total_area = total_area.recip();
    let qa = triangle_area_2x([h, b, c]) * recip_total_area;
    let qb = triangle_area_2x([a, h, c]) * recip_total_area;
    let qc = triangle_area_2x([a, b, h]) * recip_total_area;
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
/// Returns 2 times the area of a triangle.
pub fn triangle_area_2x([a, b, c]: [cgmath::Point2<f32>; 3]) -> f32 {
    cgmath::Matrix2::from_cols(b - a, b - c).determinant()
}

pub(crate) fn contrasting_text_color(background: egui::Color32) -> egui::Color32 {
    [egui::Color32::BLACK, egui::Color32::WHITE]
        .into_iter()
        .max_by_key(|&text_color| FloatOrd(egui_color_distance(text_color, background)))
        .unwrap_or_default()
}

/// Returns the perceptual distance between two colors using CIE2000.
pub(crate) fn egui_color_distance(a: egui::Color32, b: egui::Color32) -> f32 {
    empfindung::cie00::diff(egui_color32_to_lab(a), egui_color32_to_lab(b))
}
fn egui_color32_to_lab(color: egui::Color32) -> (f32, f32, f32) {
    let [r, g, b, _] = color.to_array();
    empfindung::ToLab::to_lab(&rgb_crate::RGB { r, g, b })
}

pub fn funny_autonames() -> impl Iterator<Item = String> {
    std::iter::from_fn(move || {
        Some(if rand::thread_rng().gen_bool(0.2) {
            format!("{} {}", gen_adjective(), gen_noun())
        } else {
            gen_noun()
        })
    })
}
fn gen_adjective() -> String {
    hyperpuzzle::util::titlecase(
        names::ADJECTIVES
            .choose(&mut rand::thread_rng())
            .unwrap_or(&"adjectivish"),
    )
}
fn gen_noun() -> String {
    hyperpuzzle::util::titlecase(
        names::NOUNS
            .choose(&mut rand::thread_rng())
            .unwrap_or(&"noun"),
    )
}

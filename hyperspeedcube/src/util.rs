use cgmath::SquareMatrix;
use float_ord::FloatOrd;
use hyperpuzzle::Rgb;
use rand::{seq::SliceRandom, Rng};

pub const INVALID_STR: &str = "<invalid>";

pub struct CyclicPairsIter<I: Iterator> {
    first: Option<I::Item>,
    prev: Option<I::Item>,
    rest: I,
}
impl<I> Iterator for CyclicPairsIter<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.rest.next() {
            Some(curr) => (self.prev.replace(curr.clone())?, curr),
            None => (self.prev.take()?, self.first.take()?),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.rest.size_hint();
        (lo.saturating_add(1), hi.and_then(|x| x.checked_add(1)))
    }
}

pub trait IterCyclicPairsExt: Iterator + Sized {
    fn cyclic_pairs(self) -> CyclicPairsIter<Self>;
}
impl<I> IterCyclicPairsExt for I
where
    I: Iterator,
    I::Item: Clone,
{
    fn cyclic_pairs(mut self) -> CyclicPairsIter<Self> {
        let first = self.next();
        let prev = first.clone();
        CyclicPairsIter {
            first,
            prev,
            rest: self,
        }
    }
}

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

/// Converts a [`hyperpuzzle::Rgb`] to an [`oklab::Oklab`].
pub(crate) fn rgb_to_oklab(color: Rgb) -> oklab::Oklab {
    let [r, g, b] = color.rgb;
    oklab::srgb_to_oklab(oklab::RGB { r, g, b })
}
/// Converts a [`hyperpuzzle::Rgb`] to an [`egui::Color32`].
pub(crate) fn rgb_to_egui_color32(color: Rgb) -> egui::Color32 {
    let [r, g, b] = color.rgb;
    egui::Color32::from_rgb(r, g, b)
}
/// Converts a [`hyperpuzzle::Rgb`] to an [`egui::Rgba`].
pub(crate) fn rgb_to_egui_rgba(color: Rgb) -> egui::Rgba {
    let [r, g, b] = color.rgb;
    egui::Rgba::from_srgba_unmultiplied(r, g, b, 255)
}
/// Converts an [`egui::Color32`] to a [`hyperpuzzle::Rgb`].
pub(crate) fn egui_color32_to_rgb(color: egui::Color32) -> Rgb {
    let [r, g, b, _] = color.to_array();
    Rgb { rgb: [r, g, b] }
}
/// Interpolates between two colors in linear color space.
pub(crate) fn lerp_colors(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let a = crate::util::rgb_to_egui_rgba(a);
    let b = crate::util::rgb_to_egui_rgba(b);
    let [r, g, b, _a] = hypermath::util::lerp(a, b, t).to_srgba_unmultiplied();
    Rgb { rgb: [r, g, b] }
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
/// Returns the perceptual distance between two colors using CIE2000.
pub(crate) fn color_distance(a: Rgb, b: Rgb) -> f32 {
    empfindung::cie00::diff(rgb_to_lab(a), rgb_to_lab(b))
}
fn egui_color32_to_lab(color: egui::Color32) -> (f32, f32, f32) {
    let rgb = [color.r(), color.g(), color.b()];
    rgb_to_lab(Rgb { rgb })
}
fn rgb_to_lab(rgb: Rgb) -> (f32, f32, f32) {
    let [r, g, b] = rgb.rgb;
    empfindung::ToLab::to_lab(&rgb_crate::RGB { r, g, b })
}

pub(crate) fn mix_colors(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let a = rgb_to_egui_rgba(a);
    let b = rgb_to_egui_rgba(b);
    let [r, g, b, _a] = (a * (1.0 - t) + b * t).to_srgba_unmultiplied();
    Rgb { rgb: [r, g, b] }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclic_pairs_iter() {
        assert_eq!(
            [1, 2, 3, 4].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 4), (4, 1)],
        );
        assert_eq!(
            [1, 2, 3].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 1)],
        );
        assert_eq!(
            [1, 2].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 1)],
        );
        assert_eq!(
            [1].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 1)],
        );
    }

    // TODO: test reordering
}

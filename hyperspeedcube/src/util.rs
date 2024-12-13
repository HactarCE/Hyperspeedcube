use cgmath::SquareMatrix;
use float_ord::FloatOrd;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::Rng;

pub const INVALID_STR: &str = "<invalid>";

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
    let rgba = color.to_array();
    empfindung::ToLab::to_lab(&lab::Lab::from_rgba(&rgba))
}

pub fn funny_autonames() -> impl Iterator<Item = String> {
    std::iter::from_fn(move || {
        Some(if rand::rng().random_bool(0.2) {
            format!("{} {}", gen_adjective(), gen_noun())
        } else {
            gen_noun()
        })
    })
}
fn gen_adjective() -> String {
    hyperpuzzle::util::titlecase(
        names::ADJECTIVES
            .choose(&mut rand::rng())
            .unwrap_or(&"adjectivish"),
    )
}
fn gen_noun() -> String {
    hyperpuzzle::util::titlecase(names::NOUNS.choose(&mut rand::rng()).unwrap_or(&"noun"))
}

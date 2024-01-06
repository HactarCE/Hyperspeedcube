#[macro_use]
mod combo_boxes;
// mod keybinds;
mod layer_mask;
pub mod prefs;
mod presets;
mod puzzle_list;
mod reorder;
mod reset;
mod yaml_editor;

pub use combo_boxes::*;
// pub use keybinds::*;
pub use layer_mask::*;
pub use prefs::PrefsUi;
pub use presets::*;
pub use puzzle_list::*;
pub use reorder::*;
pub use reset::*;
pub use yaml_editor::*;

pub const BIG_ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 22.0);
pub const SMALL_ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(20.0, 18.0);

pub fn big_icon_button(ui: &mut egui::Ui, text: &str, hover_text: &str) -> egui::Response {
    let r = ui.add_sized(BIG_ICON_BUTTON_SIZE, egui::Button::new(text));
    if hover_text.is_empty() {
        r
    } else {
        r.on_hover_text(hover_text)
    }
}

pub fn small_icon_button(ui: &mut egui::Ui, text: &str, hover_text: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
        let r = ui.add_sized(SMALL_ICON_BUTTON_SIZE, egui::Button::new(text));
        if hover_text.is_empty() {
            r
        } else {
            r.on_hover_text(hover_text)
        }
    })
    .inner
}

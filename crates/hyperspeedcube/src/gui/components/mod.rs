#[macro_use]
mod combo_boxes;
mod ariadne;
mod color_widgets;
mod filter_checkbox;
mod hint;
// mod keybinds;
mod layer_mask;
pub mod prefs;
mod presets;
mod reset;
mod tag_menu;
mod text_edit_popup;
mod yaml_editor;

pub use ariadne::*;
pub use color_widgets::*;
pub use combo_boxes::*;
pub use filter_checkbox::*;
pub use hint::*;
// pub use keybinds::*;
pub use layer_mask::*;
pub use prefs::PrefsUi;
pub use presets::*;
pub use reset::*;
pub use tag_menu::*;
pub use text_edit_popup::*;
pub use yaml_editor::*;

use crate::L;

pub const BIG_ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 22.0);
pub const SMALL_ICON_BUTTON_SIZE: egui::Vec2 = egui::vec2(18.0, 18.0);

fn error_label(ui: &mut egui::Ui, text: impl Into<egui::RichText>) -> egui::Response {
    ui.colored_label(ui.visuals().error_fg_color, text)
}

/// Copies text to the clipboard if `text_to_copy` is `Some` and displays a
/// tooltip under `r` if text has been copied to the clipboard since the last
/// time the mouse moved away from the widget. Returns whether the tooltip was
/// shown.
pub fn copy_on_click(ui: &mut egui::Ui, r: &egui::Response, text_to_copy: Option<String>) -> bool {
    let has_been_copied = crate::gui::util::EguiTempFlag::new(ui);
    if let Some(text) = text_to_copy {
        ui.ctx().copy_text(text);
        has_been_copied.set();
    }
    if has_been_copied.get() {
        if r.hovered() || r.has_focus() {
            r.show_tooltip_text(L.statuses.copied);
            return true;
        } else {
            has_been_copied.reset(); // Hide the tooltip when the mouse leaves
        }
    }
    false
}

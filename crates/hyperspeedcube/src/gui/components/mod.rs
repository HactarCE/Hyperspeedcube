#[macro_use]
mod combo_boxes;
mod ariadne;
mod color_widgets;
mod filter_checkbox;
mod hint;
mod leaderboards;
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
pub use leaderboards::show_leaderboards_ui;
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

pub struct IconButton<'a> {
    icon: egui::Image<'a>,
    icon_size: f32,
    min_button_size: f32,
    selected: Option<bool>,
    transparent: bool,
}

impl<'a> IconButton<'a> {
    fn new(icon: egui::Image<'a>, icon_size: f32, min_button_size: f32) -> Self {
        Self {
            icon,
            icon_size,
            min_button_size,
            selected: None,
            transparent: false,
        }
    }

    pub fn very_small(icon: egui::Image<'a>) -> Self {
        Self::new(icon, 12.0, 0.0)
    }

    pub fn small(icon: egui::Image<'a>) -> Self {
        Self::new(icon, 12.0, 18.0)
    }

    pub fn big(icon: egui::Image<'a>) -> Self {
        Self::new(icon, 16.0, 22.0)
    }

    pub fn selectable(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn transparent(mut self) -> Self {
        self.transparent = true;
        self
    }
}

impl egui::Widget for IconButton<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.scope(|ui| {
            let spacing = ui.spacing_mut();
            spacing.button_padding.x = spacing.button_padding.y;
            spacing.interact_size = egui::vec2(0.0, 0.0);

            let atoms = self
                .icon
                .fit_to_exact_size(egui::Vec2::splat(self.icon_size));

            let mut button = match self.selected {
                Some(selected) => egui::Button::selectable(selected, atoms),
                None => egui::Button::new(atoms),
            }
            .min_size(egui::Vec2::splat(self.min_button_size));

            if self.transparent {
                button = button.fill(egui::Color32::TRANSPARENT);
            }

            ui.add(button)
        })
        .inner
    }
}

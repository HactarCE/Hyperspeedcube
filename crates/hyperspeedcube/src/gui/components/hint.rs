use egui::NumExt;

use crate::gui::util::MDI_MEDIUM_BUTTON_SIZE;

pub struct HelpHoverWidget;
impl HelpHoverWidget {
    pub fn show_in_top_right(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        let button_rect =
            egui::Align2::RIGHT_TOP.align_size_within_rect(MDI_MEDIUM_BUTTON_SIZE, ui.max_rect());
        let r = ui.place(button_rect, make_widget(ui));
        show_tooltip_if_hovered(&r, markdown);

        ui.set_max_width(
            ui.max_rect().width() - MDI_MEDIUM_BUTTON_SIZE.x - ui.spacing().item_spacing.x,
        );

        r
    }
    pub fn show_right_aligned(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        ui.add_space(MDI_MEDIUM_BUTTON_SIZE.x + ui.spacing().item_spacing.x); // Ensure there's enough space
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Self::show(ui, markdown)
        })
        .inner
    }
    pub fn show(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        let r = ui.add(make_widget(ui));
        show_tooltip_if_hovered(&r, markdown);
        r
    }
}

fn make_widget<'a>(ui: &egui::Ui) -> egui::Button<'a> {
    egui::Button::new(mdi!(ui.visuals().text_color(), HELP))
        .corner_radius(8)
        .min_size(MDI_MEDIUM_BUTTON_SIZE)
        .fill(egui::Color32::TRANSPARENT)
}

fn show_tooltip_if_hovered(r: &egui::Response, markdown: &str) {
    if r.hovered() || r.has_focus() {
        egui::Popup::from_response(r)
            .gap(8.0) // prevent flashing
            .kind(egui::PopupKind::Tooltip)
            .show(|ui| {
                let w =
                    super::super::ext::HELP_TOOLTIP_WIDTH.at_most(ui.ctx().content_rect().width());
                ui.set_width(w);
                crate::gui::markdown::md(ui, markdown);
            });
    }
}

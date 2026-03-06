use crate::gui::util::{MDI_MEDIUM, MDI_MEDIUM_BUTTON_SIZE, MDI_SMALL_BUTTON_SIZE};

pub struct HelpHoverWidget;
impl HelpHoverWidget {
    pub fn show_right_aligned(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        ui.add_space(MDI_SMALL_BUTTON_SIZE.x + ui.spacing().item_spacing.x); // Ensure there's enough space
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Self::show(ui, markdown)
        })
        .inner
    }
    pub fn show(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        let r = ui.add(
            egui::Button::new(mdi!(ui.visuals().text_color(), HELP))
                .corner_radius(8)
                .min_size(MDI_SMALL_BUTTON_SIZE)
                .fill(egui::Color32::TRANSPARENT),
        );

        if r.hovered() || r.has_focus() {
            egui::Popup::from_response(&r)
                .gap(8.0) // prevent flashing
                .kind(egui::PopupKind::Tooltip)
                .show(|ui| {
                    // TODO: refactor this constant
                    ui.set_width(super::super::ext::EXPLANATION_TOOLTIP_WIDTH * 2.0);
                    crate::gui::markdown::md(ui, markdown);
                });
        }

        r
    }
}

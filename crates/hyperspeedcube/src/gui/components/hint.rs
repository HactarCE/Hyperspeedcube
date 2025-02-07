pub struct HelpHoverWidget;
impl HelpHoverWidget {
    pub fn show_right_aligned(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        ui.add_space(24.0); // Ensure there's enough space
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Self::show(ui, markdown)
        })
        .inner
    }
    pub fn show(ui: &mut egui::Ui, markdown: &str) -> egui::Response {
        let r = ui.add(
            egui::Button::new("?")
                .small()
                .corner_radius(16)
                .min_size(egui::vec2(24.0, 16.0))
                .fill(egui::Color32::TRANSPARENT),
        );

        if r.hovered() || r.has_focus() {
            r.show_tooltip_ui(|ui| {
                // TODO: refactor this constant
                ui.set_width(super::super::ext::EXPLANATION_TOOLTIP_WIDTH * 2.0);
                crate::gui::markdown::md(ui, markdown);
            });
        }

        r
    }
}

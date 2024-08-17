pub struct HelpHoverWidget;
impl HelpHoverWidget {
    pub fn show_right_aligned(
        ui: &mut egui::Ui,
        markdown: &str,
    ) -> egui::InnerResponse<Option<egui::Response>> {
        ui.add_space(24.0); // Ensure there's enough space
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Self::show(ui, markdown)
        })
        .inner
    }
    pub fn show(ui: &mut egui::Ui, markdown: &str) -> egui::InnerResponse<Option<egui::Response>> {
        let r = ui.add(
            egui::Button::new("?")
                .small()
                .rounding(16.0)
                .min_size(egui::vec2(24.0, 16.0))
                .fill(egui::Color32::TRANSPARENT),
        );

        let inner = if r.hovered() || r.has_focus() {
            // Show the tooltip immediately
            egui::show_tooltip_for(ui.ctx(), unique_id!(), &r.rect, |ui| {
                // TODO: refactor this constant
                ui.set_width(super::super::ext::EXPLANATION_TOOLTIP_WIDTH * 2.0);
                crate::gui::markdown::md(ui, markdown)
            })
        } else {
            None
        };

        egui::InnerResponse::new(inner, r)
    }
}

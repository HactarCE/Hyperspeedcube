pub struct HelpHoverWidget;
impl HelpHoverWidget {
    pub fn show_right_aligned<R>(
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            Self::show(ui, add_contents)
        })
        .inner
    }
    pub fn show<R>(
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let r = ui.add(
            egui::Button::new("?")
                .small()
                .rounding(16.0)
                .min_size(egui::vec2(24.0, 16.0))
                .fill(egui::Color32::TRANSPARENT),
        );

        let inner = if r.hovered() || r.has_focus() {
            egui::containers::show_tooltip_for(ui.ctx(), unique_id!(), &r.rect, |ui| {
                // TODO: refactor this constant
                ui.set_width(super::super::ext::EXPLANATION_TOOLTIP_WIDTH * 2.0);
                add_contents(ui)
            })
        } else {
            None
        };

        egui::InnerResponse::new(inner, r)
    }
}

use egui::NumExt;

use super::markdown::md;

pub const EXPLANATION_TOOLTIP_WIDTH: f32 = 200.0;

pub trait ResponseExt {
    fn on_hover_explanation(
        self,
        strong_text: impl AsRef<str>,
        detailed_message: impl AsRef<str>,
    ) -> Self;
}
impl ResponseExt for egui::Response {
    fn on_hover_explanation(
        self,
        strong_text: impl AsRef<str>,
        detailed_message: impl AsRef<str>,
    ) -> Self {
        self.on_hover_ui(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(EXPLANATION_TOOLTIP_WIDTH, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    if !strong_text.as_ref().is_empty() {
                        ui.strong(strong_text.as_ref());
                    }
                    if !detailed_message.as_ref().is_empty() {
                        md(ui, detailed_message);
                    }
                },
            );
        })
    }
}

pub trait ComboBoxExt {
    /// Workaround for egui being *not fabulous* at sizing combo boxes.
    fn width_to_fit(
        self,
        ui: &egui::Ui,
        options: impl IntoIterator<Item = impl Into<egui::WidgetText>>,
    ) -> Self;
}
impl ComboBoxExt for egui::ComboBox {
    fn width_to_fit(
        self,
        ui: &egui::Ui,
        options: impl IntoIterator<Item = impl Into<egui::WidgetText>>,
    ) -> Self {
        let spacing = ui.spacing();

        let text_width = options
            .into_iter()
            .map(|option| {
                option
                    .into()
                    .into_galley(ui, Some(false), f32::INFINITY, egui::TextStyle::Button)
                    .size()
                    .x
            })
            .reduce(f32::max)
            .unwrap_or(0.0);

        let mut desired_width = text_width
            + spacing.item_spacing.x
            + f32::max(
                spacing.icon_width,
                spacing.window_margin.left
                    + spacing.scroll.bar_width
                    + spacing.window_margin.right
                    + 1.0, // not sure why, but text wraps without the +1.0
            );

        if ui.layout().horizontal_justify() {
            desired_width = desired_width.at_least(ui.available_width() - spacing.item_spacing.x);
        }

        desired_width = desired_width.at_least(spacing.interact_size.x - spacing.item_spacing.x);

        self.width(desired_width)
    }
}

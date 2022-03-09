use std::hash::Hash;
use strum::IntoEnumIterator;

const EXPLANATION_TOOLTIP_WIDTH: f32 = 200.0;

pub(super) struct BasicComboBox<'a, T> {
    combo_box: egui::ComboBox,
    options: Vec<T>,
    selected: &'a mut T,
}
impl<'a, T: IntoEnumIterator + ToString> BasicComboBox<'a, T> {
    pub(super) fn new_enum(id_source: impl Hash, selected: &'a mut T) -> Self {
        Self {
            combo_box: egui::ComboBox::from_id_source(id_source),
            options: T::iter().collect(),
            selected,
        }
    }
    pub(super) fn new_enum_with_label(
        id_source: impl Hash,
        label: impl Into<egui::WidgetText>,
        selected: &'a mut T,
    ) -> Self {
        Self {
            combo_box: egui::ComboBox::new(id_source, label),
            options: T::iter().collect(),
            selected,
        }
    }
}
impl<T: ToString + Eq> egui::Widget for BasicComboBox<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut response = self
            .combo_box
            .selected_text(self.selected.to_string())
            .show_ui(ui, |ui| {
                for option in self.options {
                    let is_selected = option == *self.selected;
                    if ui
                        .selectable_label(is_selected, option.to_string())
                        .clicked()
                    {
                        *self.selected = option;
                        changed = true;
                    }
                }
            })
            .response;

        if changed {
            response.mark_changed();
        }

        response
    }
}

pub(super) trait ResponseExt {
    fn on_hover_explanation(self, strong_text: &str, detailed_message: &str) -> Self;
}
impl ResponseExt for egui::Response {
    fn on_hover_explanation(self, strong_text: &str, detailed_message: &str) -> Self {
        self.on_hover_ui(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(EXPLANATION_TOOLTIP_WIDTH, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if !strong_text.is_empty() {
                        ui.label(egui::RichText::new(strong_text).strong());
                    }
                    if !detailed_message.is_empty() {
                        ui.label(detailed_message);
                    }
                },
            );
        })
    }
}

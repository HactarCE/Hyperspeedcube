use egui::NumExt;
use itertools::Itertools;
use std::hash::Hash;
use strum::IntoEnumIterator;

const EXPLANATION_TOOLTIP_WIDTH: f32 = 200.0;

pub(super) struct BasicComboBox<'a, T> {
    combo_box: egui::ComboBox,
    selected: &'a mut T,
    options: Vec<T>,
}
impl<'a, T: IntoEnumIterator> BasicComboBox<'a, T> {
    pub(super) fn new_enum(id_source: impl Hash, selected: &'a mut T) -> Self {
        Self::new(id_source, selected, T::iter().collect_vec())
    }
    pub(super) fn new_enum_with_label(
        id_source: impl Hash,
        label: impl Into<egui::WidgetText>,
        selected: &'a mut T,
    ) -> Self {
        Self {
            combo_box: egui::ComboBox::new(id_source, label),
            selected,
            options: T::iter().collect(),
        }
    }
}
impl<'a, T> BasicComboBox<'a, T> {
    pub(super) fn new(
        id_source: impl Hash,
        selected: &'a mut T,
        options: impl Into<Vec<T>>,
    ) -> Self {
        Self {
            combo_box: egui::ComboBox::from_id_source(id_source),
            options: options.into(),
            selected,
        }
    }
}
impl<T: ToString + Eq> egui::Widget for BasicComboBox<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut r = self
            .combo_box
            .selected_text(self.selected.to_string())
            .width_to_fit(ui, self.options.iter().map(|s| s.to_string()).collect())
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
            });

        if changed {
            r.response.mark_changed();
        }
        r.response
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

pub(super) trait ComboBoxExt {
    /// Workaround for egui being *not fabulous* at sizing combo boxes.
    fn width_to_fit(self, ui: &egui::Ui, options: Vec<String>) -> Self;
}
impl ComboBoxExt for egui::ComboBox {
    fn width_to_fit(self, ui: &egui::Ui, options: Vec<String>) -> Self {
        let spacing = ui.spacing();

        let text_width = options
            .into_iter()
            .map(|option| {
                egui::WidgetText::from(option)
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
                    + spacing.scroll_bar_width
                    + spacing.window_margin.right
                    + 1.0, // not sure why, but text wraps without the +1.0
            );

        if ui.layout().horizontal_justify() {
            desired_width = desired_width.at_least(ui.available_width() - spacing.item_spacing.x);
        }

        self.width(desired_width)
    }
}

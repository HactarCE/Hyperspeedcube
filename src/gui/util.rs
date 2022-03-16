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
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    if !strong_text.is_empty() {
                        ui.strong(strong_text);
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

        desired_width = desired_width.at_least(spacing.interact_size.x - spacing.item_spacing.x);

        self.width(desired_width)
    }
}

pub(super) fn make_degrees_drag_value(value: &mut f32) -> egui::DragValue {
    egui::DragValue::new(value).suffix("°").fixed_decimals(0)
}
pub(super) fn make_percent_drag_value(value: &mut f32) -> egui::DragValue {
    egui::DragValue::from_get_set(|new_value| {
        if let Some(x) = new_value {
            *value = x as f32 / 100.0;
        }
        *value as f64 * 100.0
    })
    .suffix("%")
    .fixed_decimals(0)
    .clamp_range(0.0..=100.0_f32)
    .speed(0.5)
}

#[must_use]
pub(super) struct WidgetWithReset<'a, V, W: 'a + egui::Widget, F: FnOnce(&'a mut V) -> W> {
    pub(super) label: &'a str,
    pub(super) value: &'a mut V,
    pub(super) reset_value: V,
    pub(super) reset_value_str: String,
    pub(super) make_widget: F,
}
impl<'a, V, W, F> egui::Widget for WidgetWithReset<'a, V, W, F>
where
    V: PartialEq,
    W: 'a + egui::Widget,
    F: FnOnce(&'a mut V) -> W,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        with_reset_button(
            ui,
            self.value,
            self.reset_value,
            &self.reset_value_str,
            |ui, value| {
                let widget_resp =
                    ui.add_sized(ui.spacing().interact_size, (self.make_widget)(value));
                let mut label_resp = ui.label(self.label);

                // Return the label response so that the caller can add hover
                // text to the label if they want.
                if widget_resp.changed() {
                    label_resp.mark_changed();
                }
                label_resp
            },
        )
    }
}

#[must_use]
pub(super) struct CheckboxWithReset<'a> {
    pub(super) label: &'a str,
    pub(super) value: &'a mut bool,
    pub(super) reset_value: bool,
}
impl egui::Widget for CheckboxWithReset<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        with_reset_button(ui, self.value, self.reset_value, "", |ui, value| {
            ui.checkbox(value, self.label)
        })
    }
}

pub(super) fn with_reset_button<'a, T: PartialEq>(
    ui: &mut egui::Ui,
    value: &'a mut T,
    reset_value: T,
    reset_value_str: &str,
    widget: impl FnOnce(&mut egui::Ui, &'a mut T) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        let reset_resp = reset_button(ui, value, reset_value, reset_value_str);
        let mut r = widget(ui, value);
        if reset_resp.clicked() {
            r.mark_changed();
        }
        r
    })
    .inner
}

pub(super) fn reset_button<T: PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    reset_value: T,
    reset_value_str: &str,
) -> egui::Response {
    let hover_text = match reset_value_str {
        "" => "Reset".to_owned(),
        s => format!("Reset to {}", s),
    };
    let r = ui
        .add_enabled(*value != reset_value, egui::Button::new("⟲"))
        .on_hover_text(&hover_text);
    if r.clicked() {
        *value = reset_value;
    }
    r
}

use std::borrow::Cow;

use crate::L;

#[must_use]
pub struct WidgetWithReset<'a, V, W: 'a + egui::Widget, F: FnOnce(&'a mut V) -> W> {
    pub label: egui::WidgetText,
    pub value: &'a mut V,
    pub reset_value: Option<V>,
    pub reset_value_str: Option<Cow<'a, str>>,
    pub make_widget: F,
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
            self.reset_value_str.as_deref(),
            |ui, value| {
                let widget_resp =
                    ui.add_sized(ui.spacing().interact_size, (self.make_widget)(value));
                if !self.label.is_empty() {
                    let mut label_resp = ui.label(self.label);

                    // Return the label response so that the caller can add hover
                    // text to the label if they want.
                    if widget_resp.changed() {
                        label_resp.mark_changed();
                    }
                    label_resp
                } else {
                    widget_resp
                }
            },
        )
    }
}

pub fn with_reset_button<'a, T: PartialEq>(
    ui: &mut egui::Ui,
    value: &'a mut T,
    reset_value: Option<T>,
    reset_value_str: Option<&str>,
    widget: impl FnOnce(&mut egui::Ui, &'a mut T) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        let reset_resp = reset_value.map(|v| reset_button(ui, value, v, reset_value_str));

        let mut r = widget(ui, value);

        if let Some(reset_resp) = reset_resp
            && reset_resp.clicked()
        {
            r.mark_changed();
        }
        r
    })
    .inner
}

pub fn reset_button<T: PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    reset_value: T,
    reset_value_str: Option<&str>,
) -> egui::Response {
    let mut r = ui.add_enabled(
        *value != reset_value,
        egui::Button::new("⟲").min_size(egui::vec2(20.0, 20.0)), // TODO: extract into constant
    );
    let hover_text: Cow<'_, str> = match reset_value_str {
        None => L.reset.into(),
        Some(s) => L.reset_to_value.with(s).into(),
    };
    r = r.on_hover_text(&*hover_text);
    if r.clicked() {
        *value = reset_value;
    }
    r
}

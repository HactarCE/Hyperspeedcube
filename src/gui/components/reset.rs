#[must_use]
pub struct WidgetWithReset<'a, V, W: 'a + egui::Widget, F: FnOnce(&'a mut V) -> W> {
    pub label: &'a str,
    pub value: &'a mut V,
    pub reset_value: V,
    pub reset_value_str: String,
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

pub fn with_reset_button<'a, T: PartialEq>(
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

pub fn reset_button<T: PartialEq>(
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
        .on_hover_text(hover_text);
    if r.clicked() {
        *value = reset_value;
    }
    r
}

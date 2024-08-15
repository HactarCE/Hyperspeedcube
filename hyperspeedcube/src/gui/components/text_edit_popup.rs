use crate::gui::util::EguiTempValue;

use super::BIG_ICON_BUTTON_SIZE;

/// Function that returns `Ok` if the button should be enabled or `Err` if it
/// should not be. The contained value is the hover text.
pub type TextEditValidator<'a> = &'a dyn Fn(&str) -> Result<Option<String>, Option<String>>;

#[derive(Debug, Default, Clone)]
pub enum TextEditPopupResponse {
    Confirm(String),
    Delete,
    #[default]
    Cancel,
}

/// Popup with a single-line text edit widget as well as several other optional
/// widgets: a label, a confirm button, and a delete button.
pub struct TextEditPopup<'v> {
    ctx: egui::Context,
    new_name: EguiTempValue<String>,
    is_first_frame: bool,

    area: egui::Area,

    label: Option<String>,

    text_edit_align: Option<egui::Align>,
    text_edit_trim: bool,
    text_edit_monospace: bool,
    text_edit_width: Option<f32>,
    text_edit_hint_text: Option<String>,

    auto_confirm: bool,
    validate_confirm: Option<TextEditValidator<'v>>,
    validate_delete: Option<TextEditValidator<'v>>,
}
impl<'v> TextEditPopup<'v> {
    pub fn new(ui: &mut egui::Ui) -> Self {
        let ctx = ui.ctx().clone();
        let new_name = EguiTempValue::new(ui);
        let area = egui::Area::new(new_name.id.with("area"))
            .order(egui::Order::Middle)
            .constrain_to(ui.ctx().available_rect());

        Self {
            ctx,
            new_name,
            is_first_frame: false,

            area,

            label: None,

            text_edit_align: None,
            text_edit_trim: true, // enable by default
            text_edit_monospace: false,
            text_edit_width: None,
            text_edit_hint_text: None,

            auto_confirm: false,
            validate_confirm: None,
            validate_delete: None,
        }
    }

    /// Executes a function if the popup is open. This is useful to avoid
    /// unnecessary computation.
    pub fn if_open<R>(self, f: impl FnOnce(Self) -> Option<R>) -> Option<R> {
        if self.is_open() {
            f(self)
        } else {
            None
        }
    }

    pub fn below(mut self, r: &egui::Response) -> Self {
        self.area = self.area.fixed_pos(r.rect.left_bottom());
        self
    }
    // TODO: modify `over()` to handle the case where the label is up against
    //       the right side of the UI, then review uses of `at()`
    pub fn at(mut self, ui: &mut egui::Ui, r: &egui::Response, fudge: egui::Vec2) -> Self {
        let padding = ui.spacing().window_margin.left_top() + ui.spacing().button_padding + fudge;
        self.area = self.area.fixed_pos(r.rect.left_top() - padding);
        self
    }
    /// Same as `at()` but sets width as well.
    pub fn over(mut self, ui: &mut egui::Ui, r: &egui::Response, vertical_fudge: f32) -> Self {
        self = self.at(ui, r, egui::vec2(0.0, vertical_fudge));
        if !self.text_edit_width.is_some_and(|w| w > r.rect.width()) {
            self.text_edit_width = Some(r.rect.width());
        }
        self
    }

    pub fn label(mut self, label: impl ToString) -> Self {
        self.label = Some(label.to_string());
        self
    }
    pub fn text_edit_align(mut self, align: egui::Align) -> Self {
        self.text_edit_align = Some(align);
        self
    }
    /// Trims whitespace from the beginning and end of the text before
    /// confirming. Defaults to `true`.
    pub fn text_edit_trim(mut self, trim: bool) -> Self {
        self.text_edit_trim = trim;
        self
    }
    /// Sets the font of the text editor to monospace. Defaults to `false`.
    pub fn text_edit_monospace(mut self) -> Self {
        self.text_edit_monospace = true;
        self
    }
    /// Sets the exact width of the text edit.
    pub fn text_edit_width(mut self, w: f32) -> Self {
        self.text_edit_width = Some(w);
        self
    }
    /// Adds hint text to the text edit.
    pub fn text_edit_hint(mut self, hint_text: impl ToString) -> Self {
        self.text_edit_hint_text = Some(hint_text.to_string());
        self
    }

    /// If true, "confirms" the result every frame when possible. This is good
    /// for previewing changes live. Defaults to `false`.
    pub fn auto_confirm(mut self, auto_confirm: bool) -> Self {
        self.auto_confirm = auto_confirm;
        self
    }
    pub fn confirm_button_validator(mut self, confirm_validator: TextEditValidator<'v>) -> Self {
        self.validate_confirm = Some(confirm_validator);
        self
    }
    pub fn delete_button_validator(mut self, delete_validator: TextEditValidator<'v>) -> Self {
        self.validate_delete = Some(delete_validator);
        self
    }

    /// Opens the popup.
    pub fn open(&mut self, initial_value: String) {
        self.ctx.memory_mut(|mem| mem.open_popup(self.new_name.id));
        self.new_name.set(Some(initial_value));
        self.is_first_frame = true;
    }
    /// Toggles the popup and returns whether it is now open.
    pub fn toggle(&mut self, initial_value: String) -> bool {
        if self.is_open() {
            self.ctx.memory_mut(|mem| mem.close_popup());
            false
        } else {
            self.open(initial_value);
            true
        }
    }
    pub fn is_open(&self) -> bool {
        self.ctx.memory(|mem| mem.is_popup_open(self.new_name.id))
    }

    /// Shows the text edit popup if it is open.
    pub fn show(self, ui: &mut egui::Ui) -> Option<TextEditPopupResponse> {
        self.show_with(ui, |_| ()).0
    }

    /// Shows the text edit popup if it is open, and calls `inner` to display
    /// extra UI below the text edit component.
    pub fn show_with<R>(
        self,
        ui: &mut egui::Ui,
        inner: impl FnOnce(&mut egui::Ui) -> R,
    ) -> (Option<TextEditPopupResponse>, Option<R>) {
        let mut response = None;
        let mut inner_response = None;

        if self.is_open() {
            let area_response = self.area.show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_height(BIG_ICON_BUTTON_SIZE.y);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if let Some(label) = self.label {
                            ui.strong(label);
                        }

                        let mut s = self.new_name.get().unwrap_or_default();
                        let mut text_edit = egui::TextEdit::singleline(&mut s);
                        if let Some(align) = self.text_edit_align {
                            text_edit = text_edit.horizontal_align(align);
                        }
                        if self.text_edit_monospace {
                            text_edit = text_edit.font(egui::TextStyle::Monospace);
                        }
                        if let Some(w) = self.text_edit_width {
                            text_edit = text_edit.desired_width(w);
                        }
                        if let Some(hint_text) = self.text_edit_hint_text {
                            text_edit = text_edit.hint_text(hint_text);
                        }
                        let r = text_edit.show(ui);
                        if self.is_first_frame {
                            crate::gui::util::focus_and_select_all(ui, r);
                        }
                        self.new_name.set(Some(s.clone()));

                        let s = if self.text_edit_trim { s.trim() } else { &s };
                        if let Some(validator) = self.validate_confirm {
                            if self.auto_confirm || validated_button(ui, "✔", validator(&s), true)
                            {
                                response = Some(TextEditPopupResponse::Confirm(s.to_string()));
                                if !self.auto_confirm
                                    || ui.input(|input| input.key_pressed(egui::Key::Enter))
                                {
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            }
                        }
                        if let Some(validator) = self.validate_delete {
                            if validated_button(ui, "🗑", validator(&s), false) {
                                response = Some(TextEditPopupResponse::Delete);
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        }
                    });
                    inner_response = Some(inner(ui));
                });
            });

            let clicked_elsewhere =
                crate::gui::util::clicked_elsewhere(ui, &area_response.response);
            if (clicked_elsewhere && !self.is_first_frame)
                || ui.input(|input| input.key_pressed(egui::Key::Escape))
            {
                response = Some(TextEditPopupResponse::Cancel);
                ui.memory_mut(|mem| mem.close_popup());
            }
        }

        (response, inner_response)
    }
}

fn validated_button(
    ui: &mut egui::Ui,
    icon: &str,
    validation_result: Result<Option<String>, Option<String>>,
    accept_enter: bool,
) -> bool {
    ui.add_enabled_ui(validation_result.is_ok(), |ui| {
        let mut r = ui.add(egui::Button::new(icon).min_size(BIG_ICON_BUTTON_SIZE));
        r = match &validation_result {
            Ok(Some(hover_text)) => r.on_hover_text(hover_text),
            Err(Some(hover_text)) => r.on_disabled_hover_text(hover_text),
            Ok(None) | Err(None) => r,
        };
        if validation_result.is_ok() {
            return r.clicked()
                || (accept_enter && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        }

        false
    })
    .inner
}

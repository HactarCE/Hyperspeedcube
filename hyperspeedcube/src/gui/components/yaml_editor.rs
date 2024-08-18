use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::gui::ext::ResponseExt;

use super::BIG_ICON_BUTTON_SIZE;

#[derive(Debug, Clone)]
struct PlaintextYamlEditorState {
    contents: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PlaintextYamlEditor<T> {
    pub id: egui::Id,
    _marker: PhantomData<T>,
}
impl<T> PlaintextYamlEditor<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
{
    pub fn new(ui: &mut egui::Ui) -> PlaintextYamlEditor<T> {
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn is_open(&self, ui: &egui::Ui) -> bool {
        self.state(ui).is_some()
    }
    pub fn open(&self, ui: &egui::Ui, value: &T) {
        self.set_state(
            ui,
            Some(PlaintextYamlEditorState {
                contents: serde_yml::to_string(value)
                    .unwrap_or_else(|e| format!("serialization error: {e}")),
            }),
        );
    }
    pub fn close(&self, ui: &egui::Ui) {
        self.set_state(ui, None);
    }

    pub fn show_edit_as_plaintext_button(&self, ui: &mut egui::Ui, value: &T) -> egui::Response {
        let r = ui
            .add_sized(
                BIG_ICON_BUTTON_SIZE,
                egui::SelectableLabel::new(self.is_open(ui), "✏"),
            )
            .on_i18n_hover_explanation("edit_as_plaintext");
        if r.clicked() {
            match self.is_open(ui) {
                true => self.close(ui),
                false => self.open(ui, value),
            }
        }
        r
    }

    fn state(&self, ui: &egui::Ui) -> Option<PlaintextYamlEditorState> {
        ui.data(|data| data.get_temp::<PlaintextYamlEditorState>(self.id))
    }
    fn set_state(&self, ui: &egui::Ui, state: Option<PlaintextYamlEditorState>) {
        match state {
            Some(state) => {
                ui.data_mut(|data| data.insert_temp::<PlaintextYamlEditorState>(self.id, state))
            }
            None => ui.data_mut(|data| data.remove::<PlaintextYamlEditorState>(self.id)),
        }
    }

    pub fn show(&self, ui: &mut egui::Ui) -> Option<egui::Response> {
        self.state(ui).map(|mut state| {
            let deserialization_error = self.deserialize(ui).and_then(|result| result.err());
            if deserialization_error.is_some() {
                // Change the outline around the text editor to red.
                let visuals = ui.visuals_mut();
                let error_color = visuals.error_fg_color;
                visuals.selection.stroke.color = error_color;
                visuals.widgets.active.bg_stroke.color = error_color;
                visuals.widgets.hovered.bg_stroke.color = error_color;
                visuals.widgets.inactive.bg_stroke.color = error_color;
            }

            // Setting `.lock_focus(true)` makes the tab key insert tab
            // characters (`\t`). YAML wants two spaces per tab, but
            // right now there's no easy way to make that happen.
            let r = egui::TextEdit::multiline(&mut state.contents)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .lock_focus(false)
                .min_size(ui.available_size())
                .show(ui);

            if r.response.changed() {
                self.set_state(ui, Some(state));
            }

            r.response
        })
    }

    pub fn deserialize(&self, ui: &egui::Ui) -> Option<serde_yml::Result<T>> {
        self.state(ui)
            .map(|state| serde_yml::from_str(&state.contents))
    }
}

use serde::{Deserialize, Serialize};

use crate::gui::components::big_icon_button;
use crate::gui::ext::*;

#[derive(Debug, Clone)]
struct PlaintextYamlEditorState {
    contents: String,
    modified: bool,
}

pub struct PlaintextYamlEditor {
    pub id: egui::Id,
}
impl PlaintextYamlEditor {
    pub fn get(id: egui::Id) -> PlaintextYamlEditor {
        Self { id }
    }

    pub fn is_active(&self, ui: &egui::Ui) -> bool {
        self.state(ui).is_some()
    }
    pub fn open<T>(&self, ui: &egui::Ui, value: &T)
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        self.set_state(
            ui,
            Some(PlaintextYamlEditorState {
                contents: serde_yaml::to_string(value)
                    .unwrap_or_else(|e| format!("serialization error: {e}")),
                modified: false,
            }),
        );
    }
    pub fn close(&self, ui: &egui::Ui) {
        todo!()
        // self.
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

    pub fn show<T>(&self, ui: &mut egui::Ui, value: &mut T) -> Option<egui::Response>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        self.state(ui).map(|mut state| {
            let mut changed = false;

            let mut r = ui.scope(|ui| {
                ui.horizontal(|ui| {
                    let parsed_value: Result<T, _> = serde_yaml::from_str(&state.contents);
                    ui.add_enabled_ui(parsed_value.is_ok(), |ui| {
                        if big_icon_button(ui, "✔", "Confirm changes").clicked() {
                            self.set_state(ui, None);
                            *value = parsed_value.as_ref().unwrap().clone();
                            changed = true;
                        }
                    });
                    if big_icon_button(ui, "✖", "Discard changes").clicked() {
                        self.set_state(ui, None);
                    }
                    if big_icon_button(ui, "🗐", "Click to copy").clicked() {
                        ui.output_mut(|out| out.copied_text = state.contents.clone());
                    }
                    if let Err(e) = parsed_value {
                        ui.label(
                            egui::RichText::new("Parse error (hover for info)")
                                .color(egui::Color32::RED),
                        )
                        .on_hover_explanation("", &e.to_string());
                    }
                });

                ui.separator();

                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    // Setting `.lock_focus(true)` makes the tab key insert tab
                    // characters (`\t`). YAML wants two spaces per tab, but
                    // right now there's no easy way to make that happen.
                    let r = egui::TextEdit::multiline(&mut state.contents)
                        .code_editor()
                        .lock_focus(false)
                        .desired_width(f32::INFINITY)
                        .show(ui);

                    if r.response.changed() {
                        state.modified = true;
                        self.set_state(ui, Some(state));
                    }
                });
            });

            if changed {
                r.response.mark_changed();
            }
            r.response
        })
    }
}

use serde::{Deserialize, Serialize};

use crate::gui::util;
use crate::gui::widgets;
use crate::preferences::Preset;

// TODO: consolidate
const SQUARE_BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 22.0);
fn square_button(ui: &mut egui::Ui, text: &str, hover_text: &str) -> egui::Response {
    ui.add_sized(SQUARE_BUTTON_SIZE, egui::Button::new(text))
        .on_hover_text(hover_text)
}

pub struct PresetsUi<'a, T> {
    pub id: egui::Id,
    pub presets: &'a mut Vec<Preset<T>>,
    pub changed: &'a mut bool,
}
impl<T> PresetsUi<'_, T>
where
    T: Serialize + for<'de> Deserialize<'de> + Default + Clone,
{
    fn plaintext_yaml_editor(&self) -> util::PlaintextYamlEditor {
        util::PlaintextYamlEditor {
            id: self.id,
            button_size: SQUARE_BUTTON_SIZE,
        }
    }

    pub fn show_header_with_active_preset(
        &mut self,
        ui: &mut egui::Ui,
        current: &T,
        active: &mut Option<Preset<T>>,
    ) {
        self._show_header(ui, || current.clone(), Some(active))
    }
    pub fn show_header(&mut self, ui: &mut egui::Ui, get_current: impl FnOnce() -> T) {
        self._show_header(ui, get_current, None)
    }
    fn _show_header(
        &mut self,
        ui: &mut egui::Ui,
        get_current: impl FnOnce() -> T,
        active_preset: Option<&mut Option<Preset<T>>>,
    ) {
        let mut manage_presets = ui.data().get_temp::<bool>(self.id).unwrap_or(false);
        ui.checkbox(&mut manage_presets, "Manage presets");
        ui.data().insert_temp::<bool>(self.id, manage_presets);

        if !manage_presets {
            return;
        }

        if let Some(r) = self.plaintext_yaml_editor().show(ui, self.presets) {
            *self.changed |= r.changed();
            return;
        }

        ui.horizontal(|ui| {
            if square_button(ui, "✏", "Edit as plaintext").clicked() {
                self.plaintext_yaml_editor().set_active(ui, self.presets);
            }

            let preset_name_id = self.id.with("preset_name");
            let mut preset_name = ui
                .data()
                .get_temp::<String>(preset_name_id)
                .unwrap_or_default();
            let trimmed_preset_name = preset_name.trim().to_string();
            let is_preset_name_valid = !trimmed_preset_name.is_empty();

            let button_resp = ui
                .add_enabled_ui(is_preset_name_valid, |ui| {
                    square_button(ui, "➕", "Save preset")
                })
                .inner;
            let button_clicked = button_resp.clicked();

            let text_edit_resp = ui.add(
                egui::TextEdit::singleline(&mut preset_name)
                    .hint_text("Preset name")
                    .desired_width(f32::INFINITY),
            );
            let text_edit_confirmed =
                text_edit_resp.lost_focus() && ui.input().key_pressed(egui::Key::Enter);

            if (button_clicked || text_edit_confirmed) && is_preset_name_valid {
                let new_preset = Preset {
                    preset_name: trimmed_preset_name,
                    value: get_current(),
                };
                if let Some(active) = active_preset {
                    *active = Some(new_preset.clone());
                }
                self.presets.push(new_preset);
                preset_name.clear();
                *self.changed = true;
            }

            ui.data().insert_temp(preset_name_id, preset_name);
        });
    }

    pub fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        mut preset_ui: impl FnMut(&mut egui::Ui, usize, &mut Preset<T>) -> egui::Response,
    ) {
        let manage_presets = ui.data().get_temp::<bool>(self.id).unwrap_or(false);

        if manage_presets {
            if !self.plaintext_yaml_editor().is_active(ui) {
                // egui::ScrollArea::new([false, true]).show(ui, |ui| {
                *self.changed |= widgets::ReorderableList::new(self.id, self.presets)
                    .button_size(SQUARE_BUTTON_SIZE)
                    .show(ui, preset_ui)
                    .changed();
                // });
            }
        } else {
            for (idx, preset) in self.presets.iter_mut().enumerate() {
                ui.horizontal(|ui| *self.changed |= preset_ui(ui, idx, preset).changed());
            }
        }

        // // TODO: what is this for?
        // if ui.available_height() > 0.0 {
        //     ui.allocate_space(ui.available_size());
        // }
    }
}

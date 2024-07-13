use serde::{Deserialize, Serialize};

use super::{
    HelpHoverWidget, PrefsUi, TextEditPopup, TextEditPopupResponse, BIG_ICON_BUTTON_SIZE,
    SMALL_ICON_BUTTON_SIZE,
};
use crate::gui::components::PlaintextYamlEditor;
use crate::gui::ext::ResponseExt;
use crate::gui::util::{body_text_format, strong_text_format, EguiTempValue};
use crate::preferences::{Preferences, Preset, WithPresets, DEFAULT_PREFS};

const PRESET_NAME_TEXT_EDIT_WIDTH: f32 = 150.0;

fn show_presets_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Presets");
    ui.label(
        "A preset is a saved set of values \
         that can be loaded at any time.",
    );
    crate::gui::util::bullet_list(
        ui,
        &[
            "Click on the + button to create a preset",
            "Click on a preset to activate it",
            "Right-click on a preset to rename or delete it",
            "Drag a preset to reorder it",
        ],
    );
    ui.label("Loading a preset discards unsaved changes.");
}

pub struct PresetsUiText<'a> {
    /// Set of presets, if any.
    pub presets_set: Option<&'a str>,
    /// String standing in for the word "preset" in this context.
    pub preset: &'a str,
    /// String standing in for the word "presets" in this context.
    pub presets: &'a str,
    /// String for what the preset is for.
    pub what: &'a str,
}
impl Default for PresetsUiText<'_> {
    fn default() -> Self {
        Self {
            presets_set: None,
            preset: "preset",
            presets: "presets",
            what: "settings",
        }
    }
}

pub struct PresetsUi<'a, T: Default> {
    /// Unique widget ID.
    pub id: egui::Id,
    /// Presets that the user can add/remove/modify.
    pub presets: &'a mut WithPresets<T>,
    /// Whether any part of the presets state has changed this frame.
    pub changed: &'a mut bool,
    /// Text strings to put on the UI.
    pub text: PresetsUiText<'a>,
}
impl<'a, T> PresetsUi<'a, T>
where
    T: Serialize + for<'de> Deserialize<'de> + Default + Clone + PartialEq,
{
    fn validate_preset_name(
        &self,
        new_name: &str,
        verb: &str,
    ) -> Result<Option<String>, Option<String>> {
        if new_name.is_empty() {
            Err(Some("Name cannot be empty".to_string()))
        } else if !self.presets.is_name_available(&new_name) {
            Err(Some(format!(
                "There is already a {} with this name",
                self.text.preset,
            )))
        } else {
            Ok(Some(format!("{verb} {}", self.text.preset)))
        }
    }

    /// Shows a selectable label with a preset name on it. The name is clipped
    /// to the available space so that it does not wrap.
    fn show_preset_name_selectable_label(
        &self,
        ui: &mut egui::Ui,
        preset_name: &str,
    ) -> egui::Response {
        let is_current = preset_name == self.presets.last_loaded_name();

        let max_width = ui.available_width() - ui.spacing().button_padding.x * 2.0;
        let elided_preset_name = elide_overflowing_line(ui, preset_name, max_width);

        let mut r = ui
            .selectable_label(is_current, &elided_preset_name)
            .interact(egui::Sense::drag());

        if elided_preset_name != preset_name && !ui.memory(|mem| mem.any_popup_open()) {
            r = r.on_hover_text(preset_name);
        }

        r
    }

    pub fn show_presets_selector(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.set_min_width(200.0);

        let mut preset_to_activate = None;
        let preset_to_edit = EguiTempValue::new(ui);
        let mut edit_popup = TextEditPopup::new(ui);
        let mut new_popup = TextEditPopup::new(ui);
        let mut dnd = super::DragAndDrop::new(ui).dragging_opacity(0.4);

        // Presets selector.
        let r = ui.group(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong(format!("Saved {}", self.text.presets));
                if let Some(presets_set) = self.text.presets_set {
                    ui.label(format!("({presets_set})"));
                }
                HelpHoverWidget::show(ui, show_presets_help_ui);
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for preset in self.presets.builtin_list() {
                    crate::gui::util::wrap_if_needed_for_button(ui, &preset.name);
                    let r = ui.add_enabled(!dnd.is_dragging(), |ui: &mut egui::Ui| {
                        self.show_preset_name_selectable_label(ui, &preset.name)
                    });

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name.clone());
                    }

                    // Don't handle other interaction. We can't edit or reorder
                    // this preset.
                }

                for preset in self.presets.user_list() {
                    crate::gui::util::wrap_if_needed_for_button(ui, &preset.name);
                    let r = dnd.draggable(ui, preset.name.clone(), |ui, _is_dragging| {
                        self.show_preset_name_selectable_label(ui, &preset.name)
                    });
                    let r = r.inner;

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name.clone());
                    }

                    // Right-click -> Edit preset
                    if r.secondary_clicked() && edit_popup.toggle(preset.name.clone()) {
                        preset_to_edit.set(Some(preset.name.clone()));
                    }

                    // Drag -> Reorder preset
                    dnd.reorder_drop_zone(ui, &r, preset.name.clone());
                }

                ui.set_enabled(!dnd.is_dragging());
                let mut r = ui.add(egui::Button::new("+").min_size(SMALL_ICON_BUTTON_SIZE));
                if !ui.memory(|mem| mem.any_popup_open()) {
                    r = r.on_hover_text(format!("Add {}", self.text.preset));
                }

                // Left click -> New preset
                if r.clicked() {
                    new_popup.toggle(String::new());
                }
            });
        });

        let edit_popup_response = edit_popup.if_open(|popup| {
            popup
                .below(&r.response)
                .label(format!("Rename {}", self.text.preset))
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(format!("New {} name", self.text.preset))
                .confirm_button_validator(Box::new(|new_name| {
                    self.validate_preset_name(new_name, "Rename")
                }))
                .delete_button_validator(Box::new(|_| {
                    if self.presets.len() > 1 {
                        Ok(Some(format!("Delete {}", self.text.preset)))
                    } else {
                        Ok(Some(format!("Cannot delete last {}", self.text.preset)))
                    }
                }))
                .show(ui)
        });
        if let Some(r) = edit_popup_response {
            if let Some(preset_name) = preset_to_edit.take() {
                match r {
                    TextEditPopupResponse::Confirm(new_name) => {
                        self.presets.rename(&preset_name, &new_name);
                        *self.changed = true;
                    }
                    TextEditPopupResponse::Delete => {
                        self.presets.delete(&preset_name);
                        *self.changed = true;
                    }
                    TextEditPopupResponse::Cancel => (),
                }
            }
        }

        let new_popup_response = new_popup.if_open(|popup| {
            popup
                .below(&r.response)
                .label(format!("Add {}", self.text.preset))
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(format!("New {} name", self.text.preset))
                .confirm_button_validator(Box::new(|new_name| {
                    self.validate_preset_name(new_name, "Add")
                }))
                .show(ui)
        });
        if let Some(r) = new_popup_response {
            match r {
                TextEditPopupResponse::Confirm(new_name) => {
                    self.presets.add_preset(new_name.clone());
                    preset_to_activate = Some(new_name);
                }
                TextEditPopupResponse::Delete | TextEditPopupResponse::Cancel => (),
            }
        }

        // Activate the new preset.
        if let Some(preset_to_activate) = preset_to_activate {
            self.presets.load_preset(&preset_to_activate);
            *self.changed = true;
        }

        // Reorder the presets.
        dnd.paint_reorder_drop_lines(ui);
        if let Some(r) = dnd.end_drag() {
            if let Some(before_or_after) = r.before_or_after {
                self.presets.reorder(&r.payload, &r.end, before_or_after);
                *self.changed = true;
            }
        }

        if *self.changed {
            ui.ctx().request_repaint();
        }

        r.response
    }

    pub fn show_current_prefs_ui<'b>(
        &mut self,
        ui: &mut egui::Ui,
        get_backup_defaults: impl FnOnce(&'b Preferences) -> Option<&'b Preset<T>>,
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        let defaults = match self.presets.last_loaded_preset() {
            Some(p) => p.clone(),
            None => get_backup_defaults(&DEFAULT_PREFS)
                .cloned()
                .unwrap_or_else(|| Preset {
                    name: "Default".to_string(),
                    value: T::default(),
                }),
        };
        let current = self.presets.preset_to_save();
        let is_unsaved = self.presets.is_modified();

        let mut save_changes = false;

        let yaml = PlaintextYamlEditor::<T>::new(ui);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Save button
                    ui.add_enabled_ui(is_unsaved, |ui| {
                        let new_name = &current.name;
                        let overwrite = self.presets.has(new_name);
                        let r = ui
                            .add_sized(BIG_ICON_BUTTON_SIZE, egui::Button::new("💾"))
                            .on_hover_explanation(
                                "Save changes",
                                &format!(
                                    "{save} {preset} {new_name}",
                                    save = if overwrite { "Overwrite" } else { "Add new" },
                                    preset = self.text.preset,
                                ),
                            );
                        save_changes |= r.clicked();
                    });

                    // Edit as plaintext button
                    yaml.show_edit_as_plaintext_button(ui, &current.value);

                    let mut job = egui::text::LayoutJob::default();
                    job.append(&current.name, 0.0, strong_text_format(ui));
                    job.append(" ", 0.0, body_text_format(ui));
                    job.append(&self.text.what, 0.0, body_text_format(ui));
                    crate::gui::util::label_centered_unless_multiline(ui, job);
                });
            });
            ui.separator();
            egui::ScrollArea::both()
                .id_source(self.id.with(yaml.is_open(ui)))
                .auto_shrink(false)
                .show(ui, |ui| match yaml.is_open(ui) {
                    true => {
                        if let Some(r) = yaml.show(ui) {
                            if r.changed() {
                                // Update value from YAML editor.
                                if let Some(Ok(deserialized)) = yaml.deserialize(ui) {
                                    self.presets.current = deserialized;
                                    *self.changed |= r.changed();
                                }
                            }
                        }
                    }
                    false => {
                        add_contents(PrefsUi {
                            ui,
                            current: &mut self.presets.current,
                            defaults: Some(&defaults.value),
                            changed: &mut self.changed,
                        });
                    }
                });
        });

        if save_changes {
            self.presets.save_preset();
            *self.changed = true;
        }
    }
}

fn elide_overflowing_line(ui: &mut egui::Ui, s: &str, max_width: f32) -> String {
    let mut job = egui::text::LayoutJob::default();
    job.append(s, 0.0, body_text_format(ui));
    job.wrap.max_rows = 1;
    job.wrap.max_width = max_width;
    ui.fonts(|fonts| fonts.layout_job(job))
        .rows
        .first()
        .map(|row| row.text())
        .unwrap_or_default()
}

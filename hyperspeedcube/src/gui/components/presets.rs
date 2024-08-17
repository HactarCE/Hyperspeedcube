use serde::{Deserialize, Serialize};

use super::{
    HelpHoverWidget, PrefsUi, TextEditPopup, TextEditPopupResponse, BIG_ICON_BUTTON_SIZE,
    SMALL_ICON_BUTTON_SIZE,
};
use crate::gui::components::PlaintextYamlEditor;
use crate::gui::ext::ResponseExt;
use crate::gui::markdown::md_inline;
use crate::gui::util::{
    body_text_format, set_widget_spacing_to_space_width, strong_text_format, EguiTempValue,
};
use crate::preferences::{Preferences, Preset, WithPresets, DEFAULT_PREFS};

pub const PRESET_NAME_TEXT_EDIT_WIDTH: f32 = 150.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PresetsUiText<'a> {
    /// Set of presets, if any.
    pub presets_set: Option<&'a str>,
    /// String standing in for the word "preset" in this context.
    pub preset: &'a str,
    /// String standing in for the phrase "Saved presets" in this context.
    pub saved_presets: &'a str,
    /// String for what the preset is for.
    pub what: &'a str,
}
impl Default for PresetsUiText<'_> {
    fn default() -> Self {
        Self {
            presets_set: None,
            preset: "preset",
            saved_presets: "Saved presets",
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
    /// Whether to automatically save changes.
    pub autosave: bool,
    /// Whether to allow vertical scrolling in the content area.
    pub vscroll: bool,
    /// Help text to show for the current settings UI.
    pub help_contents: Option<&'a str>,
    /// Function to apply context-specific validation for new preset names.
    /// Whether or not this is present, names will still be checked for
    /// uniqueness and non-emptiness.
    pub extra_validation: Option<Box<dyn Fn(&Self, &str) -> Result<(), String>>>,
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
        } else if let Some(Err(e)) = self.extra_validation.as_ref().map(|f| f(self, new_name)) {
            Err(Some(e))
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

    pub fn show<'b>(
        mut self,
        ui: &mut egui::Ui,
        get_backup_defaults: impl FnOnce(&'b Preferences) -> Option<Preset<T>>,
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        ui.group(|ui| {
            self.show_wrapping_presets_selector(ui);
            // TODO: reconsider spacing
            ui.add_space(ui.spacing().item_spacing.y);
            ui.separator();
            ui.add_space(ui.spacing().item_spacing.y);
            self.show_preset_editor(ui, get_backup_defaults, add_contents);
        });
    }

    pub fn show_wrapping_presets_selector(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.set_min_width(200.0);

        let can_delete = self.presets.len() > 1;

        let mods = ui.input(|input| input.modifiers);
        let cmd = mods.command;
        let alt = mods.alt;

        let mut preset_to_activate = None;
        let preset_to_edit = EguiTempValue::new(ui);
        let mut preset_to_delete = None;
        let mut edit_popup = TextEditPopup::new(ui);
        let mut new_popup = TextEditPopup::new(ui);
        let mut dnd = super::DragAndDrop::new(ui).dragging_opacity(0.4);

        // Presets selector.
        let r = ui.scope(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong(self.text.saved_presets);
                if let Some(presets_set) = self.text.presets_set.filter(|s| !s.is_empty()) {
                    ui.label(format!("({presets_set})"));
                }
                HelpHoverWidget::show_right_aligned(ui, crate::strings::PRESETS_HELP);
            });
            ui.add_space(ui.spacing().item_spacing.y);
            ui.horizontal_wrapped(|ui| {
                for preset in self.presets.builtin_list() {
                    crate::gui::util::wrap_if_needed_for_button(ui, &preset.name);
                    let r = ui.add_enabled(!dnd.is_dragging(), |ui: &mut egui::Ui| {
                        self.show_preset_name_selectable_label(ui, &preset.name)
                            .on_hover_text(md_inline(ui, "**Click** to activate"))
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
                        let r = self.show_preset_name_selectable_label(ui, &preset.name);
                        egui::InnerResponse::new((), r)
                    });
                    let r = r.inner.response.on_hover_ui(|ui| {
                        // TODO: markdown renderer
                        set_widget_spacing_to_space_width(ui);
                        ui.horizontal(|ui| {
                            ui.strong("Click");
                            ui.label("to activate");
                        });
                        ui.horizontal(|ui| {
                            ui.strong("Right-click");
                            ui.label("to rename");
                        });
                        ui.horizontal(|ui| {
                            ui.strong("Drag");
                            ui.label("to reorder");
                        });
                        ui.add_enabled_ui(can_delete, |ui| {
                            ui.horizontal(|ui| {
                                ui.strong("Middle-click");
                                ui.label("or");
                                ui.strong("alt + click");
                                ui.label("to delete");
                            });
                        });
                    });

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name.clone());
                    }

                    // Right-click -> Edit preset
                    if r.secondary_clicked() && edit_popup.toggle(preset.name.clone()) {
                        preset_to_edit.set(Some(preset.name.clone()));
                    }

                    // Middle-click -> Delete preset
                    if r.middle_clicked() || alt && !cmd && r.clicked() {
                        preset_to_delete = Some(preset.name.clone());
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
                .confirm_button_validator(&|new_name| self.validate_preset_name(new_name, "Rename"))
                .delete_button_validator(&|_| {
                    if self.presets.len() > 1 {
                        Ok(Some(format!("Delete {}", self.text.preset)))
                    } else {
                        Err(Some(format!("Cannot delete last {}", self.text.preset)))
                    }
                })
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
        } else if let Some(preset_name) = preset_to_delete {
            self.presets.delete(&preset_name);
            *self.changed = true;
        }

        let new_popup_response = new_popup.if_open(|popup| {
            popup
                .below(&r.response)
                .label(format!("Add {}", self.text.preset))
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(format!("New {} name", self.text.preset))
                .confirm_button_validator(&|new_name| self.validate_preset_name(new_name, "Add"))
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
        if let Some(r) = dnd.end_drag(ui) {
            if let Some(before_or_after) = r.before_or_after {
                self.presets.reorder(&r.payload, &r.end, before_or_after);
                *self.changed = true;
            }
        }

        if *self.changed {
            // TODO: is this necessary here?
            //       should we use it in other places too?
            ui.ctx().request_repaint();
        }

        r.response
    }

    pub fn show_preset_editor<'b>(
        &mut self,
        ui: &mut egui::Ui,
        get_backup_defaults: impl FnOnce(&'b Preferences) -> Option<Preset<T>>,
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        let mut save_changes = false;

        let defaults = match self.presets.last_loaded_preset() {
            Some(p) => p.clone(),
            None => get_backup_defaults(&DEFAULT_PREFS).unwrap_or_else(|| Preset {
                name: "Default".to_string(),
                value: T::default(),
            }),
        };
        let current = self.presets.preset_to_save();
        let is_unsaved = self.presets.is_modified();
        save_changes |= self.autosave && is_unsaved && self.presets.has(&current.name);

        let yaml = PlaintextYamlEditor::<T>::new(ui);

        ui.add(PresetHeaderUi {
            text: self.text,
            preset_name: &current.name,

            help_contents: self.help_contents,
            yaml: Some((&yaml, &current.value)),
            save_status: if self.autosave {
                PresetSaveStatus::Autosave
            } else {
                PresetSaveStatus::ManualSave {
                    is_unsaved,
                    overwrite: self.presets.has(&current.name),
                }
            },

            save_changes: &mut save_changes,
        });
        ui.add_space(ui.spacing().item_spacing.y);
        egui::ScrollArea::new([true, self.vscroll])
            .id_source(self.id.with(yaml.is_open(ui)))
            .auto_shrink(!self.vscroll)
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

        if save_changes {
            self.presets.save_preset();
            *self.changed = true;
        }
    }
}

pub struct PresetHeaderUi<'a, T> {
    pub text: PresetsUiText<'a>,
    pub preset_name: &'a str,

    pub help_contents: Option<&'a str>,
    pub yaml: Option<(&'a PlaintextYamlEditor<T>, &'a T)>,
    pub save_status: PresetSaveStatus,

    pub save_changes: &'a mut bool,
}
impl<T> egui::Widget for PresetHeaderUi<'_, T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Save button
                match self.save_status {
                    PresetSaveStatus::CantSave { .. } | PresetSaveStatus::Autosave => (),
                    PresetSaveStatus::ManualSave {
                        is_unsaved,
                        overwrite,
                    } => {
                        ui.add_enabled_ui(is_unsaved, |ui| {
                            let r = ui
                                .add_sized(BIG_ICON_BUTTON_SIZE, egui::Button::new("💾"))
                                .on_hover_explanation(
                                    "Save changes",
                                    &format!(
                                        "{save} {preset} {new_name}",
                                        save = if overwrite { "Overwrite" } else { "Add new" },
                                        preset = self.text.preset,
                                        new_name = self.preset_name,
                                    ),
                                );
                            *self.save_changes |= r.clicked();
                        });
                    }
                }

                // Edit as plaintext button
                if let Some((yaml, current_value)) = self.yaml {
                    yaml.show_edit_as_plaintext_button(ui, current_value);
                }

                // Help hover widget
                if let Some(help_contents) = self.help_contents {
                    crate::gui::components::HelpHoverWidget::show(ui, help_contents);
                }

                let mut job = egui::text::LayoutJob::default();
                if self.preset_name.is_empty() {
                    job.append("No ", 0.0, body_text_format(ui));
                } else {
                    job.append(self.preset_name, 0.0, strong_text_format(ui));
                    job.append(" ", 0.0, body_text_format(ui));
                }
                job.append(&self.text.what, 0.0, body_text_format(ui));
                crate::gui::util::label_centered_unless_multiline(ui, job);
            });
        })
        .response
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PresetSaveStatus {
    CantSave {
        message: &'static str,
    },
    Autosave,
    ManualSave {
        /// Whether the preset has unsaved changes. If this is `false`, then the
        /// save button will be disabled.
        is_unsaved: bool,
        /// Whether saving changes to the preset will overwrite an existing
        /// preset.
        overwrite: bool,
    },
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

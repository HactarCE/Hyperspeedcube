use std::borrow::Cow;

use hyperprefs::{ModifiedPreset, PresetData, PresetsList};
use serde::{Deserialize, Serialize};

use super::{
    BIG_ICON_BUTTON_SIZE, HelpHoverWidget, PrefsUi, SMALL_ICON_BUTTON_SIZE, TextEditPopup,
    TextEditPopupResponse, TextValidationResult,
};
use crate::L;
use crate::gui::components::PlaintextYamlEditor;
use crate::gui::ext::ResponseExt;
use crate::gui::markdown::{md, md_bold_user_text, md_inline};
use crate::gui::util::EguiTempValue;
use crate::locales::PresetStrings;

pub const PRESET_NAME_TEXT_EDIT_WIDTH: f32 = 150.0;

pub type NameValidationResult<'a> = Result<(), Cow<'a, str>>;

pub struct PresetsUi<'a, T: PresetData + Default> {
    /// Unique widget ID.
    pub id: egui::Id,
    /// Presets that the user can add/remove/modify.
    pub presets: &'a mut PresetsList<T>,
    /// Preset that is being edited.
    pub current: &'a mut ModifiedPreset<T>,
    /// Whether any part of the presets state has changed this frame.
    pub changed: &'a mut bool,
    /// Text strings to put on the UI.
    pub text: &'a PresetStrings,
    /// Whether to automatically save changes.
    pub autosave: bool,
    /// Whether to allow vertical scrolling in the content area.
    pub vscroll: bool,
    /// Help text to show for the current settings UI.
    pub help_contents: Option<&'a str>,
    /// Function to apply context-specific validation for new preset names.
    ///
    /// Whether or not this is present, names will still be checked for
    /// uniqueness and non-emptiness.
    ///
    /// This can't be a `Box<dyn FnOnce>` because then the lifetime would be
    /// invariant.
    pub extra_validation: Option<fn(&PresetsUi<'_, T>, &str) -> NameValidationResult<'a>>,
}
impl<'a, T: PresetData> PresetsUi<'a, T>
where
    T: Serialize + for<'de> Deserialize<'de> + Default + Clone + PartialEq,
{
    /// Constructs a `PresetsUi` with dummy values. It should be made
    /// noninteractive by calling `ui.disable()` before displaying it.
    ///
    /// Call this using `&mut Default::default()`.
    pub fn new_dummy(
        id: egui::Id,
        args: &'a mut (PresetsList<T>, ModifiedPreset<T>, bool),
    ) -> Self {
        let (presets, current, changed) = args;
        Self::new(id, presets, current, changed)
    }
    /// Constructs a `PresetsUi` with default parameters.
    pub fn new(
        id: egui::Id,
        presets: &'a mut PresetsList<T>,
        current: &'a mut ModifiedPreset<T>,
        changed: &'a mut bool,
    ) -> Self {
        Self {
            id,
            presets,
            current,
            changed,
            text: &L.presets.default,
            autosave: false,
            vscroll: true,
            help_contents: None,
            extra_validation: None,
        }
    }
    /// Sets non-default text strings to put on the UI.
    pub fn with_text(mut self, text: &'a PresetStrings) -> Self {
        self.text = text;
        self
    }
    /// Sets whether to automatically save changes.
    ///
    /// Default: `false`
    pub fn with_autosave(mut self, autosave: bool) -> Self {
        self.autosave = autosave;
        self
    }
    /// Sets whether to allow vertical scrolling in the content area.
    ///
    /// Default: `true`
    pub fn vscroll(mut self, vscroll: bool) -> Self {
        self.vscroll = vscroll;
        self
    }
    /// Adds help text to show for the current settings UI.
    pub fn with_help_contents(mut self, help_contents: &'a str) -> Self {
        self.help_contents = Some(help_contents);
        self
    }
    /// Sets a function to apply context-specific validation for new preset
    /// names.
    ///
    /// Whether or not this is present, names will still be checked for
    /// uniqueness and non-emptiness.
    pub fn with_extra_validation(
        mut self,
        extra_validation: fn(&PresetsUi<'_, T>, &str) -> Result<(), Cow<'a, str>>,
    ) -> Self {
        self.extra_validation = Some(extra_validation);
        self
    }

    fn validate_preset_name(&self, new_name: &str, ok: &'a str) -> TextValidationResult<'a> {
        if new_name.is_empty() {
            Err(Some(self.text.errors.empty_name.into()))
        } else if !self.presets.is_name_available(new_name) {
            Err(Some(self.text.errors.name_conflict.into()))
        } else if let Some(Err(e)) = self.extra_validation.as_ref().map(|f| f(self, new_name)) {
            Err(Some(e))
        } else {
            Ok(Some(ok.into()))
        }
    }

    /// Shows a selectable label with a preset name on it. The name is clipped
    /// to the available space so that it does not wrap.
    fn show_preset_name_selectable_label(
        &self,
        ui: &mut egui::Ui,
        preset_name: &str,
    ) -> egui::Response {
        let current_name = self.current.base.name();

        let is_current = preset_name == current_name;

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
        presets_set: Option<&str>,
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        ui.group(|ui| {
            self.show_wrapping_presets_selector(ui, presets_set);
            // TODO: reconsider spacing
            ui.add_space(ui.spacing().item_spacing.y);
            ui.separator();
            ui.add_space(ui.spacing().item_spacing.y);
            self.show_preset_editor(ui, add_contents);
        });
    }

    pub fn show_wrapping_presets_selector(
        &mut self,
        ui: &mut egui::Ui,
        presets_set: Option<&str>,
    ) -> egui::Response {
        ui.set_min_width(200.0);

        let can_delete = self.presets.len() > 1;

        let mut preset_to_activate = None;
        let preset_to_edit = EguiTempValue::new(ui);
        let preset_to_delete = EguiTempValue::new(ui);
        let mut edit_popup = TextEditPopup::new(ui);
        let mut new_popup = TextEditPopup::new(ui);
        let mut dnd = super::DragAndDrop::new(ui).dragging_opacity(0.4);
        let any_popup = edit_popup.is_open() || new_popup.is_open();

        // Presets selector.
        let r = ui.scope(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong(self.text.saved_presets);
                if let Some(presets_set) = presets_set.filter(|s| !s.is_empty()) {
                    ui.label(format!("({presets_set})"));
                }
                HelpHoverWidget::show_right_aligned(ui, L.help.presets);
            });
            ui.add_space(ui.spacing().item_spacing.y);
            ui.horizontal_wrapped(|ui| {
                for preset in self.presets.user_presets() {
                    crate::gui::util::wrap_if_needed_for_button(ui, preset.name());
                    let r = dnd.draggable(ui, preset.name().clone(), |ui, _is_dragging| {
                        let r = self.show_preset_name_selectable_label(ui, preset.name());
                        egui::InnerResponse::new((), r)
                    });
                    let mut r = r.inner.response;
                    if !any_popup {
                        r = r.on_hover_ui(|ui| {
                            for action in [
                                L.click_to.activate.with(L.inputs.click),
                                L.click_to.rename.with(L.inputs.right_click),
                                L.click_to.reorder.with(L.inputs.drag),
                                crate::gui::middle_click_to_delete_text(ui),
                            ] {
                                md(ui, action);
                            }
                        });
                    }

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name().clone());
                    }

                    // Right-click -> Edit preset
                    if r.secondary_clicked() && edit_popup.toggle(preset.name().clone()) {
                        preset_to_edit.set(Some(preset.name().clone()));
                    }

                    // Middle-click -> Delete preset
                    if crate::gui::middle_clicked(ui, &r).is_some() {
                        preset_to_delete.set(Some(preset.name().clone()));
                    }

                    // Drag -> Reorder preset
                    dnd.reorder_drop_zone(ui, &r, preset.name().clone());
                }

                dnd.disable_ui_if_dragging(ui);
                let mut r = ui.add(egui::Button::new("+").min_size(SMALL_ICON_BUTTON_SIZE));
                if !ui.memory(|mem| mem.any_popup_open()) {
                    r = r.on_hover_text(self.text.actions.add);
                }

                // Left click -> New preset
                if r.clicked() {
                    new_popup.toggle(String::new());
                }
            });
        });

        enum EditPresetAction<T> {
            ResetToBuiltin(T),
        }

        let edit_popup_response = edit_popup.if_open(|popup| {
            popup
                .below(&r.response)
                .label(self.text.actions.rename)
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(self.text.new_name_hint)
                .confirm_button_validator(&|new_name| {
                    self.validate_preset_name(new_name, self.text.actions.rename)
                })
                .delete_button_validator(&|_| {
                    if can_delete {
                        Ok(Some(self.text.actions.delete.into()))
                    } else {
                        Err(Some(self.text.errors.cannot_delete_last.into()))
                    }
                })
                .show_with(ui, |ui| {
                    let preset_name = preset_to_edit.get()?;
                    let builtin_value = self.presets.builtin_presets().get(&preset_name)?;

                    let is_modified_from_builtin =
                        self.presets.is_preset_modified_from_builtin(&preset_name)
                            || (self.current.base.name() == preset_name
                                && self.current.value != *builtin_value);

                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        crate::gui::util::set_menu_style(ui.style_mut());
                        ui.separator();
                        ui.add_enabled_ui(is_modified_from_builtin, |ui| {
                            ui.button(L.presets.reset_to_builtin).clicked().then(|| {
                                TextEditPopupResponse::Other(EditPresetAction::ResetToBuiltin(
                                    builtin_value.clone(),
                                ))
                            })
                        })
                        .inner
                    })
                    .inner
                })
        });
        if let Some(r) = edit_popup_response {
            if let Some(preset_name) = preset_to_edit.take() {
                match r {
                    TextEditPopupResponse::Confirm(new_name) => {
                        self.presets.rename(&preset_name, &new_name);
                        *self.changed = true;
                    }
                    TextEditPopupResponse::Delete => {
                        self.presets.remove(&preset_name);
                        *self.changed = true;
                    }
                    TextEditPopupResponse::Cancel => (),
                    TextEditPopupResponse::Other(EditPresetAction::ResetToBuiltin(
                        builtin_value,
                    )) => {
                        self.presets.save_preset(&preset_name, builtin_value);
                        preset_to_activate = Some(preset_name);
                    }
                }
            }
        } else if let Some(preset_name) = preset_to_delete.get() {
            // Don't delete the last preset.
            if self.presets.len() > 1 {
                self.presets.remove(&preset_name);
                *self.changed = true;
            }
        }

        enum NewPresetAction<T> {
            AddBuiltin(String, T),
            AddAllBuiltins,
        }

        let new_popup_response = new_popup.if_open(|popup| {
            popup
                .below(&r.response)
                .label(self.text.actions.add)
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(self.text.new_name_hint)
                .confirm_button_validator(&|new_name| {
                    self.validate_preset_name(new_name, self.text.actions.add)
                })
                .show_with(ui, |ui| {
                    let builtins = self.presets.builtin_presets();
                    if builtins.is_empty() {
                        return None;
                    }

                    let mut can_add_any = false;
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        crate::gui::util::set_menu_style(ui.style_mut());
                        ui.separator();
                        for (name, value) in builtins {
                            let can_add_this_preset = !self.presets.contains_key(name);
                            can_add_any |= can_add_this_preset;
                            let r = ui.add_enabled_ui(can_add_this_preset, |ui| {
                                ui.button(L.presets.add_named.with(name))
                            });
                            if r.inner.clicked() {
                                return Some(TextEditPopupResponse::Other(
                                    NewPresetAction::AddBuiltin(name.clone(), value.clone()),
                                ));
                            }
                        }
                        if builtins.len() > 1 {
                            ui.separator();
                            let r = ui.add_enabled_ui(can_add_any, |ui| {
                                ui.button(self.text.actions.add_all_builtin)
                            });
                            if r.inner.clicked() {
                                return Some(TextEditPopupResponse::Other(
                                    NewPresetAction::AddAllBuiltins,
                                ));
                            }
                        }
                        None
                    })
                    .inner
                })
        });
        if let Some(r) = new_popup_response {
            match r {
                TextEditPopupResponse::Confirm(new_name) => {
                    self.presets
                        .save_preset(&new_name, self.current.value.clone());
                    preset_to_activate = Some(new_name);
                }
                TextEditPopupResponse::Delete | TextEditPopupResponse::Cancel => (),
                TextEditPopupResponse::Other(NewPresetAction::AddBuiltin(name, value)) => {
                    self.presets.save_preset(&name, value);
                    preset_to_activate = Some(name);
                }
                TextEditPopupResponse::Other(NewPresetAction::AddAllBuiltins) => {
                    for (name, value) in self.presets.builtin_presets().clone() {
                        if !self.presets.contains_key(&name) {
                            self.presets.save_preset(name, value.clone());
                        }
                    }
                    *self.changed = true;
                }
            }
        }

        // Activate the new preset.
        if let Some(p) = preset_to_activate.and_then(|s| self.presets.load(&s)) {
            *self.current = p;
            *self.changed = true;
        }

        // Reorder the presets.
        *self.changed |= dnd.end_reorder(ui, self.presets);

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
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        let mut save_preset = false;

        let defaults = match self.presets.last_loaded() {
            Some(p) => p.value.clone(),
            None => T::default(),
        };

        let yaml = PlaintextYamlEditor::<T>::new(ui);

        let base_preset_name = self.current.base.name();

        ui.add(PresetHeaderUi {
            text: self.text,
            preset_name: &base_preset_name,

            help_contents: self.help_contents,
            yaml: Some((&yaml, &self.current.value)),
            save_status: if self.autosave {
                PresetSaveStatus::Autosave
            } else {
                PresetSaveStatus::ManualSave {
                    is_unsaved: self.presets.is_modified(self.current),
                    overwrite: self.presets.contains_key(&base_preset_name),
                }
            },

            save_preset: &mut save_preset,
        });
        ui.add_space(ui.spacing().item_spacing.y);
        egui::ScrollArea::new([true, self.vscroll])
            .id_salt(self.id.with(yaml.is_open(ui)))
            .auto_shrink(!self.vscroll)
            .show(ui, |ui| match yaml.is_open(ui) {
                true => {
                    if let Some(r) = yaml.show(ui) {
                        if r.changed() {
                            // Update value from YAML editor.
                            if let Some(Ok(deserialized)) = yaml.deserialize(ui) {
                                self.current.value = deserialized;
                                *self.changed = true;
                            }
                        }
                    }
                }
                false => {
                    add_contents(PrefsUi {
                        ui,
                        current: &mut self.current.value,
                        defaults: Some(&defaults),
                        changed: self.changed,
                    });
                }
            });

        let current_name = self.current.base.name();
        save_preset |= self.autosave
            && self.presets.is_modified(self.current)
            && self.presets.contains_key(&current_name);

        if save_preset {
            self.presets.save_over_preset(self.current);
            *self.changed = true;
        }
    }
}

pub struct PresetHeaderUi<'a, T> {
    pub text: &'a PresetStrings,
    pub preset_name: &'a str,

    pub help_contents: Option<&'a str>,
    pub yaml: Option<(&'a PlaintextYamlEditor<T>, &'a T)>,
    pub save_status: PresetSaveStatus,

    pub save_preset: &'a mut bool,
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
                    PresetSaveStatus::Autosave => (),
                    PresetSaveStatus::ManualSave {
                        is_unsaved,
                        overwrite,
                    } => {
                        ui.add_enabled_ui(is_unsaved, |ui| {
                            if is_unsaved {
                                let visuals = ui.visuals_mut();
                                visuals.widgets.inactive.expansion = 2.0;
                                visuals.widgets.inactive.bg_stroke = egui::Stroke {
                                    width: 2.0,
                                    color: visuals.warn_fg_color,
                                };
                            }

                            let r = ui
                                .add_sized(BIG_ICON_BUTTON_SIZE, egui::Button::new("💾"))
                                .on_hover_explanation(L.presets.save_changes, {
                                    let current = md_bold_user_text(self.preset_name);
                                    if overwrite {
                                        L.presets.overwrite_current.with(&current)
                                    } else {
                                        L.presets.create_current.with(&current)
                                    }
                                });
                            *self.save_preset |= r.clicked();
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

                let markdown: Cow<'_, str> = if self.preset_name.is_empty() {
                    self.text.current_empty.into()
                } else {
                    self.text
                        .current
                        .with(&md_bold_user_text(self.preset_name))
                        .into()
                };
                crate::gui::util::label_centered_unless_multiline(ui, md_inline(ui, markdown));
            });
        })
        .response
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PresetSaveStatus {
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
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let color = ui.visuals().text_color();
    let mut job = egui::text::LayoutJob::simple_singleline(s.to_owned(), font_id, color);
    job.wrap.max_rows = 1;
    job.wrap.max_width = max_width;
    ui.fonts(|fonts| fonts.layout_job(job))
        .rows
        .first()
        .map(|row| row.text())
        .unwrap_or_default()
}

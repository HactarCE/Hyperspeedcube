use egui::InnerResponse;
use serde::{Deserialize, Serialize};

use crate::gui::components::{big_icon_button, PlaintextYamlEditor, ReorderableList};
use crate::gui::ext::ResponseExt;
use crate::gui::util::{body_text_format, set_widget_spacing_to_space_width, strong_text_format};
use crate::preferences::{Preferences, Preset, WithPresets, DEFAULT_PREFS};

use super::{HintWidget, PrefsUi, BIG_ICON_BUTTON_SIZE};

struct PresetsState {
    yaml_text: Option<String>,
}

fn show_presets_help_ui(ui: &mut egui::Ui) {
    // TODO: markdown renderer
    ui.spacing_mut().item_spacing.y = 9.0;
    ui.heading("Presets");
    ui.label(
        "A preset is a saved set of values \
         that can be loaded at any time.",
    );
    super::super::util::bullet_list(
        ui,
        &[
            "Click on the + button to create a preset",
            "Click on a preset to activate it",
            "Right click on a preset to rename or delete it",
            "Drag a preset to reorder it",
        ],
    );
    ui.label("Loading a preset discards unsaved changes.");
}

pub struct PresetsUi<'a, T: Default> {
    pub id: egui::Id,
    pub presets: &'a mut WithPresets<T>,
    pub changed: &'a mut bool,
}
impl<'a, T> PresetsUi<'a, T>
where
    T: Serialize + for<'de> Deserialize<'de> + Default + Clone,
{
    fn edit_popup_id(&self, preset_name: &str) -> egui::Id {
        egui::Id::new((self.id, "edit", preset_name))
    }

    pub fn show_presets_selector(
        &mut self,
        ui: &mut egui::Ui,
        add_selector: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let mut preset_to_activate = None;

        // Presets selector.
        let r = ui.group(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong("Saved presets");
                add_selector(ui);
                HintWidget::show(ui, show_presets_help_ui);
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for preset in &self.presets.presets {
                    let is_current = preset.name == self.presets.last_loaded;
                    let r = ui.selectable_label(is_current, &preset.name);

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name.clone());
                        *self.changed = true;
                    }

                    // Right click -> Edit preset
                    if r.secondary_clicked() {
                        ui.close_menu();
                        ui.memory_mut(|mem| mem.toggle_popup(self.edit_popup_id(&preset.name)));
                    }
                }

                ui.allocate_ui_with_layout(
                    egui::Vec2::splat(18.0),
                    egui::Layout {
                        main_dir: egui::Direction::LeftToRight,
                        main_wrap: false,
                        main_align: egui::Align::Center,
                        main_justify: true,
                        cross_align: egui::Align::Center,
                        cross_justify: true,
                    },
                    |ui| {
                        let mut is_open = false;
                        let r = ui.menu_button("+", |ui| {
                            is_open = true;
                            ui.set_max_width(200.0);
                            let s: Option<String> = ui
                                .data_mut(|data| data.get_temp::<String>("my_preset_name".into()));
                            let is_first = s.is_none();
                            let mut s = s.unwrap_or_else(|| "My Preset Name".to_string());
                            let r = ui.text_edit_singleline(&mut s);
                            if is_first {
                                r.request_focus();
                            }
                            ui.data_mut(|data| {
                                data.insert_temp::<String>("my_preset_name".into(), s)
                            });
                            ui.button("Confirm");
                            // ui.button("New empty preset");
                            // ui.button("New preset from current settings");
                        });
                        r.response.on_hover_text("Add preset");
                        if !is_open {
                            ui.data_mut(|data| data.remove_temp::<String>("my_preset_name".into()));
                        }
                    },
                );
            });
        });

        if let Some(preset_to_activate) = preset_to_activate {
            self.presets.load_preset(&preset_to_activate);
            *self.changed = true;
        }

        r.response
        // for &s in &preset_names {
        //     let id = format!("edit_{s}").into();
        //     if ui.memory(|mem| mem.is_popup_open(id)) {
        //         let area_resp = egui::Area::new(id)
        //             .order(egui::Order::Foreground)
        //             .fixed_pos(r.response.rect.left_bottom())
        //             .constrain_to(ui.ctx().available_rect())
        //             // .constrain(true)
        //             // .constrain_to(ui.ctx().screen_rect())
        //             .sense(egui::Sense::hover())
        //             .show(ui.ctx(), |ui| {
        //                 // let style = ui.style_mut();
        //                 // style.spacing.button_padding = egui::vec2(2.0, 0.0);
        //                 // style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        //                 // style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        //                 // style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        //                 // style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

        //                 egui::Frame::menu(ui.style())
        //                     .show(ui, |ui| {
        //                         ui.set_max_width(ui.spacing().menu_width);

        //                         ui.horizontal(|ui| {
        //                             set_widget_spacing_to_space_width(ui);
        //                             ui.strong(s);
        //                             ui.label("preset")
        //                         });
        //                         ui.horizontal(|ui| {
        //                             ui.text_edit_singleline(&mut s.to_string());
        //                             ui.button("Rename");
        //                             ui.button("Delete");
        //                         });
        //                         // ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        //                         //     ui.with_layout(
        //                         //         egui::Layout::left_to_right(egui::Align::TOP),
        //                         //         |ui| {
        //                         //         },
        //                         //     );
        //                         // });
        //                         // ui.with_layout(
        //                         //     egui::Layout::top_down_justified(egui::Align::LEFT),
        //                         //     |ui| {
        //                         //     },
        //                         // )
        //                         // .inner
        //                     })
        //                     .inner
        //             });

        //         if ui.input(|i| i.key_pressed(egui::Key::Escape))
        //             || (area_resp.response.clicked_elsewhere() && r.response.clicked_elsewhere())
        //         {
        //             ui.memory_mut(|mem| mem.close_popup());
        //         }
        //     }
        // }
    }

    pub fn show_current_prefs_ui(
        &mut self,
        ui: &mut egui::Ui,
        get_backup_defaults: impl FnOnce(&Preferences) -> Option<&Preset<T>>,
        add_contents: impl FnOnce(PrefsUi<'_, T>),
    ) where
        T: 'static + PartialEq + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    {
        let last_loaded = &self.presets.last_loaded;
        let defaults = match self
            .presets
            .presets
            .iter_mut()
            .find(|p| p.name == *last_loaded)
        {
            Some(p) => &p.value,
            None => match get_backup_defaults(&DEFAULT_PREFS) {
                Some(p) => &p.value,
                None => &T::default(),
            },
        };
        let current = self.presets.current.get_or_insert_with(|| defaults.clone()); // `defaults.clone()` branch should never happen
        let is_unsaved = current != defaults;
        let mut save_changes = false;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Save button
                    ui.add_enabled_ui(is_unsaved, |ui| {
                        let r = ui
                            .add_sized(BIG_ICON_BUTTON_SIZE, egui::Button::new("💾"))
                            .on_hover_explanation(
                                "Save changes",
                                &format!("Overwrite preset {}", last_loaded),
                            );
                        save_changes |= r.clicked();
                    });

                    // Edit as plaintext button
                    let yaml = PlaintextYamlEditor::get(self.id);
                    let r = ui
                        .add_sized(
                            BIG_ICON_BUTTON_SIZE,
                            egui::SelectableLabel::new(yaml.is_active(ui), "✏"),
                        )
                        .on_hover_explanation(
                            "Edit as plaintext",
                            "View and edit presets as plaintext to share them with others",
                        );
                    if r.clicked() {
                        dbg!("ya! this thing");
                        // yaml.toggle_active(ui);
                    }

                    // TODO: factor out text layout
                    let mut job = egui::text::LayoutJob::default();
                    job.append(last_loaded, 0.0, strong_text_format(ui));
                    job.append(" view settings", 0.0, body_text_format(ui));
                    let widget_text = egui::WidgetText::from(job);
                    let galley = widget_text.clone().into_galley(
                        ui,
                        Some(true),
                        ui.available_width(),
                        egui::TextStyle::Body,
                    );
                    let is_multiline = galley.rows.len() > 1;
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_wrap(is_multiline),
                        |ui| {
                            ui.label(widget_text);
                        },
                    )
                });
            });
            ui.separator();
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                add_contents(PrefsUi {
                    ui,
                    current,
                    defaults,
                    changed: &mut self.changed,
                });
            });
        });

        if save_changes {
            self.presets.save_preset();
            *self.changed = true;
        }
    }

    // fn plaintext_yaml_editor(&self) -> PlaintextYamlEditor {
    //     PlaintextYamlEditor { id: self.id }
    // }

    // pub fn show_header_with_active_preset(
    //     &mut self,
    //     ui: &mut egui::Ui,
    //     get_current: impl FnOnce() -> T,
    //     set_active: impl FnOnce(&Preset<T>),
    // ) {
    //     self._show_header(ui, get_current, set_active)
    // }
    // pub fn show_header(&mut self, ui: &mut egui::Ui, get_current: impl FnOnce() -> T) {
    //     self._show_header(ui, get_current, |_| ())
    // }
    // fn _show_header(
    //     &mut self,
    //     ui: &mut egui::Ui,
    //     get_current: impl FnOnce() -> T,
    //     on_new_preset: impl FnOnce(&Preset<T>),
    // ) {
    //     let mut edit_presets = ui.data(|data| data.get_temp::<bool>(self.id).unwrap_or(false));
    //     ui.checkbox(&mut edit_presets, self.strings.edit);
    //     ui.data_mut(|data| data.insert_temp::<bool>(self.id, edit_presets));

    //     if !edit_presets {
    //         return;
    //     }

    //     if let Some(r) = self.plaintext_yaml_editor().show(ui, self.presets) {
    //         *self.changed |= r.changed();
    //         return;
    //     }

    //     ui.horizontal(|ui| {
    //         ui.add_visible_ui(self.enable_yaml, |ui| {
    //             if big_icon_button(ui, "✏", "Edit as plaintext").clicked() {
    //                 self.plaintext_yaml_editor().set_active(ui, self.presets);
    //             }
    //         });

    //         let preset_name_id = self.id.with("preset_name");
    //         let mut preset_name =
    //             ui.data(|data| data.get_temp::<String>(preset_name_id).unwrap_or_default());
    //         let trimmed_preset_name = preset_name.trim().to_string();
    //         let is_preset_name_valid = !trimmed_preset_name.is_empty();

    //         let button_resp = ui
    //             .add_enabled_ui(is_preset_name_valid, |ui| {
    //                 big_icon_button(ui, "➕", self.strings.save)
    //             })
    //             .inner;
    //         let button_clicked = button_resp.clicked();

    //         let text_edit_resp = ui.add(
    //             egui::TextEdit::singleline(&mut preset_name)
    //                 .hint_text(self.strings.name)
    //                 .desired_width(f32::INFINITY),
    //         );
    //         let text_edit_confirmed = text_edit_resp.lost_focus()
    //             && ui.input(|input| input.key_pressed(egui::Key::Enter));

    //         if (button_clicked || text_edit_confirmed) && is_preset_name_valid {
    //             let new_preset = Preset {
    //                 name: trimmed_preset_name,
    //                 value: get_current(),
    //             };
    //             on_new_preset(&new_preset);
    //             self.presets.push(new_preset);
    //             preset_name.clear();
    //             *self.changed = true;
    //         }

    //         ui.data_mut(|data| data.insert_temp(preset_name_id, preset_name));
    //     });
    // }

    // pub fn show_postheader<R>(
    //     &mut self,
    //     ui: &mut egui::Ui,
    //     postheader_ui: impl FnOnce(&mut egui::Ui) -> R,
    // ) -> Option<R> {
    //     let edit_presets = ui.data(|data| data.get_temp::<bool>(self.id).unwrap_or(false));
    //     (edit_presets && !self.plaintext_yaml_editor().is_active(ui)).then(|| postheader_ui(ui))
    // }

    // pub fn show_list(
    //     &mut self,
    //     ui: &mut egui::Ui,
    //     mut preset_ui: impl FnMut(&mut egui::Ui, usize, &mut Preset<T>) -> egui::Response,
    // ) {
    //     let edit_presets = ui.data(|data| data.get_temp::<bool>(self.id).unwrap_or(false));

    //     if edit_presets {
    //         if !self.plaintext_yaml_editor().is_active(ui) {
    //             *self.changed |= ReorderableList::new(self.id, self.presets)
    //                 .show(ui, preset_ui)
    //                 .changed();
    //         }
    //     } else {
    //         for (idx, preset) in self.presets.iter_mut().enumerate() {
    //             ui.horizontal(|ui| *self.changed |= preset_ui(ui, idx, preset).changed());
    //         }
    //     }
    // }
}

// #[derive(Debug, Copy, Clone)]
// pub struct PresetsUiStrings {
//     pub edit: &'static str,
//     pub save: &'static str,
//     pub name: &'static str,
// }
// impl Default for PresetsUiStrings {
//     fn default() -> Self {
//         Self {
//             edit: "Edit presets",
//             save: "Save preset",
//             name: "Preset name",
//         }
//     }
// }

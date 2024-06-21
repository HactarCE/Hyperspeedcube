use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::gui::components::PlaintextYamlEditor;
use crate::gui::ext::ResponseExt;
use crate::gui::util::{body_text_format, strong_text_format};
use crate::preferences::{Preferences, Preset, WithPresets, DEFAULT_PREFS};

use super::{HintWidget, PrefsUi, BIG_ICON_BUTTON_SIZE, SMALL_ICON_BUTTON_SIZE};

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
    fn popup_id_edit(&self, preset_name: &str) -> egui::Id {
        self.id.with(("edit", preset_name))
    }
    fn popup_id_new(&self) -> egui::Id {
        self.id.with("new")
    }
    fn show_preset_name_widget(
        &self,
        ui: &mut egui::Ui,
        is_first_frame: bool,
        first_frame_value: &str,
    ) -> egui::InnerResponse<String> {
        let id = self.id.with("new_name");
        let mut s = match is_first_frame {
            true => first_frame_value.to_string(),
            false => ui.data(|data| data.get_temp(id).unwrap_or_default()),
        };
        let mut r = egui::TextEdit::singleline(&mut s)
            .hint_text("New preset name")
            .desired_width(120.0)
            .show(ui);
        if is_first_frame {
            // Focus the textbox
            r.response.request_focus();

            // Select everything in the textbox
            r.state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(s.len()),
                )));
            r.state.store(ui.ctx(), r.response.id);
        }
        ui.data_mut(|data| data.insert_temp(id, s.clone()));
        egui::InnerResponse::new(s, r.response)
    }

    fn set_drag_state(&self, ui: &egui::Ui, name: Option<String>) {
        let id = self.id.with("drag");
        match name {
            Some(name) => {
                ui.data_mut(|data| data.insert_temp::<String>(id, name));
            }
            None => {
                ui.data_mut(|data| data.remove_temp::<String>(id));
            }
        }
    }
    fn get_drag_state(&self, ui: &egui::Ui) -> Option<String> {
        let id = self.id.with("drag");
        ui.data(|data| data.get_temp::<String>(id))
    }

    pub fn show_presets_selector(
        &mut self,
        ui: &mut egui::Ui,
        add_extra_heading: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let mut preset_to_activate = None;
        let mut is_first_frame_of_popup = false;

        let drag_start = self.get_drag_state(ui);
        let end_drag_this_frame = !ui.input(|input| input.pointer.is_decidedly_dragging());
        if end_drag_this_frame {
            // Reset drag state.
            self.set_drag_state(ui, None);
        }
        let mut drag_end = None;

        // Presets selector.
        let r = ui.group(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.strong("Saved presets");
                add_extra_heading(ui);
                HintWidget::show(ui, show_presets_help_ui);
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                for preset in &self.presets.presets {
                    let is_current = preset.name == self.presets.last_loaded;

                    let max_width = ui.available_width() - ui.spacing().button_padding.x * 2.0;
                    let elided_preset_name = elide_overflowing_line(ui, &preset.name, max_width);

                    let mut r = ui
                        .selectable_label(is_current, &elided_preset_name)
                        .interact(egui::Sense::drag());

                    if elided_preset_name != preset.name {
                        r = r.on_hover_text(&preset.name);
                    }

                    // Left click -> Activate preset
                    if r.clicked() {
                        preset_to_activate = Some(preset.name.clone());
                        *self.changed = true;
                    }

                    // Right click -> Edit preset
                    if r.secondary_clicked() {
                        ui.memory_mut(|mem| mem.toggle_popup(self.popup_id_edit(&preset.name)));
                        is_first_frame_of_popup = true;
                    }

                    // Drag -> Reorder preset
                    if r.drag_started() {
                        self.set_drag_state(ui, Some(preset.name.clone()));
                    }
                    if drag_start.is_some() && r.contains_pointer() {
                        ui.painter_at(r.rect).rect(
                            r.rect,
                            egui::Rounding::same(3.0),
                            ui.visuals().selection.bg_fill.linear_multiply(0.75),
                            ui.visuals().selection.stroke,
                        );
                        if end_drag_this_frame {
                            drag_end = Some(preset.name.clone());
                        }
                    }
                }

                let r = ui
                    .add(egui::Button::new("+").min_size(SMALL_ICON_BUTTON_SIZE))
                    .on_hover_text("Add preset");

                // Left click -> New preset
                if r.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(self.popup_id_new()));
                    is_first_frame_of_popup = true;
                }
            });
        });

        // Activate the new preset.
        if let Some(preset_to_activate) = preset_to_activate {
            self.presets.load_preset(&preset_to_activate);
            *self.changed = true;
        }

        // Reorder the presets.
        if let (Some(from), Some(to)) = (drag_start, drag_end) {
            self.presets.reorder(&from, &to);
        }

        let preset_names = self
            .presets
            .presets
            .iter()
            .map(|p| p.name.clone())
            .collect_vec();
        for preset_name in preset_names {
            let id = self.popup_id_edit(&preset_name);
            fake_popup(ui, id, is_first_frame_of_popup, r.response.rect, |ui| {
                ui.strong("Rename preset");

                let r = self.show_preset_name_widget(ui, is_first_frame_of_popup, &preset_name);
                let new_name = r.inner;
                let is_name_valid = !new_name.is_empty() && !self.presets.has(&new_name);

                ui.add_enabled_ui(is_name_valid, |ui| {
                    let r = ui
                        .add(egui::Button::new("✔").min_size(BIG_ICON_BUTTON_SIZE))
                        .on_hover_text("Rename preset")
                        .on_disabled_hover_text(if new_name.is_empty() {
                            "Name cannot be empty"
                        } else {
                            "There is already a preset with this name"
                        });
                    let wants_confirm =
                        r.clicked() || ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if is_name_valid && wants_confirm {
                        self.presets.rename(&preset_name, &new_name);
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                });

                let r = ui
                    .add(egui::Button::new("🗑").min_size(BIG_ICON_BUTTON_SIZE))
                    .on_hover_text("Delete preset");
                if r.clicked() {
                    self.presets.delete(&preset_name);
                    ui.memory_mut(|mem| mem.close_popup());
                }
            });
        }

        let id = self.popup_id_new();
        fake_popup(ui, id, is_first_frame_of_popup, r.response.rect, |ui| {
            ui.strong("Add preset");

            let r = self.show_preset_name_widget(ui, is_first_frame_of_popup, "");
            let new_name = r.inner;
            let is_name_valid = !new_name.is_empty() && !self.presets.has(&new_name);

            ui.add_enabled_ui(is_name_valid, |ui| {
                let r = ui
                    .add(egui::Button::new("✔").min_size(BIG_ICON_BUTTON_SIZE))
                    .on_hover_text("Add preset")
                    .on_disabled_hover_text(if new_name.is_empty() {
                        "Name cannot be empty"
                    } else {
                        "There is already a preset with this name"
                    });
                let wants_confirm =
                    r.clicked() || ui.input(|input| input.key_pressed(egui::Key::Enter));
                if is_name_valid && wants_confirm {
                    self.presets.add_preset(new_name.clone());
                    ui.memory_mut(|mem| mem.close_popup());
                }
            });
        });

        r.response
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

        let yaml = PlaintextYamlEditor::<T>::get(self.id);

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
                    let r = ui
                        .add_sized(
                            BIG_ICON_BUTTON_SIZE,
                            egui::SelectableLabel::new(yaml.is_open(ui), "✏"),
                        )
                        .on_hover_explanation(
                            "Edit as plaintext",
                            "View and edit settings as plaintext to share them with others",
                        );
                    if r.clicked() {
                        match yaml.is_open(ui) {
                            true => yaml.close(ui),
                            false => yaml.open(ui, current),
                        }
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
            egui::ScrollArea::both()
                .id_source(self.id.with(yaml.is_open(ui)))
                .auto_shrink(false)
                .show(ui, |ui| match yaml.is_open(ui) {
                    true => {
                        if let Some(r) = yaml.show(ui) {
                            if r.changed() {
                                // Update value from YAML editor.
                                if let Some(Ok(deserialized)) = yaml.deserialize(ui) {
                                    *current = deserialized;
                                    *self.changed |= r.changed();
                                }
                            }
                        }
                    }
                    false => {
                        add_contents(PrefsUi {
                            ui,
                            current,
                            defaults,
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

fn fake_popup<R>(
    ui: &mut egui::Ui,
    id: egui::Id,
    is_first_frame: bool,
    below_rect: egui::Rect,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<egui::InnerResponse<R>> {
    if ui.memory(|mem| mem.is_popup_open(id)) {
        let area_resp = egui::Area::new(unique_id!())
            .order(egui::Order::Foreground)
            .fixed_pos(below_rect.left_bottom())
            .constrain_to(ui.ctx().available_rect())
            .sense(egui::Sense::hover())
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_height(BIG_ICON_BUTTON_SIZE.y);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        add_contents(ui)
                    })
                    .inner
                })
            });
        if area_resp.response.clicked_elsewhere() && !is_first_frame
            || ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            ui.memory_mut(|mem| mem.close_popup());
        }
        Some(area_resp.inner)
    } else {
        None
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

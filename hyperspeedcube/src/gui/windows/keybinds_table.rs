use itertools::Itertools;
use std::borrow::Cow;
use strum::IntoEnumIterator;

use crate::app::App;
use crate::commands::{
    Command, FilterMode, LayerMaskDesc, PuzzleCommand, PARTIAL_SCRAMBLE_MOVE_COUNT_MAX,
    PARTIAL_SCRAMBLE_MOVE_COUNT_MIN,
};
use crate::gui::key_combo_popup;
use crate::gui::keybind_set_accessors::*;
use crate::gui::util::{self, ComboBoxExt, FancyComboBox, ResponseExt};
use crate::gui::widgets;
use crate::preferences::{Keybind, KeybindSet, Preferences};
use crate::puzzle::*;

const KEY_BUTTON_SIZE: egui::Vec2 = egui::vec2(200.0, 22.0);
const LAYER_DESCRIPTION_WIDTH: f32 = 50.0;

pub(super) struct PresetsList<'a> {
    pub app: &'a mut App,
}
impl egui::Widget for PresetsList<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.scope(|ui| {
            let puzzle_keybinds = &mut self.app.prefs.puzzle_keybinds[self.app.puzzle.ty()];

            let mut changed = false;

            let mut presets_ui = widgets::PresetsUi {
                id: unique_id!(),
                presets: &mut puzzle_keybinds.sets,
                changed: &mut changed,
                strings: widgets::PresetsUiStrings {
                    edit: "Edit keybind sets",
                    save: "Add new keybind set",
                    name: "Keybind set name",
                },
                enable_yaml: false,
            };

            presets_ui.show_header_with_active_preset(ui, KeybindSet::default, |new_preset| {
                puzzle_keybinds.active = new_preset.preset_name.clone();
            });
            ui.separator();
            presets_ui.show_list(ui, |ui, _idx, set| {
                let mut changed = false;

                let mut r = ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown)
                        .with_cross_align(egui::Align::LEFT),
                    |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut puzzle_keybinds.active,
                                set.preset_name.clone(),
                                &set.preset_name,
                            )
                            .changed();

                        // // Highlight name of active keybind set.
                        // if puzzle_keybinds.active == set.preset_name {
                        //     let visuals = ui.visuals_mut();
                        //     visuals.widgets.hovered = visuals.widgets.active;
                        //     visuals.widgets.inactive = visuals.widgets.active;
                        // }

                        // if ui
                        //     .add(egui::Button::new(&set.preset_name).frame(false))
                        //     .clicked()
                        // {
                        //     changed = true;
                        //     puzzle_keybinds.active = set.preset_name.clone();
                        // }
                    },
                );

                if changed {
                    r.response.mark_changed();
                }
                r.response
            });

            // If the active set was deleted, then pick a new active set.
            if puzzle_keybinds.get(&puzzle_keybinds.active).is_none() {
                if let Some(set) = puzzle_keybinds.sets.first() {
                    puzzle_keybinds.active = set.preset_name.clone();
                }
            }
        })
        .response
    }
}

pub(super) struct IncludePresetsList<'a> {
    pub app: &'a mut App,
}
impl egui::Widget for IncludePresetsList<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let puzzle_keybinds = &mut self.app.prefs.puzzle_keybinds[self.app.puzzle.ty()];
        let other_sets = puzzle_keybinds
            .sets
            .iter()
            .map(|set| set.preset_name.clone())
            .filter(|name| *name != puzzle_keybinds.active)
            .collect_vec();
        let active = puzzle_keybinds.active.clone();
        let includes = &mut puzzle_keybinds.get_mut(&active).value.includes;

        let mut r = ui
            .scope(|ui| {
                for set_name in other_sets {
                    let mut b = includes.contains(&set_name);
                    if ui.checkbox(&mut b, &set_name).clicked() {
                        changed = true;
                        if b {
                            includes.insert(set_name);
                        } else {
                            includes.remove(&set_name);
                        }
                    }
                }
            })
            .response;

        if changed {
            r.mark_changed();
        }
        r
    }
}

pub(super) struct KeybindsTable<'a, S> {
    app: &'a mut App,
    keybind_set: S,
}
impl<'a, S> KeybindsTable<'a, S> {
    pub(super) fn new(app: &'a mut App, keybind_set: S) -> Self {
        Self { app, keybind_set }
    }
}
impl<S: KeybindSetAccessor> egui::Widget for KeybindsTable<'_, S>
where
    for<'a> CommandSelectWidget<'a, S>: egui::Widget,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut keybinds = std::mem::take(self.keybind_set.get_mut(&mut self.app.prefs));

        let yaml_editor = widgets::PlaintextYamlEditor {
            id: unique_id!(&self.keybind_set),
        };

        let mut r = yaml_editor.show(ui, &mut keybinds).unwrap_or_else(|| {
            ui.scope(|ui| {
                ui.horizontal(|ui| {
                    if widgets::big_icon_button(ui, "✏", "Edit as plaintext").clicked() {
                        yaml_editor.set_active(ui, &keybinds);
                    }

                    if widgets::big_icon_button(ui, "➕", "Add a new keybind").clicked() {
                        keybinds.push(Keybind::default());
                        changed = true;
                    };

                    ui.allocate_ui_with_layout(
                        KEY_BUTTON_SIZE,
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| ui.strong("Keybind"),
                    );

                    ui.strong("Command");
                });

                ui.separator();

                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    let id = unique_id!(&self.keybind_set);
                    let r = widgets::ReorderableList::new(id, &mut keybinds).show(
                        ui,
                        |ui, idx, keybind| {
                            let mut r = ui.add_sized(
                                KEY_BUTTON_SIZE,
                                egui::Button::new(keybind.key.to_string()),
                            );
                            if r.clicked() {
                                key_combo_popup::open(
                                    ui.ctx(),
                                    Some(keybind.key),
                                    self.keybind_set.clone(),
                                    idx,
                                )
                            }

                            r |= ui.add(CommandSelectWidget {
                                cmd: &mut keybind.command,

                                keybind_set: &self.keybind_set,
                                idx,

                                prefs: &self.app.prefs,
                            });

                            ui.allocate_space(egui::vec2(ui.available_width(), 0.0));

                            r
                        },
                    );
                    changed |= r.changed();

                    ui.allocate_space(egui::vec2(1.0, 200.0));
                });

                if ui.available_height() > 0.0 {
                    ui.allocate_space(ui.available_size());
                }
            })
            .response
        });

        *self.keybind_set.get_mut(&mut self.app.prefs) = keybinds;

        if changed {
            r.mark_changed();
        }
        r
    }
}

struct CommandSelectWidget<'a, S: KeybindSetAccessor> {
    cmd: &'a mut S::Command,

    keybind_set: &'a S,
    idx: usize,

    prefs: &'a Preferences,
}

impl egui::Widget for CommandSelectWidget<'_, GlobalKeybindsAccessor> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use Command as Cmd;

        let mut changed = false;

        let mut r = ui.scope(|ui| {
            let r = enum_combobox!(
                ui,
                unique_id!(self.idx),
                match (self.cmd) {
                    "None" => Cmd::None,

                    "Open..." => Cmd::Open,
                    "Save" => Cmd::Save,
                    "Save as..." => Cmd::SaveAs,
                    "Exit" => Cmd::Exit,

                    "Undo" => Cmd::Undo,
                    "Redo" => Cmd::Redo,
                    "Reset" => Cmd::Reset,

                    "Scramble partially" => Cmd::ScrambleN(PARTIAL_SCRAMBLE_MOVE_COUNT_MIN),
                    "Scramble fully" => Cmd::ScrambleFull,
                    "Toggle blindfold" => Cmd::ToggleBlindfold,
                    "New puzzle" => Cmd::NewPuzzle(PuzzleTypeEnum::default()),
                }
            );
            changed |= r.changed();

            match self.cmd {
                Cmd::ScrambleN(n) => {
                    let r = ui.add(egui::DragValue::new(n).clamp_range(
                        PARTIAL_SCRAMBLE_MOVE_COUNT_MIN..=PARTIAL_SCRAMBLE_MOVE_COUNT_MAX,
                    ));
                    changed |= r.changed();
                }

                Cmd::NewPuzzle(puzzle_type) => {
                    if let Some(Some(ty)) = ui
                        .menu_button(puzzle_type.name(), util::puzzle_select_menu)
                        .inner
                    {
                        *puzzle_type = ty;
                        changed |= true;
                    }
                }

                _ => (),
            }
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

impl egui::Widget for CommandSelectWidget<'_, PuzzleKeybindsAccessor> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use PuzzleCommand as Cmd;

        let puzzle_type = self.keybind_set.puzzle_type;

        let mut changed = false;

        let mut r = ui.scope(|ui| {
            let r = enum_combobox!(
                ui,
                unique_id!(self.idx),
                match (self.cmd) {
                    "None" => Cmd::None,

                    "Grip" => Cmd::Grip {
                        axis: self.cmd.axis_mut().cloned().unwrap_or_default(),
                        layers: self.cmd.layers_mut().cloned().unwrap_or_default(),
                    },
                    "Twist" => Cmd::Twist {
                        axis: self.cmd.axis_mut().cloned().unwrap_or_default(),
                        direction: self.cmd.direction_mut().cloned().unwrap_or_else(|| {
                            puzzle_type.twist_directions()[0].name.to_owned()
                        }),
                        layers: self.cmd.layers_mut().cloned().unwrap_or_default(),
                    },
                    "Recenter" => Cmd::Recenter {
                        axis: self.cmd.axis_mut().cloned().unwrap_or_default(),
                    },

                    "Filter" => Cmd::Filter {
                        mode: self.cmd.filter_mode_mut().cloned().unwrap_or_default(),
                        filter_name: self.cmd.filter_name_mut().cloned().unwrap_or_default(),
                    },

                    "Keybind set" => Cmd::KeybindSet {
                        keybind_set_name: self
                            .cmd
                            .keybind_set_name_mut()
                            .cloned()
                            .unwrap_or_default(),
                    },
                }
            );
            changed |= r.changed();

            if let Some(layers) = self.cmd.layers_mut() {
                let r = ui.add(LayerMaskEdit {
                    id: unique_id!(self.idx),
                    layers,
                });
                changed |= r.changed();
            }
            if let Some(axis) = self.cmd.axis_mut() {
                let r = ui.add(FancyComboBox::new_optional(
                    unique_id!(self.idx),
                    axis,
                    puzzle_type.twist_axes(),
                ));
                changed |= r.changed();
            }
            if let Some(direction) = self.cmd.direction_mut() {
                let r = ui.add(FancyComboBox::new(
                    unique_id!(self.idx),
                    direction,
                    puzzle_type.twist_directions(),
                ));
                changed |= r.changed();
            }
            if let Some(filter_mode) = self.cmd.filter_mode_mut() {
                let r = ui.add(FancyComboBox {
                    combo_box: egui::ComboBox::from_id_source(unique_id!(self.idx)),
                    selected: filter_mode,
                    options: FilterMode::iter()
                        .map(|mode| (mode, Cow::Borrowed(mode.into())))
                        .collect(),
                });
                changed |= r.changed();
            }
            if let Some(filter_name) = self.cmd.filter_name_mut() {
                let preset_names = self.prefs.piece_filters[puzzle_type]
                    .iter()
                    .map(|preset| &preset.preset_name);
                let r = ui
                    .add(FancyComboBox::new(
                        unique_id!(self.idx),
                        filter_name,
                        std::iter::once(&"Everything".to_string()).chain(preset_names),
                    ))
                    .on_hover_explanation(
                        "",
                        "You can manage piece filter presets \
                         in the \"Piece filters\" tool.",
                    );
                changed |= r.changed();
            }
            if let Some(keybind_set_name) = self.cmd.keybind_set_name_mut() {
                let r = ui
                    .add(FancyComboBox::new(
                        unique_id!(self.idx),
                        keybind_set_name,
                        self.prefs.puzzle_keybinds[puzzle_type]
                            .sets
                            .iter()
                            .map(|set| &set.preset_name),
                    ))
                    .on_hover_explanation(
                        "",
                        "You can manage keybind sets in Settings ➡ Keybind sets.",
                    );
                changed |= r.changed();
            }
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

struct LayerMaskEdit<'a> {
    id: egui::Id,
    layers: &'a mut LayerMaskDesc,
}
impl<'a> egui::Widget for LayerMaskEdit<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;
        let mut r = ui
            .scope(|ui| {
                let text_id = self.id.with("layer_text");

                let default_string = format!("{{{}}}", self.layers);

                let mut text: String = ui
                    .data()
                    .get_temp(text_id)
                    .unwrap_or_else(|| default_string.clone());

                let r = egui::TextEdit::singleline(&mut text)
                    .desired_width(LAYER_DESCRIPTION_WIDTH)
                    .show(ui)
                    .response;

                if r.changed() {
                    // Try to parse the new layer mask string.
                    *self.layers = text
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .parse()
                        .unwrap_or_default();
                    changed = true;
                } else if !r.has_focus() {
                    text = default_string;
                }

                r.on_hover_explanation(
                    "Layer mask string",
                    "Comma-separated list of layers or layer ranges, such as '1..3'. \
                     Negative numbers count from the other side of the puzzle. \
                     Exclamation mark prefix excludes a range.\n\
                     \n\
                     Examples:\n\
                     • {1} = outer layer\n\
                     • {2} = next layer in\n\
                     • {1,-1} = outer layer on either side\n\
                     • {1..3} = three outer layers\n\
                     • {1..-1} = whole puzzle\n\
                     • {1..-1,!3} = all except layer 3",
                );

                ui.data().insert_temp(text_id, text);
            })
            .response;
        if changed {
            r.mark_changed();
        }
        r
    }
}

use crate::app::App;
use crate::commands::{
    Command, LayerMaskDesc, PuzzleCommand, PARTIAL_SCRAMBLE_MOVE_COUNT_MAX,
    PARTIAL_SCRAMBLE_MOVE_COUNT_MIN,
};
use crate::gui::key_combo_popup;
use crate::gui::keybinds_set::*;
use crate::gui::util::{self, ComboBoxExt, FancyComboBox, PlaintextYamlEditor, ResponseExt};
use crate::gui::widgets;
use crate::preferences::Keybind;
use crate::puzzle::*;

const SQUARE_BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 22.0);
const KEY_BUTTON_SIZE: egui::Vec2 = egui::vec2(200.0, 22.0);
const LAYER_DESCRIPTION_WIDTH: f32 = 50.0;

pub(super) struct KeybindsTable<'a, S> {
    app: &'a mut App,
    keybind_set: S,
}
impl<'a, S> KeybindsTable<'a, S> {
    pub(super) fn new(app: &'a mut App, keybind_set: S) -> Self {
        Self { app, keybind_set }
    }
}
impl<S: KeybindSet> egui::Widget for KeybindsTable<'_, S>
where
    for<'a> CommandSelectWidget<'a, S>: egui::Widget,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let keybinds = self.keybind_set.get_mut(&mut self.app.prefs);

        fn square_button(ui: &mut egui::Ui, text: &str, hover_text: &str) -> egui::Response {
            ui.add_sized(SQUARE_BUTTON_SIZE, egui::Button::new(text))
                .on_hover_text(hover_text)
        }

        let plaintext_yaml_editor = PlaintextYamlEditor {
            id: unique_id!(self.keybind_set),
            button_size: SQUARE_BUTTON_SIZE,
        };

        let mut r = plaintext_yaml_editor.show(ui, keybinds).unwrap_or_else(|| {
            ui.scope(|ui| {
                ui.horizontal(|ui| {
                    if square_button(ui, "✏", "Edit as plaintext").clicked() {
                        plaintext_yaml_editor.set_active(ui, keybinds);
                    }

                    if square_button(ui, "➕", "Add a new keybind").clicked() {
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
                    let r = widgets::ReorderableList::new(unique_id!(self.keybind_set), keybinds)
                        .button_size(SQUARE_BUTTON_SIZE)
                        .show(ui, |ui, idx, keybind| {
                            let mut r = ui.add_sized(
                                KEY_BUTTON_SIZE,
                                egui::Button::new(keybind.key.to_string()),
                            );
                            if r.clicked() {
                                key_combo_popup::open(
                                    ui.ctx(),
                                    Some(keybind.key),
                                    self.keybind_set,
                                    idx,
                                )
                            }

                            r |= ui.add(CommandSelectWidget {
                                cmd: &mut keybind.command,

                                keybind_set: self.keybind_set,
                                idx,
                            });

                            ui.allocate_space(egui::vec2(ui.available_width(), 0.0));

                            r
                        });
                    changed |= r.changed();

                    ui.allocate_space(egui::vec2(1.0, 200.0));
                });

                if ui.available_height() > 0.0 {
                    ui.allocate_space(ui.available_size());
                }
            })
            .response
        });

        if changed {
            r.mark_changed();
        }
        r
    }
}

struct CommandSelectWidget<'a, S: KeybindSet> {
    cmd: &'a mut S::Command,

    keybind_set: S,
    idx: usize,
}

impl egui::Widget for CommandSelectWidget<'_, GlobalKeybinds> {
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

impl egui::Widget for CommandSelectWidget<'_, PuzzleKeybinds> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use PuzzleCommand as Cmd;

        let puzzle_type = self.keybind_set.0;

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
                        axis: self.cmd.axis_mut().cloned().unwrap_or_default()
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
                    .unwrap_or(default_string.clone());

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

use itertools::Itertools;
use std::hash::Hash;
use strum::IntoEnumIterator;

use super::util::{self, ComboBoxExt};
use crate::commands::{Command, PuzzleCommand, SelectHow, SelectThing};
use crate::gui::util::BasicComboBox;
use crate::preferences::Keybind;
use crate::puzzle::{LayerMask, PieceType, PuzzleType, PuzzleTypeTrait, TwistDirection};

macro_rules! unique_id {
    ($($args:tt)*) => {
        (std::file!(), std::line!(), std::column!(), $($args)*)
    };
}

#[derive(Debug, Copy, Clone)]
struct DragData {
    from: usize,
    to: usize,
}

const SQUARE_BUTTON_SIZE: egui::Vec2 = egui::vec2(24.0, 24.0);
const KEY_BUTTON_SIZE: egui::Vec2 = egui::vec2(200.0, 22.0);

pub(super) fn keybinds_window_id(title: &str) -> egui::Id {
    egui::Id::new(unique_id!()).with(title)
}

pub(super) fn build<T, C>(
    ctx: &egui::Context,
    title: &str,
    keybinds: &mut Vec<Keybind<T>>,
    keybinds_context: C,
    needs_save: &mut bool,
) where
    for<'a> KeybindsTable<'a, T, C>: egui::Widget,
{
    let id = keybinds_window_id(title);
    let mut open = ctx.data().get_persisted(id).unwrap_or(false);
    egui::Window::new(title).open(&mut open).show(ctx, |ui| {
        *needs_save |= ui
            .add(KeybindsTable {
                keybinds,
                context: keybinds_context,
            })
            .changed();
    });
    ctx.data().insert_persisted(id, open);
}

pub(super) struct KeybindsTable<'a, T, C = ()> {
    pub keybinds: &'a mut Vec<Keybind<T>>,
    pub context: C,
}
impl<T, C> egui::Widget for KeybindsTable<'_, T, C>
where
    T: Default,
    C: Copy,
    for<'a> CommandSelectWidget<'a, T, C>: egui::Widget,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut r = ui.scope(|ui| {
            ui.horizontal(|ui| {
                ui.allocate_exact_size(SQUARE_BUTTON_SIZE, egui::Sense::hover());

                let r = ui
                    .add_sized(SQUARE_BUTTON_SIZE, egui::Button::new("➕"))
                    .on_hover_text("Add a new keybind");
                if r.clicked() {
                    self.keybinds.push(Keybind::default());
                    changed = true;
                };

                ui.allocate_ui_with_layout(
                    KEY_BUTTON_SIZE,
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.label(egui::RichText::new("Keybind").strong());
                    },
                );

                ui.label(egui::RichText::new("Command").strong());
            });

            ui.separator();

            egui::ScrollArea::new([false, true]).show(ui, |ui| {
                let drag_id = ui.make_persistent_id("drag");
                let mut drag_data = ui.data().get_temp::<DragData>(drag_id);
                if !ui.memory().is_anything_being_dragged() {
                    drag_data = None;
                }

                let mut reorder_responses = vec![];
                let mut delete_idx = None;

                for (i, keybind) in self.keybinds.iter_mut().enumerate() {
                    let is_being_dragged = drag_data.map_or(false, |drag| drag.from == i);

                    ui.horizontal(|ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(SQUARE_BUTTON_SIZE, egui::Sense::drag());
                        if ui.is_rect_visible(rect) {
                            let color = if resp.has_focus() || is_being_dragged {
                                ui.visuals().strong_text_color()
                            } else if resp.hovered() {
                                ui.visuals().text_color()
                            } else {
                                ui.visuals().weak_text_color()
                            };

                            for dy in [-6.0, 0.0, 6.0] {
                                for dx in [-3.0, 3.0] {
                                    const RADIUS: f32 = 1.5;
                                    let pos = rect.center() + egui::vec2(dx, dy);
                                    ui.painter().circle_filled(pos, RADIUS, color);
                                }
                            }
                        }

                        reorder_responses.push(resp);

                        if ui
                            .add_sized(SQUARE_BUTTON_SIZE, egui::Button::new("🗑"))
                            .clicked()
                        {
                            delete_idx = Some(i);
                        }

                        if ui
                            .add_sized(KEY_BUTTON_SIZE, egui::Button::new(keybind.key.to_string()))
                            .clicked()
                        {
                            println!("TODO keybind popup");
                        }

                        let r = ui.add(CommandSelectWidget {
                            idx: i,
                            cmd: &mut keybind.command,
                            context: self.context,
                        });
                        changed |= r.changed();

                        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
                    });
                }

                if reorder_responses.iter().any(|r| r.hovered()) {
                    ui.output().cursor_icon = egui::CursorIcon::Grab;
                }
                if let Some(i) = reorder_responses.iter().position(|r| r.has_focus()) {
                    let up = ui.input().num_presses(egui::Key::ArrowUp);
                    let down = ui.input().num_presses(egui::Key::ArrowDown);
                    let to = (i + down).saturating_sub(up);
                    reorder_responses[to].request_focus();
                    drag_data = Some(DragData { from: i, to });
                } else if ui.memory().is_anything_being_dragged() {
                    if let Some(i) = reorder_responses.iter().position(|r| r.drag_started()) {
                        drag_data = Some(DragData { from: i, to: i });
                    }
                    if let Some(DragData { from: _, to }) = &mut drag_data {
                        ui.output().cursor_icon = egui::CursorIcon::Grabbing;

                        if let Some(egui::Pos2 { y, .. }) = ui.ctx().pointer_interact_pos() {
                            while *to > 0 && y < reorder_responses[*to - 1].rect.bottom() {
                                *to -= 1;
                            }
                            while *to + 1 < reorder_responses.len()
                                && y > reorder_responses[*to + 1].rect.top()
                            {
                                *to += 1
                            }
                        }
                    }
                }

                if let Some(DragData { mut from, to }) = drag_data {
                    // do those swaps
                    while from < to {
                        self.keybinds.swap(from, from + 1);
                        from += 1;
                        changed = true;
                    }
                    while to < from {
                        self.keybinds.swap(from, from - 1);
                        from -= 1;
                        changed = true;
                    }
                    ui.data().insert_temp(drag_id, DragData { from, to });
                } else {
                    ui.data().remove::<DragData>(drag_id);
                }

                if let Some(i) = delete_idx {
                    self.keybinds.remove(i);
                    changed = true;
                }
            });

            if ui.available_height() > 0.0 {
                ui.allocate_space(ui.available_size());
            }
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

struct CommandSelectWidget<'a, T, C = ()> {
    idx: usize,
    cmd: &'a mut T,
    context: C,
}

impl egui::Widget for CommandSelectWidget<'_, Command> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use Command as Cmd;

        let mut changed = false;

        let mut r = ui.scope(|ui| {
            #[derive(Debug, Display, EnumIter, Copy, Clone, PartialEq, Eq)]
            enum CmdType {
                None,
                #[strum(serialize = "Open...")]
                Open,
                Save,
                #[strum(serialize = "Save As...")]
                SaveAs,
                Exit,
                Undo,
                Redo,
                Reset,
                #[strum(serialize = "New puzzle")]
                NewPuzzle,
            }

            let mut cmd_type = match self.cmd {
                Cmd::Open => CmdType::Open,
                Cmd::Save => CmdType::Save,
                Cmd::SaveAs => CmdType::SaveAs,
                Cmd::Exit => CmdType::Exit,

                Cmd::Undo => CmdType::Undo,
                Cmd::Redo => CmdType::Redo,
                Cmd::Reset => CmdType::Reset,

                Cmd::NewPuzzle(_) => CmdType::NewPuzzle,

                Cmd::None => CmdType::None,
            };
            let old_cmd_type = cmd_type;

            egui::ComboBox::from_id_source(unique_id!(self.idx))
                .selected_text(cmd_type.to_string())
                .show_ui(ui, |ui| {
                    for option in CmdType::iter() {
                        changed |= ui
                            .selectable_value(&mut cmd_type, option, option.to_string())
                            .changed();
                    }
                });
            if cmd_type != old_cmd_type {
                *self.cmd = match cmd_type {
                    CmdType::None => Cmd::None,

                    CmdType::Open => Cmd::Open,
                    CmdType::Save => Cmd::Save,
                    CmdType::SaveAs => Cmd::SaveAs,
                    CmdType::Exit => Cmd::Exit,

                    CmdType::Undo => Cmd::Undo,
                    CmdType::Redo => Cmd::Redo,
                    CmdType::Reset => Cmd::Reset,

                    CmdType::NewPuzzle => Cmd::NewPuzzle(self.cmd.get_puzzle_type()),
                }
            }

            match self.cmd {
                Cmd::NewPuzzle(puzzle_type) => {
                    add_pre_label_space(ui);
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        egui::ComboBox::from_id_source(unique_id!(self.idx))
                            .selected_text(puzzle_type.name())
                            .show_ui(ui, |ui| {
                                for option in PuzzleType::iter() {
                                    changed |= ui
                                        .selectable_value(puzzle_type, option, option.name())
                                        .changed();
                                }
                            });
                    });
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

impl egui::Widget for CommandSelectWidget<'_, PuzzleCommand, PuzzleType> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use PuzzleCommand as Cmd;

        let puzzle_type = self.context;

        let mut changed = false;

        let mut r = ui.scope(|ui| {
            #[derive(Display, EnumIter, Copy, Clone, PartialEq, Eq)]
            enum CmdType {
                None,
                Twist,
                Recenter,
                #[strum(serialize = "Select")]
                HoldSelect,
                #[strum(serialize = "Toggle select")]
                ToggleSelect,
                #[strum(serialize = "Clear selected")]
                ClearToggleSelect,
            }

            let mut cmd_type = match self.cmd {
                Cmd::Twist { .. } => CmdType::Twist,
                Cmd::Recenter { .. } => CmdType::Recenter,

                Cmd::HoldSelect(_) => CmdType::HoldSelect,
                Cmd::ToggleSelect(_) => CmdType::ToggleSelect,
                Cmd::ClearToggleSelect(_) => CmdType::ClearToggleSelect,

                Cmd::None => CmdType::None,
            };

            let default_direction = TwistDirection::default(puzzle_type);

            let r = ui.add(util::BasicComboBox::new_enum(
                unique_id!(self.idx),
                &mut cmd_type,
            ));
            changed |= r.changed();
            if r.changed() {
                *self.cmd = match cmd_type {
                    CmdType::None => Cmd::None,

                    CmdType::Twist => Cmd::Twist {
                        face: None,
                        direction: default_direction,
                        layer_mask: LayerMask(1),
                    },
                    CmdType::Recenter => Cmd::Recenter { face: None },

                    CmdType::HoldSelect => Cmd::HoldSelect(self.cmd.get_select_thing(puzzle_type)),
                    CmdType::ToggleSelect => {
                        Cmd::ToggleSelect(self.cmd.get_select_thing(puzzle_type))
                    }
                    CmdType::ClearToggleSelect => {
                        Cmd::ClearToggleSelect(self.cmd.get_select_category())
                    }
                }
            }

            if let Some(select_how) = self.cmd.get_select_how() {
                let mut category = self.cmd.get_select_category();
                let r = ui.add(util::BasicComboBox::new_enum(
                    unique_id!(self.idx),
                    &mut category,
                ));
                changed |= r.changed();
                if r.changed() {
                    *self.cmd = match select_how {
                        SelectHow::Hold => {
                            Cmd::HoldSelect(SelectThing::default(category, puzzle_type))
                        }
                        SelectHow::Toggle => {
                            Cmd::ToggleSelect(SelectThing::default(category, puzzle_type))
                        }
                        SelectHow::Clear => Cmd::ClearToggleSelect(category),
                    };
                }
            }

            match self.cmd {
                Cmd::None => (),

                Cmd::Twist {
                    face,
                    direction,
                    layer_mask,
                } => {
                    add_pre_label_space(ui);
                    ui.label("Face:");
                    let r = ui.add(OptionalComboBox::new(
                        unique_id!(self.idx),
                        face,
                        puzzle_type.faces(),
                    ));
                    changed |= r.changed();

                    add_pre_label_space(ui);
                    ui.label("Layers:");
                    let r = ui.add(LayerMaskCheckboxes {
                        layer_mask,
                        layer_count: puzzle_type.layer_count(),
                    });
                    changed |= r.changed();

                    add_pre_label_space(ui);
                    ui.label("Direction:");
                    let r = ui.add(BasicComboBox::new(
                        unique_id!(self.idx),
                        direction,
                        TwistDirection::iter(puzzle_type).collect_vec(),
                    ));
                    changed |= r.changed();
                }
                Cmd::Recenter { face } => {
                    add_pre_label_space(ui);
                    ui.label("Face:");
                    let r = ui.add(OptionalComboBox::new(
                        unique_id!(self.idx),
                        face,
                        puzzle_type.faces(),
                    ));
                    changed |= r.changed();
                }

                Cmd::HoldSelect(thing) | Cmd::ToggleSelect(thing) => match thing {
                    SelectThing::Face(face) => {
                        let r = ui.add(BasicComboBox::new(
                            unique_id!(self.idx),
                            face,
                            puzzle_type.faces(),
                        ));
                        changed |= r.changed();
                    }
                    SelectThing::Layers(layer_mask) => {
                        let r = ui.add(LayerMaskCheckboxes {
                            layer_mask,
                            layer_count: puzzle_type.layer_count(),
                        });
                        changed |= r.changed();
                    }
                    SelectThing::PieceType(piece_type) => {
                        let r = ui.add(BasicComboBox::new(
                            unique_id!(self.idx),
                            piece_type,
                            PieceType::iter(puzzle_type).collect_vec(),
                        ));
                        changed |= r.changed();
                    }
                },
                Cmd::ClearToggleSelect(_) => (),
            }
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

struct OptionalComboBox<'a, T> {
    combo_box: egui::ComboBox,
    selected: &'a mut Option<T>,
    options: Vec<Option<T>>,
}
impl<'a, T: Clone> OptionalComboBox<'a, T> {
    pub(super) fn new(id_source: impl Hash, selected: &'a mut Option<T>, options: &'a [T]) -> Self {
        Self {
            combo_box: egui::ComboBox::from_id_source(id_source),
            selected,
            options: std::iter::once(None)
                .chain(options.iter().cloned().map(Some))
                .collect(),
        }
    }
}
impl<T: Eq + ToString> egui::Widget for OptionalComboBox<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let get_string = |opt: &Option<T>| match opt {
            None => "(selected)".to_string(),
            Some(x) => x.to_string(),
        };

        let mut r = self
            .combo_box
            .selected_text(get_string(self.selected))
            .width_to_fit(ui, self.options.iter().map(get_string).collect())
            .show_ui(ui, |ui| {
                for option in self.options {
                    let is_selected = option == *self.selected;
                    if ui
                        .selectable_label(is_selected, get_string(&option))
                        .clicked()
                    {
                        *self.selected = option;
                        changed = true;
                    }
                }
            });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

struct LayerMaskCheckboxes<'a> {
    layer_mask: &'a mut LayerMask,
    layer_count: usize,
}
impl egui::Widget for LayerMaskCheckboxes<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut r = ui.scope(|ui| {
            // Checkbox size workaround
            ui.spacing_mut().interact_size.x = 0.0;
            ui.spacing_mut().button_padding.x = 0.0;

            for i in 0..self.layer_count {
                let mut flag = self.layer_mask.0 & (1 << i) != 0;

                let r = ui
                    .checkbox(&mut flag, "")
                    .on_hover_text(format!("{}", i + 1));
                if r.changed() {
                    self.layer_mask.0 ^= 1 << i;
                    changed = true;
                }
            }
        });

        if changed {
            r.response.mark_changed();
        }
        r.response
    }
}

fn add_pre_label_space(ui: &mut egui::Ui) {
    ui.add_space(ui.spacing().item_spacing.x * 2.0);
}

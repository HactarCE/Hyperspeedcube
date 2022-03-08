use strum::IntoEnumIterator;

use crate::commands::Command;
use crate::preferences::Keybind;
use crate::puzzle::{PuzzleType, PuzzleTypeTrait};

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

pub(super) struct KeybindsWindow<'a, T> {
    pub keybinds: &'a mut Vec<Keybind<T>>,
}
impl<T> egui::Widget for KeybindsWindow<'_, T>
where
    T: Default,
    for<'a> CommandSelectWidget<'a, T>: egui::Widget,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut r = ui.scope(|_| ()).response;

        ui.horizontal(|ui| {
            ui.allocate_exact_size(SQUARE_BUTTON_SIZE, egui::Sense::hover());

            let resp = ui.add_sized(SQUARE_BUTTON_SIZE, egui::Button::new("➕"));
            if resp.on_hover_text("Add a new keybind").clicked() {
                self.keybinds.push(Keybind::default());
                r.mark_changed();
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

                    // ui.with_layout(egui::Layout::left_to_right().with_main_wrap(true), |ui| {
                    let resp = ui.add(CommandSelectWidget {
                        idx: i,
                        cmd: &mut keybind.command,
                    });
                    if resp.changed() {
                        r.mark_changed();
                    }
                    // });

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
                    r.mark_changed();
                }
                while to < from {
                    self.keybinds.swap(from, from - 1);
                    from -= 1;
                    r.mark_changed();
                }
                ui.data().insert_temp(drag_id, DragData { from, to });
            } else {
                ui.data().remove::<DragData>(drag_id);
            }

            if let Some(i) = delete_idx {
                self.keybinds.remove(i);
                r.mark_changed();
            }
        });

        if ui.available_height() > 0.0 {
            ui.allocate_space(ui.available_size());
        }

        r
    }
}

struct CommandSelectWidget<'a, T> {
    idx: usize,
    cmd: &'a mut T,
}

impl egui::Widget for CommandSelectWidget<'_, Command> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use Command as Cmd;

        let mut r = ui.scope(|ui| {
            let mut changed = false;

            #[derive(AsRefStr, EnumIter, Copy, Clone, PartialEq, Eq)]
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
                .selected_text(cmd_type.as_ref())
                .show_ui(ui, |ui| {
                    for option in CmdType::iter() {
                        changed |= ui
                            .selectable_value(&mut cmd_type, option, option.as_ref())
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
                    ui.horizontal(|ui| {
                        ui.label("Type");
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

            changed
        });
        if r.response.changed() {
            println!("YAY",);
        }
        if r.inner {
            r.response.mark_changed();
        }
        r.response
    }
}

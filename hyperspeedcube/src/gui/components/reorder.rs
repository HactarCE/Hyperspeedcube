use egui::NumExt;

use crate::gui::components::{big_icon_button, BIG_ICON_BUTTON_SIZE};

pub struct ReorderableList<'a, T> {
    id: egui::Id,
    list: &'a mut Vec<T>,
}
impl<'a, T> ReorderableList<'a, T> {
    pub fn new(id: egui::Id, list: &'a mut Vec<T>) -> Self {
        Self { id, list }
    }
    pub fn show(
        self,
        ui: &mut egui::Ui,
        mut row_ui: impl FnMut(&mut egui::Ui, usize, &mut T) -> egui::Response,
    ) -> egui::Response {
        let drag_id = self.id.with("drag");
        let is_anything_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());
        let mut reorder_from: Option<usize> = ui.data(|data| {
            data.get_temp::<usize>(drag_id)
                .filter(|_| is_anything_being_dragged)
                .filter(|&i| i < self.list.len())
        });
        let mut reorder_to: Option<usize> = None;
        let mut to_delete: Option<usize> = None;

        let mut drag_handle = Vec::with_capacity(self.list.len());

        let mut changed = false;
        let mut resp = ui
            .scope(|ui| {
                for (i, elem) in self.list.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            let is_being_dragged = reorder_from == Some(i);
                            drag_handle.push(ui.add(DragReorderHandle { is_being_dragged }));

                            if big_icon_button(ui, "🗑", "").clicked() {
                                to_delete = Some(i);
                            }

                            changed |= row_ui(ui, i, elem).changed();
                        })
                    });
                }
            })
            .response;

        // Set cursor icon when hovering a reorder handle.
        if drag_handle.iter().any(|r| r.hovered()) || reorder_from.is_some() {
            ui.output_mut(|out| out.cursor_icon = egui::CursorIcon::ResizeVertical);
        }
        if let Some(from) = drag_handle.iter().position(|r| r.has_focus()) {
            // Reorder using keyboard.
            let up = ui.input(|input| input.num_presses(egui::Key::ArrowUp));
            let down = ui.input(|input| input.num_presses(egui::Key::ArrowDown));
            let to = (from + down)
                .saturating_sub(up)
                .at_most(self.list.len() - 1);
            if from != to {
                drag_handle[to].request_focus();
                reorder_from = Some(from);
                reorder_to = Some(to);
            }
        } else if ui.memory(|mem| mem.is_anything_being_dragged()) {
            // Reorder using mouse.
            if let Some(i) = drag_handle.iter().position(|r| r.drag_started()) {
                // A drag is beginning!
                reorder_from = Some(i);
            }
            if let (Some(from), Some(mouse)) = (reorder_from, ui.ctx().pointer_interact_pos()) {
                // Figure out which row we should drag to.
                let from_rect = drag_handle[from].rect;
                reorder_to = if mouse.y < from_rect.bottom() {
                    (0..from).find(|&i| mouse.y < drag_handle[i].rect.bottom())
                } else {
                    (from + 1..self.list.len()).rfind(|&i| mouse.y > drag_handle[i].rect.top())
                };
            }
        }

        // Reorder as necessary.
        if let (Some(from), Some(to)) = (reorder_from, reorder_to) {
            let to = to.at_most(self.list.len() - 1);
            if from < to {
                resp.mark_changed();
                self.list[from..=to].rotate_left(1);
            }
            if to < from {
                resp.mark_changed();
                self.list[to..=from].rotate_right(1);
            }
            reorder_from = reorder_to;
        }

        // Delete as necessary.
        if let Some(i) = to_delete {
            self.list.remove(i);
            changed = true;
        }

        match reorder_from {
            Some(from) => ui.data_mut(|data| data.insert_temp::<usize>(drag_id, from)),
            None => ui.data_mut(|data| data.remove::<usize>(drag_id)),
        }

        if changed {
            resp.mark_changed();
        }

        resp
    }
}

struct DragReorderHandle {
    is_being_dragged: bool,
}
impl egui::Widget for DragReorderHandle {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(BIG_ICON_BUTTON_SIZE, egui::Sense::drag());
        if ui.is_rect_visible(rect) {
            // Change color based on hover/focus.
            let color = if resp.has_focus() || self.is_being_dragged {
                ui.visuals().strong_text_color()
            } else if resp.hovered() {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            };

            // Draw 6 dots.
            let r = ui.spacing().button_padding.x / 2.0;
            for dy in [-2.0, 0.0, 2.0] {
                for dx in [-1.0, 1.0] {
                    const RADIUS: f32 = 1.0;
                    let pos = rect.center() + egui::vec2(dx, dy) * r;
                    ui.painter().circle_filled(pos, RADIUS, color);
                }
            }
        }
        resp
    }
}

use std::any::Any;

use crate::preferences::BeforeOrAfter;

pub struct DragAndDropResponse<Payload, End> {
    pub payload: Payload,
    pub end: End,
    pub before_or_after: Option<BeforeOrAfter>,
}

pub struct DragAndDrop<Payload, End = Payload> {
    id: egui::Id,
    ctx: egui::Context,

    /// Response containing the initial payload and where it ended up.
    response: Option<DragAndDropResponse<Payload, End>>,
}
impl<Payload, End> DragAndDrop<Payload, End>
where
    Payload: Any + Default + Clone + Send + Sync,
    End: Clone,
{
    pub fn new(ui: &mut egui::Ui) -> Self {
        let this = Self {
            id: ui.next_auto_id(),
            ctx: ui.ctx().clone(),

            response: None,
        };
        ui.skip_ahead_auto_ids(1);

        // Cancel drag if we get into a weird state somehow.
        if !ui.input(|input| input.pointer.any_down() || input.pointer.any_released()) {
            this.take_payload();
        }

        this
    }

    pub fn is_dragging(&self) -> bool {
        self.payload().is_some()
    }
    pub fn set_payload(&self, payload: Payload) {
        self.ctx
            .data_mut(|data| data.insert_temp::<Payload>(self.id, payload))
    }
    pub fn payload(&self) -> Option<Payload> {
        self.ctx.data(|data| data.get_temp::<Payload>(self.id))
    }
    pub fn take_payload(&self) -> Option<Payload> {
        self.ctx
            .data_mut(|data| data.remove_temp::<Payload>(self.id))
    }

    /// Adds a draggable widget.
    pub fn draggable(
        &self,
        ui: &mut egui::Ui,
        payload: Payload,
        add_contents: impl FnOnce(&mut egui::Ui) -> egui::Response,
    ) -> egui::Response {
        let drag_start_id = unique_id!();
        let id = ui.auto_id_with("hyperspeedcube::drag_and_drop");

        if ui.ctx().is_being_dragged(id) {
            // Paint the widget to a different layer so that we can move it
            // around independently. Highlight the widget so that it looks like
            // it's still being hovered.
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let r = ui
                .with_layer_id(layer_id, |ui| {
                    ui.set_opacity(0.4);
                    add_contents(ui)
                })
                .inner
                .highlight();

            ui.painter().rect_filled(
                r.rect,
                3.0,
                ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.1),
            );

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos
                    - ui.data(|data| data.get_temp(drag_start_id))
                        .unwrap_or_else(|| r.rect.center());
                ui.ctx().transform_layer_shapes(
                    layer_id,
                    egui::emath::TSTransform::from_translation(delta),
                );
            }

            r
        } else {
            let mut r = add_contents(ui);

            if !r.sense.click {
                r = r.on_hover_and_drag_cursor(egui::CursorIcon::Grab);
            }

            if r.drag_started() {
                ui.ctx().set_dragged_id(id);
                if let Some(pos) = r.hover_pos() {
                    ui.data_mut(|data| data.insert_temp(drag_start_id, pos))
                };
                self.set_payload(payload);
            }

            r
        }
    }

    /// Add a drop zone onto an existing widget.
    pub fn drop_zone(&mut self, ui: &mut egui::Ui, r: &egui::Response, end: End) {
        if !self.is_dragging() {
            return;
        }

        let color = ui.visuals().widgets.active.bg_stroke.color;
        let active_stroke = egui::Stroke { width: 2.0, color };

        let color = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let inactive_stroke = egui::Stroke { width: 2.0, color };

        let stroke = if r.contains_pointer() {
            active_stroke
        } else {
            inactive_stroke
        };

        ui.painter().rect_stroke(r.rect, 3.0, stroke);

        if ui.input(|input| input.pointer.any_released()) && r.contains_pointer() {
            let Some(payload) = self.take_payload() else {
                return;
            };
            self.response = Some(DragAndDropResponse {
                payload,
                end,
                before_or_after: None,
            });
        }
    }

    pub fn take_response(&mut self) -> Option<DragAndDropResponse<Payload, End>> {
        self.response.take()
    }

    /// Adds a reordering drop zone onto an existing widget.
    pub fn reorder_drop_zone(&mut self, ui: &mut egui::Ui, r: egui::Response, end: End) {
        if !self.is_dragging() {
            return;
        }

        let Some(interact_pos) = ui.ctx().pointer_interact_pos() else {
            return;
        };

        let rect = r.rect.expand2(ui.spacing().item_spacing / 2.0);
        // Split the rectangle into left & right halves.
        let (left_half, right_half) = rect.split_left_right_at_x(rect.center().x);
        let hovering_left = left_half.contains(interact_pos);
        let hovering_right = right_half.contains(interact_pos);
        let before_or_after = match (hovering_left, hovering_right) {
            (true, _) => Some(BeforeOrAfter::Before),
            (_, true) => Some(BeforeOrAfter::After),
            _ => None,
        };

        // Compute stroke.
        let color = ui.visuals().widgets.active.bg_stroke.color;
        let get_stroke = |is_hovering| egui::Stroke {
            width: 2.0,
            color: color.linear_multiply(if is_hovering { 1.0 } else { 0.005 }),
        };
        let left_stroke = get_stroke(hovering_left);
        let right_stroke = get_stroke(hovering_right);

        ui.painter()
            .line_segment([rect.left_top(), rect.left_bottom()], left_stroke);
        ui.painter()
            .line_segment([rect.right_top(), rect.right_bottom()], right_stroke);

        if ui.input(|input| input.pointer.any_released()) && (hovering_left || hovering_right) {
            let Some(payload) = self.take_payload() else {
                return;
            };
            self.response = Some(DragAndDropResponse {
                payload,
                end,
                before_or_after,
            });
        }
    }
}

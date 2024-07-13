use std::any::Any;

use float_ord::FloatOrd;

use crate::{gui::util::EguiTempValue, util::BeforeOrAfter};

const REORDER_STROKE_WIDTH: f32 = 2.0;
const DROP_ZONE_STROKE_WIDTH: f32 = 2.0;
const DROP_ZONE_ROUNDING: f32 = 3.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DragAndDropResponse<Payload, End> {
    pub payload: Payload,
    pub end: End,
    pub before_or_after: Option<BeforeOrAfter>,
}

#[derive(Debug, Clone)]
pub struct DragAndDrop<Payload, End = Payload> {
    /// Opacity of UI element being dragged.
    pub dragging_opacity: f32,

    reorder_drop_zones: Vec<([egui::Pos2; 2], egui::Direction, End, BeforeOrAfter)>,

    /// Response containing the initial payload and where it ended up.
    response: Option<DragAndDropResponse<Payload, End>>,
    done_dragging: bool,

    payload: EguiTempValue<Payload>,
    cursor_offset: EguiTempValue<egui::Vec2>,
}
impl<Payload, End> DragAndDrop<Payload, End>
where
    Payload: Any + Default + Clone + Send + Sync,
    End: Clone,
{
    pub fn new(ui: &mut egui::Ui) -> Self {
        let this = Self {
            dragging_opacity: 1.0,

            reorder_drop_zones: vec![],

            response: None,
            done_dragging: ui.input(|input| input.pointer.any_released()),

            payload: EguiTempValue::new(ui),
            cursor_offset: EguiTempValue::new(ui),
        };

        if !ui.input(|input| input.pointer.any_down() || input.pointer.any_released()) {
            // Done dragging -> delete payload
            this.payload.take();
        }

        if ui.input(|input| input.key_pressed(egui::Key::Escape) || input.pointer.any_pressed()) {
            // Cancel drag
            if this.payload.take().is_some() {
                ui.ctx().stop_dragging();
            }
        }

        this
    }

    pub fn dragging_opacity(mut self, dragging_opacity: f32) -> Self {
        self.dragging_opacity = dragging_opacity;
        self
    }

    pub fn is_dragging(&self) -> bool {
        self.payload.get().is_some()
    }

    /// Adds a widget that is draggable only by its handle, along with a reorder
    /// drop zone. See [`Self::draggable()`].
    pub fn vertical_reorder_by_handle(
        &mut self,
        ui: &mut egui::Ui,
        payload: Payload,
        end: End,
        add_contents: impl FnOnce(&mut egui::Ui, bool),
    ) -> egui::Response {
        let r = self
            .draggable(ui, payload, |ui, is_dragging| {
                ui.horizontal(|ui| {
                    ui.set_width(ui.available_width());
                    let r = drag_handle(ui, is_dragging);
                    add_contents(ui, is_dragging);
                    r
                })
                .inner
            })
            .response;
        self.reorder_drop_zone(ui, &r, end);
        r
    }

    /// Adds a draggable widget.
    ///
    /// `payload` is a value representing the value that will be dragged. The
    /// boolean passed into `add_contents` is `true` if the widget is currently
    /// being dragged.
    pub fn draggable(
        &mut self,
        ui: &mut egui::Ui,
        payload: Payload,
        add_contents: impl FnOnce(&mut egui::Ui, bool) -> egui::Response,
    ) -> egui::InnerResponse<egui::Response> {
        let id = ui.auto_id_with("hyperspeedcube::drag_and_drop");

        if ui.ctx().is_being_dragged(id) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            // Paint the widget to a different layer so that we can move it
            // around independently. Highlight the widget so that it looks like
            // it's still being hovered.
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let mut r = ui.with_layer_id(layer_id, |ui| {
                ui.set_opacity(self.dragging_opacity);
                add_contents(ui, true)
            });
            r.inner = r.inner.highlight();

            ui.painter().rect_filled(
                r.response.rect,
                3.0,
                ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.1),
            );

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta = pointer_pos + self.cursor_offset.get().unwrap_or_default()
                    - r.response.rect.center();
                ui.ctx().transform_layer_shapes(
                    layer_id,
                    egui::emath::TSTransform::from_translation(delta),
                );
            }

            r
        } else {
            let mut r = ui.scope(|ui| add_contents(ui, false));

            if !r.inner.sense.click {
                r.inner = r.inner.on_hover_and_drag_cursor(egui::CursorIcon::Grab);
            }

            if r.inner.drag_started() {
                ui.ctx().set_dragged_id(id);
                self.payload.set(Some(payload));
                self.cursor_offset.set(
                    r.inner
                        .interact_pointer_pos()
                        .map(|interact_pos| r.response.rect.center() - interact_pos),
                );
            }

            r
        }
    }

    /// Add a drop zone onto an existing widget.
    ///
    /// `end` is a value representing this drop zone.
    pub fn drop_zone(&mut self, ui: &mut egui::Ui, r: &egui::Response, end: End) {
        if !self.is_dragging() {
            return;
        }

        let color = ui.visuals().widgets.active.bg_stroke.color;
        let width = DROP_ZONE_STROKE_WIDTH;
        let active_stroke = egui::Stroke { width, color };

        let color = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let inactive_stroke = egui::Stroke { width, color };

        let is_active = ui
            .input(|input| input.pointer.interact_pos())
            .is_some_and(|pos| {
                r.interact_rect
                    .contains(pos + self.cursor_offset.get().unwrap_or_default())
            });

        let stroke = if is_active {
            active_stroke
        } else {
            inactive_stroke
        };

        ui.painter().rect_stroke(r.rect, DROP_ZONE_ROUNDING, stroke);

        if is_active {
            let Some(payload) = self.payload.get() else {
                return;
            };
            self.response = Some(DragAndDropResponse {
                payload,
                end,
                before_or_after: None,
            });
        }
    }

    /// Adds a reordering drop zone onto an existing widget.
    pub fn reorder_drop_zone(&mut self, ui: &mut egui::Ui, r: &egui::Response, end: End) {
        if !self.is_dragging() {
            return;
        }

        let rect = r.rect.expand2(ui.spacing().item_spacing / 2.0);

        let dir = ui.layout().main_dir;
        let ul = rect.left_top();
        let ur = rect.right_top();
        let dl = rect.left_bottom();
        let dr = rect.right_bottom();
        let line1 = [ul, if dir.is_horizontal() { dl } else { ur }];
        let line2 = [if dir.is_horizontal() { ur } else { dl }, dr];
        self.reorder_drop_zones
            .push((line1, dir, end.clone(), BeforeOrAfter::Before));
        self.reorder_drop_zones
            .push((line2, dir, end, BeforeOrAfter::After));
    }

    pub fn paint_reorder_drop_lines(&mut self, ui: &mut egui::Ui) {
        let Some(payload) = self.payload.get() else {
            return; // nothing being dragged
        };

        if self.response.is_some() {
            return; // already hovering a non-reorder drop zone
        }

        let Some(interact_pos) = ui.input(|input| input.pointer.interact_pos()) else {
            return; // no cursor position
        };
        let drop_pos = interact_pos + self.cursor_offset.get().unwrap_or_default();

        if !ui.clip_rect().contains(drop_pos) {
            return; // cursor position is outside the current UI
        }

        let closest = std::mem::take(&mut self.reorder_drop_zones)
            .into_iter()
            .filter_map(|params| {
                let (points, dir, _, _) = &params;
                let distance_to_cursor = if dir.is_horizontal() {
                    (points[0].y..=points[1].y)
                        .contains(&drop_pos.y)
                        .then(|| (points[0].x - drop_pos.x).abs())
                } else {
                    (points[0].x..=points[1].x)
                        .contains(&drop_pos.x)
                        .then(|| (points[0].y - drop_pos.y).abs())
                };
                Some((params, distance_to_cursor?))
            })
            .min_by_key(|(_params, distance_to_cursor)| FloatOrd(*distance_to_cursor));

        self.response = closest.map(|((points, _dir, end, before_or_after), _distance)| {
            let color = ui.visuals().widgets.active.bg_stroke.color;
            let stroke = egui::Stroke::new(REORDER_STROKE_WIDTH, color);
            ui.painter().line_segment(points, stroke);

            DragAndDropResponse {
                payload,
                end,
                before_or_after: Some(before_or_after),
            }
        });
    }

    pub fn mid_drag(&mut self) -> Option<&DragAndDropResponse<Payload, End>> {
        self.response.as_ref()
    }

    pub fn end_drag(&mut self) -> Option<DragAndDropResponse<Payload, End>> {
        if self.done_dragging {
            self.payload.take();
            self.response.take()
        } else {
            None
        }
    }
}

pub fn drag_handle(ui: &mut egui::Ui, is_dragging: bool) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::drag());
    if ui.is_rect_visible(rect) {
        // Change color based on hover/focus.
        let color = if r.has_focus() || is_dragging {
            ui.visuals().strong_text_color()
        } else if r.hovered() {
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
    r
}

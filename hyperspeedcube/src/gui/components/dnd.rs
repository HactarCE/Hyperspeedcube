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

            payload: EguiTempValue::from_ui(ui),
            cursor_offset: EguiTempValue::from_ui(ui),
        };

        if !ui.input(|input| input.pointer.any_down() || input.pointer.any_released()) {
            // Done dragging -> delete payload
            this.take_payload();
        }

        if ui.input(|input| input.key_pressed(egui::Key::Escape) || input.pointer.any_pressed()) {
            // Cancel drag
            if this.take_payload().is_some() {
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
        self.payload().is_some()
    }
    pub fn set_payload(&self, payload: Payload) {
        self.payload.set(Some(payload));
    }
    pub fn payload(&self) -> Option<Payload> {
        self.payload.get()
    }
    pub fn take_payload(&self) -> Option<Payload> {
        self.payload.set(None)
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
    ) -> egui::Response {
        let drag_start_id = unique_id!();
        let id = ui.auto_id_with("hyperspeedcube::drag_and_drop");

        if ui.ctx().is_being_dragged(id) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            // Paint the widget to a different layer so that we can move it
            // around independently. Highlight the widget so that it looks like
            // it's still being hovered.
            let layer_id = egui::LayerId::new(egui::Order::Tooltip, id);
            let r = ui
                .with_layer_id(layer_id, |ui| {
                    ui.set_opacity(self.dragging_opacity);
                    add_contents(ui, true)
                })
                .inner
                .highlight();

            ui.painter().rect_filled(
                r.rect,
                3.0,
                ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.1),
            );

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let delta =
                    pointer_pos + self.cursor_offset.get().unwrap_or_default() - r.rect.center();
                ui.ctx().transform_layer_shapes(
                    layer_id,
                    egui::emath::TSTransform::from_translation(delta),
                );
            }

            r
        } else {
            let mut r = add_contents(ui, false);

            if !r.sense.click {
                r = r.on_hover_and_drag_cursor(egui::CursorIcon::Grab);
            }

            if r.drag_started() {
                ui.ctx().set_dragged_id(id);
                self.payload.set(Some(payload));
                self.cursor_offset.set(
                    r.interact_pointer_pos()
                        .map(|interact_pos| r.rect.center() - interact_pos),
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
            let Some(payload) = self.payload() else {
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
    pub fn reorder_drop_zone(&mut self, ui: &mut egui::Ui, r: egui::Response, end: End) {
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

    pub fn draw_reorder_drop_lines(&mut self, ui: &mut egui::Ui) {
        let Some(payload) = self.payload() else {
            return;
        };

        let mut last_line = None;
        let closest = std::mem::take(&mut self.reorder_drop_zones)
            .into_iter()
            .filter_map(|params| {
                let (line, dir, _end, _before_or_after) = &params;
                if last_line
                    .replace(*line)
                    .is_some_and(|it| lines_approx_eq(it, *line))
                {
                    return None;
                }
                let distance = self.draw_reorder_drop_line(ui, *line, *dir, false)?;
                Some((params, distance))
            })
            .min_by_key(|(_params, distance)| FloatOrd(*distance));

        let Some(((line, dir, end, before_or_after), _distance)) = closest else {
            return;
        };

        self.draw_reorder_drop_line(ui, line, dir, true);

        self.response = Some(DragAndDropResponse {
            payload,
            end,
            before_or_after: Some(before_or_after),
        });
    }
    /// Draws a reorder drop line and returns the distance from the line to the
    /// cursor, if the cursor is in line with the target.
    fn draw_reorder_drop_line(
        &self,
        ui: &mut egui::Ui,
        mut points: [egui::Pos2; 2],
        dir: egui::Direction,
        is_selected: bool,
    ) -> Option<f32> {
        let color = ui.visuals().widgets.active.bg_stroke.color;
        let stroke = egui::Stroke {
            width: REORDER_STROKE_WIDTH,
            color: color.linear_multiply(if is_selected { 1.0 } else { 0.05 }),
        };

        let drop_pos = ui.input(|input| input.pointer.interact_pos())?
            + self.cursor_offset.get().unwrap_or_default();
        if !ui.clip_rect().contains(drop_pos) {
            return None;
        }

        let distance = if dir.is_horizontal() {
            points.sort_by_key(|p| FloatOrd(p.y));
            (points[0].y..=points[1].y)
                .contains(&drop_pos.y)
                .then(|| (points[0].x - drop_pos.x).abs())
        } else {
            points.sort_by_key(|p| FloatOrd(p.x));
            (points[0].x..=points[1].x)
                .contains(&drop_pos.x)
                .then(|| (points[0].y - drop_pos.y).abs())
        };

        // Shrink each line a tiny bit so they don't overlap in a wrapping
        // layout. Do this after calculating distance so that there's no gap
        // when measuring pointer position.
        let [mut a, mut b] = points;
        let v = (b - a).normalized();
        a += v;
        b -= v;
        ui.painter().line_segment([a, b], stroke);

        distance
    }

    pub fn mid_drag(&mut self) -> Option<&DragAndDropResponse<Payload, End>> {
        self.response.as_ref()
    }

    pub fn end_drag(&mut self) -> Option<DragAndDropResponse<Payload, End>> {
        if self.done_dragging {
            self.take_payload();
            self.response.take()
        } else {
            None
        }
    }
}

pub fn drag_handle(ui: &mut egui::Ui) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::drag());
    if ui.is_rect_visible(rect) {
        // Change color based on hover/focus.
        let color = if r.has_focus() {
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

fn lines_approx_eq([a1, b1]: [egui::Pos2; 2], [a2, b2]: [egui::Pos2; 2]) -> bool {
    [(a1.x, a2.x), (a1.y, a2.y), (b1.x, b2.x), (b1.y, b2.y)]
        .into_iter()
        .all(|(x1, x2)| (x1 - x2).abs() < 1.0)
}

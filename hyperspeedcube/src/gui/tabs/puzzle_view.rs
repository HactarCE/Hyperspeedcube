use std::sync::Arc;

use bitvec::boxed::BitBox;
use cgmath::EuclideanSpace;
use egui::NumExt;
use hypermath::prelude::*;
use hyperpuzzle::Puzzle;
use parking_lot::Mutex;

use crate::{
    gfx::*,
    preferences::Preferences,
    puzzle::{PieceStyleState, PuzzleController, PuzzleViewController},
};

#[derive(Debug)]
pub struct PuzzleView {
    pub view_controller: PuzzleViewController,
    /// Puzzle renderer. This is wrapped in an `Arc<Mutex<T>>` because egui
    /// needs access to it during rendering, when we are not in control.
    renderer: Arc<Mutex<PuzzleRenderer>>,

    is_dragging: bool,
}
impl PuzzleView {
    pub(crate) fn new(gfx: &Arc<GraphicsState>, puzzle: &Arc<Puzzle>) -> Self {
        Self::with_controller(gfx, &Arc::new(Mutex::new(PuzzleController::new(puzzle))))
    }
    pub(crate) fn with_controller(
        gfx: &Arc<GraphicsState>,
        controller: &Arc<Mutex<PuzzleController>>,
    ) -> Self {
        let view_controller = PuzzleViewController::with_state(&controller);
        let puzzle = view_controller.puzzle();
        let renderer = Arc::new(Mutex::new(PuzzleRenderer::new(gfx, &puzzle)));
        Self {
            view_controller,
            renderer,

            is_dragging: false,
        }
    }

    /// Returns the puzzle controller.
    pub fn controller(&self) -> &Arc<Mutex<PuzzleController>> {
        &self.view_controller.state
    }
    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.controller().lock().puzzle)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
        let mut renderer = self.renderer.lock();

        let dpi = ui.ctx().pixels_per_point();

        // Round rectangle to pixel boundary for crisp image.
        let mut pixels_rect = ui.available_rect_before_wrap();
        pixels_rect.set_left((dpi * pixels_rect.left()).ceil());
        pixels_rect.set_bottom((dpi * pixels_rect.bottom()).floor());
        pixels_rect.set_right((dpi * pixels_rect.right()).floor());
        pixels_rect.set_top((dpi * pixels_rect.top()).ceil());

        // Convert back from pixel coordinates to egui coordinates.
        let mut egui_rect = pixels_rect;
        *egui_rect.left_mut() /= dpi;
        *egui_rect.bottom_mut() /= dpi;
        *egui_rect.right_mut() /= dpi;
        *egui_rect.top_mut() /= dpi;

        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());

        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
        }

        // egui reports `r.dragged()` whenever the mouse is held, even if it
        // didn't move, so we manually keep track of whether the mouse has
        // moved.
        if !r.dragged() {
            self.is_dragging = false;
        }
        if r.drag_delta() != egui::Vec2::ZERO {
            self.is_dragging = true
        }

        let puzzle = self.puzzle();
        let view_prefs = prefs.view(&puzzle).clone();

        let view_ctrl = &mut self.view_controller;

        let target_size = [
            pixels_rect.width() as u32 / view_prefs.downscale_rate,
            pixels_rect.height() as u32 / view_prefs.downscale_rate,
        ];

        let min_size = egui_rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta = r.drag_delta() * DRAG_SPEED / min_size.abs();
        // Convert to higher precision before dividing.
        let scaled_drag_x = drag_delta.x as Float / view_ctrl.zoom.at_least(1.0) as Float;
        let scaled_drag_y = drag_delta.y as Float / view_ctrl.zoom.at_least(1.0) as Float;

        let scroll_delta = ui.input(|input| input.scroll_delta);
        let mut mouse_pos: Option<cgmath::Point2<f32>> = None;
        if r.hovered() && !self.is_dragging {
            view_ctrl.zoom *= (scroll_delta.y / 100.0).exp2();
            view_ctrl.zoom = view_ctrl.zoom.clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(8.0));
            if let Some(pos) = r.hover_pos() {
                let mouse_pos_ndc = (pos - r.rect.center()) * 2.0 / r.rect.size();
                mouse_pos = Some(cgmath::point2(mouse_pos_ndc.x, -mouse_pos_ndc.y));
            }
        }

        let mut z_axis = 2;
        if ui.input(|input| input.modifiers.shift) {
            z_axis += 1;
        }
        if ui.input(|input| input.modifiers.alt) {
            z_axis += 2;
        };
        view_ctrl.rot = Isometry::from_angle_in_axis_plane(0, z_axis, -scaled_drag_x)
            * Isometry::from_angle_in_axis_plane(1, z_axis, scaled_drag_y)
            * &view_ctrl.rot;

        if r.has_focus() {
            ui.input(|input| {
                for (key, n) in [
                    (egui::Key::Num1, 1),
                    (egui::Key::Num2, 2),
                    (egui::Key::Num3, 3),
                    (egui::Key::Num4, 4),
                    (egui::Key::Num5, 5),
                    (egui::Key::Num6, 6),
                    (egui::Key::Num7, 7),
                    (egui::Key::Num8, 8),
                    (egui::Key::Num9, 9),
                    (egui::Key::Num0, 0),
                ] {
                    if input.key_pressed(key) {
                        let piece_set: BitBox<u64> = puzzle
                            .pieces
                            .iter()
                            .map(|(_piece, info)| info.stickers.len() == n)
                            .collect();
                        let hidden = !view_ctrl.styles.is_any_hidden(&piece_set);
                        view_ctrl
                            .styles
                            .set_piece_states(&piece_set, |old| PieceStyleState { hidden, ..old });
                    }
                }
            });
        }

        if renderer.vertex_3d_positions().is_none() {
            // Redraw each frame until the image is stable and we have
            // computed 3D vertex positions.
            ui.ctx().request_repaint();
        }

        // Update hover state.
        view_ctrl.set_hover_state((|| {
            // IIFE to mimic try_block
            let p = mouse_pos?.to_vec();
            let s = DrawParams::compute_xy_scale(target_size, view_ctrl.zoom).ok()?;
            let screen_space_mouse_pos = cgmath::point2(p.x / s.x, p.y / s.y);

            let vertex_3d_positions = renderer.vertex_3d_positions()?;

            view_ctrl.compute_hover_state(screen_space_mouse_pos, &vertex_3d_positions, prefs)
        })());

        let dark_mode = ui.ctx().style().visuals.dark_mode;
        let background_color = crate::util::color_to_u8x3(prefs.styles.background_color(dark_mode));
        let internals_color = crate::util::color_to_u8x3(prefs.styles.internals_color);

        let draw_params = DrawParams {
            prefs: view_prefs,

            target_size,
            mouse_pos: mouse_pos.unwrap_or_else(cgmath::Point2::origin).into(),

            rot: view_ctrl.rot.clone(),
            zoom: view_ctrl.zoom,

            background_color,
            internals_color,
            piece_styles: view_ctrl.styles.values(&prefs.styles),
        };

        let draw_prep = renderer.prepare_draw(draw_params);

        if !draw_prep
            .vertex_3d_positions
            .is_some_and(|inner| inner.lock().is_some())
        {
            ui.ctx().request_repaint();
        }

        // Draw puzzle.
        let painter = ui.painter_at(egui_rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            egui_rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&renderer.gfx),
                renderer: Arc::clone(&self.renderer),
            },
        ));

        r
    }
}

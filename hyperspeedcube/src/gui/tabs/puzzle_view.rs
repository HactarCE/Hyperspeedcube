use std::sync::Arc;

use hypermath::prelude::*;
use hyperpuzzle::{Mesh, Piece, Puzzle};
use parking_lot::Mutex;

use crate::{
    gfx::{GraphicsState, PuzzleRenderResources, PuzzleRenderer, RenderEngine, ViewParams},
    preferences::{Preferences, ViewPreferences},
};

#[derive(Debug)]
pub struct PuzzleView {
    pub puzzle: Option<Arc<Puzzle>>,
    renderer: Option<Arc<Mutex<PuzzleRenderer>>>,
    pub view_params: ViewParams,
    gfx: Arc<GraphicsState>,

    rect: egui::Rect,
    // TODO: rename this to `render_strategy`
    pub render_engine: RenderEngine,

    highlighted_piece_types: [bool; 10],

    pub overlay: Vec<(Overlay, f32, egui::Color32)>,
}
impl PuzzleView {
    // TODO: remove `cc` parameter if not needed
    pub(crate) fn new(gfx: &Arc<GraphicsState>) -> Self {
        PuzzleView {
            puzzle: None,
            renderer: None,
            view_params: ViewParams::default(),
            gfx: Arc::clone(gfx),

            rect: egui::Rect::NOTHING,
            render_engine: RenderEngine::default(),

            highlighted_piece_types: [true; 10],

            overlay: vec![],
        }
    }
    pub(crate) fn set_puzzle(&mut self, puzzle: Arc<Puzzle>) {
        self.view_params.piece_opacities = puzzle.pieces.map_ref(|_, _| 1.0);
        self.puzzle = Some(Arc::clone(&puzzle));
        self.renderer = Some(Arc::new(Mutex::new(PuzzleRenderer::new(&self.gfx, puzzle))));
    }
    pub(crate) fn ndim(&self) -> Option<u8> {
        self.puzzle.as_ref().map(|puzzle| puzzle.ndim())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
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

        self.rect = egui_rect;

        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());
        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
        }

        let (Some(puzzle), Some(renderer)) = (&self.puzzle, &self.renderer) else {
            // Hint to the user to load a puzzle.
            ui.allocate_ui_at_rect(egui_rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a puzzle from the puzzle list");
                });
            });
            return r;
        };

        let min_size = egui_rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta = r.drag_delta() * DRAG_SPEED / min_size.abs();

        let scroll_delta = ui.input(|input| input.scroll_delta);
        if r.hovered() {
            self.view_params.zoom *= (scroll_delta.y / 100.0).exp2();
            self.view_params.zoom = self
                .view_params
                .zoom
                .clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(12.0));
        }

        let mut z_axis = 2;
        if ui.input(|input| input.modifiers.shift) {
            z_axis += 1;
        }
        if ui.input(|input| input.modifiers.alt) {
            z_axis += 2;
        };
        self.view_params.rot =
            Isometry::from_angle_in_axis_plane(0, z_axis, -drag_delta.x as Float)
                * Isometry::from_angle_in_axis_plane(1, z_axis, drag_delta.y as Float)
                * &self.view_params.rot;

        self.view_params.width = pixels_rect.width() as u32;
        self.view_params.height = pixels_rect.height() as u32;
        if r.has_focus() {
            ui.input(|input| {
                if input.key_pressed(egui::Key::Num1) {
                    self.highlighted_piece_types[1] ^= true;
                }
                if input.key_pressed(egui::Key::Num2) {
                    self.highlighted_piece_types[2] ^= true;
                }
                if input.key_pressed(egui::Key::Num3) {
                    self.highlighted_piece_types[3] ^= true;
                }
                if input.key_pressed(egui::Key::Num4) {
                    self.highlighted_piece_types[4] ^= true;
                }
                if input.key_pressed(egui::Key::Num5) {
                    self.highlighted_piece_types[5] ^= true;
                }
                if input.key_pressed(egui::Key::Num6) {
                    self.highlighted_piece_types[6] ^= true;
                }
                if input.key_pressed(egui::Key::Num7) {
                    self.highlighted_piece_types[7] ^= true;
                }
                if input.key_pressed(egui::Key::Num8) {
                    self.highlighted_piece_types[8] ^= true;
                }
                if input.key_pressed(egui::Key::Num9) {
                    self.highlighted_piece_types[9] ^= true;
                }
                if input.key_pressed(egui::Key::Num0) {
                    self.highlighted_piece_types[0] ^= true;
                }
            });
        }
        self.view_params.piece_opacities = puzzle.pieces.map_ref(|piece, info| {
            let sticker_count = info.stickers.len();
            let fallback = self.highlighted_piece_types[0];
            match self
                .highlighted_piece_types
                .get(sticker_count)
                .unwrap_or(&fallback)
            {
                true => 1.0,
                false => 0.2,
            }
        });

        self.view_params.prefs = prefs.view(puzzle).clone();
        self.view_params.background_color = prefs.colors.background;
        self.view_params.outlines_color = prefs.outlines.default_color;

        // Render overlay
        let transform_point = |p: &Vector| -> Option<egui::Pos2> {
            let mut p = self.view_params.project_point(p)?;
            p.x *= egui_rect.size().x / 2.0 / 1.5;
            p.y *= egui_rect.size().y / 2.0 / 1.5;
            Some(egui_rect.center() + egui::vec2(p.x, -p.y))
        };
        for (overlay, size, color) in &self.overlay {
            let color = *color;
            // IIFE to mimic try_block
            let _ = (|| -> Option<()> {
                match overlay {
                    Overlay::Point(p) => {
                        ui.painter()
                            .circle_filled(transform_point(p)?, 5.0 * size, color)
                    }
                    Overlay::Line(p1, p2) => ui.painter().line_segment(
                        [transform_point(p1)?, transform_point(p2)?],
                        egui::Stroke {
                            width: 4.0 * size,
                            color,
                        },
                    ),
                    Overlay::Arrow(p1, p2) => ui.painter().arrow(
                        transform_point(p1)?,
                        transform_point(p2)? - transform_point(p1)?,
                        egui::Stroke {
                            width: 4.0 * size,
                            color,
                        },
                    ),
                }
                None
            })();
        }

        // Draw puzzle.
        let painter = ui.painter_at(egui_rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            egui_rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&self.gfx),
                renderer: Arc::clone(&renderer),
                render_engine: self.render_engine,
                view_params: self.view_params.clone(),
            },
        ));

        r
    }

    /// Adds an animation to the view settings animation queue.
    pub fn animate_from_view_settings(&mut self, view_prefs: ViewPreferences) {
        // TODO: animate
        // self.view_settings_anim.queue.push_back(view_prefs);
    }
}

#[derive(Debug, Clone)]
pub enum Overlay {
    Point(Vector),
    Line(Vector, Vector),
    Arrow(Vector, Vector),
}

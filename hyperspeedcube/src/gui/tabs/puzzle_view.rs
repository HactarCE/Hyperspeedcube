use std::sync::Arc;

use egui::NumExt;
use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, Puzzle};
use parking_lot::Mutex;

use crate::{
    gfx::{DrawParams, GraphicsState, PuzzleRenderResources, PuzzleRenderer, RenderEngine},
    preferences::{Preferences, ViewPreferences},
};

#[derive(Debug)]
pub struct PuzzleView {
    pub puzzle: Option<Arc<Puzzle>>,
    renderer: Option<Arc<Mutex<PuzzleRenderer>>>,
    gfx: Arc<GraphicsState>,

    prev_draw_params: Option<DrawParams>,
    pub rot: Isometry,
    zoom: f32,
    piece_face_opacities: PerPiece<f32>,
    piece_edge_opacities: PerPiece<f32>,

    rect: egui::Rect,
    // TODO: rename this to `render_strategy`
    pub render_engine: RenderEngine,

    highlighted_piece_types: [bool; 10],

    pub overlay: Vec<(Overlay, f32, egui::Color32)>,
}
impl PuzzleView {
    pub(crate) fn new(gfx: &Arc<GraphicsState>) -> Self {
        PuzzleView {
            puzzle: None,
            renderer: None,
            gfx: Arc::clone(gfx),

            prev_draw_params: None,
            rot: Isometry::ident(),
            zoom: 0.5,
            piece_face_opacities: PerPiece::default(),
            piece_edge_opacities: PerPiece::default(),

            rect: egui::Rect::NOTHING,
            render_engine: RenderEngine::default(),

            highlighted_piece_types: [true; 10],

            overlay: vec![],
        }
    }
    pub(crate) fn set_puzzle(&mut self, puzzle: Arc<Puzzle>) {
        self.piece_face_opacities = puzzle.pieces.map_ref(|_, _| 1.0);
        self.piece_edge_opacities = puzzle.pieces.map_ref(|_, _| 1.0);
        self.zoom = 1.0;
        self.rot = Isometry::ident();

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

        let view_prefs = prefs.view(puzzle).clone();

        let target_size = [
            pixels_rect.width() as u32 / view_prefs.downscale_rate,
            pixels_rect.height() as u32 / view_prefs.downscale_rate,
        ];

        let min_size = egui_rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta = r.drag_delta() * DRAG_SPEED / min_size.abs();
        // Convert to higher precision before dividing.
        let scaled_drag_x = drag_delta.x as Float / self.zoom.at_least(1.0) as Float;
        let scaled_drag_y = drag_delta.y as Float / self.zoom.at_least(1.0) as Float;

        let scroll_delta = ui.input(|input| input.scroll_delta);
        let mut mouse_pos: Option<[f32; 2]> = None;
        if r.hovered() {
            self.zoom *= (scroll_delta.y / 100.0).exp2();
            self.zoom = self.zoom.clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(8.0));
            if let Some(pos) = r.hover_pos() {
                let mouse_pos_ndc = (pos - r.rect.min) / r.rect.size();
                mouse_pos = Some(mouse_pos_ndc.into());
            }
        }

        let mut z_axis = 2;
        if ui.input(|input| input.modifiers.shift) {
            z_axis += 1;
        }
        if ui.input(|input| input.modifiers.alt) {
            z_axis += 2;
        };
        self.rot = Isometry::from_angle_in_axis_plane(0, z_axis, -scaled_drag_x)
            * Isometry::from_angle_in_axis_plane(1, z_axis, scaled_drag_y)
            * &self.rot;

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
        self.piece_face_opacities = puzzle.pieces.map_ref(|piece, info| {
            let sticker_count = info.stickers.len();
            let fallback = self.highlighted_piece_types[0];
            match self
                .highlighted_piece_types
                .get(sticker_count)
                .unwrap_or(&fallback)
            {
                true => 1.0,
                false => 0.0,
            }
        });
        self.piece_edge_opacities = puzzle.pieces.map_ref(|piece, info| 1.0);

        let current_draw_params = DrawParams {
            prefs: view_prefs,
            target_size,
            mouse_pos: [0.0; 2], // Don't bother refreshing when mouse moves.
            rot: self.rot.clone(),
            zoom: self.zoom,
            background_color: prefs.colors.background,
            outlines_color: prefs.outlines.default_color,
            piece_face_opacities: self.piece_face_opacities.clone(),
            piece_edge_opacities: self.piece_edge_opacities.clone(),
        };

        let force_redraw = !self
            .prev_draw_params
            .as_ref()
            .is_some_and(|x| *x == current_draw_params);
        if force_redraw {
            self.prev_draw_params = Some(current_draw_params.clone());
        }

        // Draw puzzle.
        let painter = ui.painter_at(egui_rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            egui_rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&self.gfx),
                renderer: Arc::clone(&renderer),
                render_engine: self.render_engine,
                draw_params: current_draw_params,
                force_redraw,
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

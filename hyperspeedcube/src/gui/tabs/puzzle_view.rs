use std::fmt;
use std::sync::Arc;

use hypermath::prelude::*;
use hyperpuzzle::{Mesh, Puzzle};
use hypershape::Space;

use crate::render::{GraphicsState, PuzzleRenderer, ViewParams};

#[derive(Debug)]
pub struct PuzzleView {
    pub space: Space,
    pub puzzle: Option<Arc<Puzzle>>,
    renderer: PuzzleRenderer,
    pub view_params: ViewParams,

    texture_id: egui::TextureId,
    rect: egui::Rect,
    pub render_engine: RenderEngine,

    pub overlay: Vec<(Overlay, f32, egui::Color32)>,
}
impl PuzzleView {
    pub(crate) fn new(gfx: &GraphicsState, egui_renderer: &mut egui_wgpu::Renderer) -> Self {
        let texture_id = egui_renderer.register_native_texture(
            &gfx.device,
            &gfx.dummy_texture_view(),
            wgpu::FilterMode::Linear,
        );

        let space = Space::new(3);
        let mesh = Mesh::default();

        PuzzleView {
            space,
            puzzle: None,
            renderer: PuzzleRenderer::new(gfx, &mesh),
            view_params: ViewParams::default(),

            texture_id,
            rect: egui::Rect::NOTHING,
            render_engine: RenderEngine::SinglePass,

            overlay: vec![],
        }
    }
    pub(crate) fn set_mesh(&mut self, gfx: &GraphicsState, space: Space, mesh: Option<&Mesh>) {
        self.space = space;
        if let Some(mesh) = mesh {
            self.renderer = PuzzleRenderer::new(gfx, mesh);
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
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

        let r = ui.put(
            egui_rect,
            egui::Image::new(self.texture_id, egui_rect.size())
                .sense(egui::Sense::click_and_drag()),
        );

        let min_size = egui_rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta = r.drag_delta() * DRAG_SPEED / min_size.abs();

        let scroll_delta = ui.input(|input| input.scroll_delta);
        if r.hovered() {
            self.view_params.zoom *= (scroll_delta.y / 100.0).exp2();
        }

        let z_axis = if ui.input(|input| input.modifiers.shift) {
            3
        } else {
            2
        };
        self.view_params.rot =
            Isometry::from_angle_in_axis_plane(0, z_axis, -drag_delta.x as Float)
                * Isometry::from_angle_in_axis_plane(1, z_axis, drag_delta.y as Float)
                * &self.view_params.rot;

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

        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
            true
        } else {
            false
        }
    }

    pub(crate) fn render_and_update_texture(
        &mut self,
        gfx: &GraphicsState,
        egui_ctx: &egui::Context,
        egui_renderer: &mut egui_wgpu::Renderer,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let view_params = &mut self.view_params;

        view_params.width = self.rect.width() as u32;
        view_params.height = self.rect.height() as u32;
        let new_texture = match self.render_engine {
            RenderEngine::SinglePass => {
                self.renderer
                    .draw_puzzle_single_pass(gfx, encoder, &view_params)
            }
            RenderEngine::MultiPass => self.renderer.draw_puzzle(gfx, encoder, &view_params),
        };

        // Draw puzzle if necessary.
        if let Ok(texture) = new_texture {
            log::trace!("Updating puzzle texture");

            // Update texture for egui.
            egui_renderer.update_egui_texture_from_wgpu_texture(
                &gfx.device,
                texture,
                wgpu::FilterMode::Linear,
                self.texture_id,
            );

            // Request a repaint.
            egui_ctx.request_repaint();
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RenderEngine {
    SinglePass,
    #[default]
    MultiPass,
}
impl fmt::Display for RenderEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderEngine::SinglePass => write!(f, "Fast"),
            RenderEngine::MultiPass => write!(f, "Fancy"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Overlay {
    Point(Vector),
    Line(Vector, Vector),
    Arrow(Vector, Vector),
}

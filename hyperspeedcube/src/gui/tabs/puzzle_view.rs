use std::sync::Arc;

use cgmath::EuclideanSpace;
use hypermath::prelude::*;
use hyperpuzzle::{PieceMask, Puzzle};
use parking_lot::Mutex;

use crate::gfx::*;
use crate::gui::App;
use crate::preferences::Preferences;
use crate::puzzle::{DragState, PieceStyleState, PuzzleSimulation, PuzzleView, PuzzleViewInput};

pub fn show(ui: &mut egui::Ui, app: &mut App, puzzle_view: &Arc<Mutex<Option<PuzzleWidget>>>) {
    let r = match &mut *puzzle_view.lock() {
        Some(puzzle_view) => puzzle_view.ui(ui, &app.prefs),
        None => {
            // Hint to the user to load a puzzle.
            ui.allocate_ui_at_rect(ui.available_rect_before_wrap(), |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a puzzle from the puzzle list");
                });
            })
            .response
        }
    };

    if r.gained_focus() {
        app.active_puzzle_view = Arc::downgrade(puzzle_view);
    }
}

#[derive(Debug)]
pub struct PuzzleWidget {
    /// View into a puzzle simulation.
    pub view: PuzzleView,

    /// Puzzle renderer. This is wrapped in an `Arc<Mutex<T>>` because egui
    /// needs access to it during rendering, when we are not in control.
    renderer: Arc<Mutex<PuzzleRenderer>>,

    queued_arrows: Vec<[Vector; 2]>,
}
impl PuzzleWidget {
    pub(crate) fn new(gfx: &Arc<GraphicsState>, puzzle: &Arc<Puzzle>) -> Self {
        Self::with_sim(gfx, &Arc::new(Mutex::new(PuzzleSimulation::new(puzzle))))
    }
    pub(crate) fn with_sim(gfx: &Arc<GraphicsState>, sim: &Arc<Mutex<PuzzleSimulation>>) -> Self {
        let view = PuzzleView::new(&sim);
        let puzzle = view.puzzle();
        let renderer = Arc::new(Mutex::new(PuzzleRenderer::new(gfx, &puzzle)));
        Self {
            view,
            renderer,

            queued_arrows: vec![],
        }
    }

    /// Returns the puzzle simulation.
    pub fn sim(&self) -> &Arc<Mutex<PuzzleSimulation>> {
        &self.view.sim
    }
    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.sim().lock().puzzle_type())
    }

    /// Draws the puzzle in the UI and handles input.
    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
        let puzzle = self.puzzle();
        let view_prefs = prefs.view(&puzzle).clone();

        // Allocate space in the UI.
        let (egui_rect, target_size) = crate::gui::util::rounded_pixel_rect(
            ui,
            ui.available_rect_before_wrap(),
            view_prefs.downscale_rate,
        );
        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());

        // Request focus on click.
        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
        }

        // egui reports `r.dragged()` whenever the mouse is held, even if it
        // didn't move, so we manually keep track of whether the mouse has
        // moved.
        if r.drag_delta() != egui::Vec2::ZERO && self.view.drag_state().is_none() {
            let is_primary = ui.input(|input| input.pointer.primary_down());
            if is_primary && self.view.hover_state().is_some() {
                self.view.set_drag_state(DragState::PreTwist);
            } else {
                self.view.set_drag_state(DragState::ViewRot { z_axis: 2 });
            }
        }
        // Confirm drag on mouse button release.
        if !r.dragged() {
            self.view.confirm_drag();
        }
        // Cancel drag on ESC key press.
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.view.cancel_drag();
        }

        // Change which axis we're rotating depending on modifiers.
        if matches!(self.view.drag_state(), Some(DragState::ViewRot { .. })) {
            let modifiers = ui.input(|input| input.modifiers);
            let mut z_axis = 2;
            if modifiers.shift {
                z_axis += 1;
            }
            if modifiers.alt {
                z_axis += 2;
            }
            self.view.set_drag_state(DragState::ViewRot { z_axis });
        }

        let exceeded_twist_drag_threshold = ui
            .input(|input| {
                let delta = input.pointer.press_origin()? - input.pointer.interact_pos()?;
                Some(delta.length() >= crate::TWIST_DRAG_THRESHOLD)
            })
            .unwrap_or(false);

        // Compute the screen-space cursor position.
        let scroll_delta = ui.input(|input| input.raw_scroll_delta);
        let mut cursor_pos: Option<cgmath::Point2<f32>> = None;
        if r.hovered() || r.is_pointer_button_down_on() {
            // IIFE to mimic try_block
            cursor_pos = (|| {
                let egui_pos = r.hover_pos()?;
                // Convert to normalized device coordinates (-1 to +1).
                let mut ndc = (egui_pos - r.rect.center()) * 2.0 / r.rect.size();
                ndc.y = -ndc.y;
                // Convert to screen space.
                let s = self.view.camera.xy_scale().ok()?;
                Some(cgmath::point2(ndc.x / s.x, ndc.y / s.y))
            })();

            if self.view.drag_state().is_none() {
                // Adjust camera zoom using scroll wheel.
                let cam = &mut self.view.camera;
                cam.zoom *= (scroll_delta.y / 100.0).exp2();
                cam.zoom = cam.zoom.clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(8.0));
            }
        }

        // TODO: remove temporary piece filters
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
                        let all_pieces = &puzzle.pieces;
                        let piece_set = PieceMask::from_iter(
                            all_pieces.len(),
                            all_pieces.iter_filter(|_, info| info.stickers.len() == n),
                        );
                        let hidden = !self.view.styles.is_any_hidden(&piece_set);
                        self.view
                            .styles
                            .set_piece_states(&piece_set, |old| PieceStyleState { hidden, ..old });
                    }
                }
            });
        }

        let mut renderer = self.renderer.lock();

        self.view.update(PuzzleViewInput {
            cursor_pos,
            target_size,
            vertex_3d_positions: renderer.vertex_3d_positions(),
            prefs,
            exceeded_twist_drag_threshold,
        });

        // Redraw each frame until the image is stable and we have computed 3D
        // vertex positions.
        if renderer.vertex_3d_positions().is_none() {
            ui.ctx().request_repaint();
        }

        // Check for twist clicks.
        if r.clicked() {
            self.view.do_sticker_click(Sign::Neg);
        }
        if r.secondary_clicked() {
            self.view.do_sticker_click(Sign::Pos);
        }

        let dark_mode = ui.ctx().style().visuals.dark_mode;
        let background_color = crate::util::color_to_u8x3(prefs.styles.background_color(dark_mode));
        let internals_color = crate::util::color_to_u8x3(prefs.styles.internals_color);

        let draw_params = DrawParams {
            cam: self.view.camera.clone(),

            cursor_pos: cursor_pos.unwrap_or_else(cgmath::Point2::origin).into(),

            background_color,
            internals_color,
            piece_styles: self.view.styles.values(&prefs.styles),
            piece_transforms: self.view.sim.lock().piece_transforms().map_ref(
                |_piece, transform| transform.euclidean_rotation_matrix().at_ndim(puzzle.ndim()),
            ),
        };

        let draw_prep = renderer.prepare_draw(draw_params);

        if !draw_prep
            .vertex_3d_positions
            .is_some_and(|inner| inner.lock().is_some())
        {
            ui.ctx().request_repaint();
        }

        // Draw puzzle.
        let painter = ui.painter_at(r.rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            r.rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&renderer.gfx),
                renderer: Arc::clone(&self.renderer),
            },
        ));

        self.queued_arrows.extend(self.view.drag_delta_3d());

        let project_point = |p: &Vector| {
            let ndc = self.view.camera.project_point_to_ndc(p)?;
            let egui_pos = egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
            Some(r.rect.lerp_inside(egui_pos))
        };
        // TODO: proper overlay system
        for [start, end] in std::mem::take(&mut self.queued_arrows) {
            (|| {
                let start = project_point(&start)?;
                let end = project_point(&end)?;
                painter.circle_filled(start, 3.0, egui::Color32::WHITE);
                painter.arrow(
                    start,
                    end - start,
                    egui::Stroke::new(3.0, egui::Color32::WHITE),
                );
                Some(())
            })();
        }

        r
    }
}

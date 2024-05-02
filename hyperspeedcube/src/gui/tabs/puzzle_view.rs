use std::sync::Arc;

use bitvec::boxed::BitBox;
use cgmath::{EuclideanSpace, InnerSpace, SquareMatrix};
use egui::NumExt;
use float_ord::FloatOrd;
use hypermath::prelude::*;
use hyperpuzzle::{LayerMask, Puzzle};
use parking_lot::Mutex;

use crate::gfx::*;
use crate::gui::App;
use crate::preferences::Preferences;
use crate::puzzle::{Camera, HoverState, PieceStyleState, PuzzleController, PuzzleViewController};

pub fn show(ui: &mut egui::Ui, app: &mut App, puzzle_view: &Arc<Mutex<Option<PuzzleView>>>) {
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
pub struct PuzzleView {
    pub view_controller: PuzzleViewController,
    /// Puzzle renderer. This is wrapped in an `Arc<Mutex<T>>` because egui
    /// needs access to it during rendering, when we are not in control.
    renderer: Arc<Mutex<PuzzleRenderer>>,

    drag_state: Option<DragType>,
    /// Hover state at the start of the current pointer drag, if any.
    drag_init_hover_state: Option<HoverState>,
    drag_init_pos: Option<egui::Pos2>,

    queued_arrows: Vec<[Vector; 2]>,
    last_click: Option<HoverState>,
}
impl PuzzleView {
    pub(crate) fn new(gfx: &Arc<GraphicsState>, puzzle: &Arc<Puzzle>) -> Self {
        Self::with_controller(gfx, &Arc::new(Mutex::new(PuzzleController::new(puzzle))))
    }
    pub(crate) fn with_controller(
        gfx: &Arc<GraphicsState>,
        controller: &Arc<Mutex<PuzzleController>>,
    ) -> Self {
        let view_controller = PuzzleViewController::new(&controller);
        let puzzle = view_controller.puzzle();
        let renderer = Arc::new(Mutex::new(PuzzleRenderer::new(gfx, &puzzle)));
        Self {
            view_controller,
            renderer,

            drag_state: None,
            drag_init_hover_state: None,
            drag_init_pos: None,

            queued_arrows: vec![],
            last_click: None,
        }
    }

    /// Returns the puzzle controller.
    pub fn controller(&self) -> &Arc<Mutex<PuzzleController>> {
        &self.view_controller.state
    }
    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Arc<Puzzle> {
        Arc::clone(&self.controller().lock().puzzle_type())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
        let puzzle = self.puzzle();
        let view_prefs = prefs.view(&puzzle).clone();

        // Allocate space in the UI.
        let r: egui::Response;
        let target_size: [u32; 2];
        {
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

            r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());
            target_size = [
                pixels_rect.width() as u32 / view_prefs.downscale_rate,
                pixels_rect.height() as u32 / view_prefs.downscale_rate,
            ];
        };

        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
        }

        if r.is_pointer_button_down_on() && ui.input(|input| input.pointer.any_pressed()) {
            self.drag_init_hover_state = self.view_controller.hover_state();
            self.drag_init_pos = r.hover_pos();
        }

        // egui reports `r.dragged()` whenever the mouse is held, even if it
        // didn't move, so we manually keep track of whether the mouse has
        // moved.
        if !r.dragged() {
            self.drag_state = None;
        }
        if r.drag_delta() != egui::Vec2::ZERO && self.drag_state.is_none() {
            let is_primary = ui.input(|input| input.pointer.primary_down());
            if is_primary && self.drag_init_hover_state.is_some() {
                self.drag_state = Some(DragType::PreTwist);
            } else {
                self.drag_state = Some(DragType::ViewRot);
            }
        }

        self.update_drag_state(ui, &r);
        if !matches!(self.drag_state, Some(DragType::Twist { .. })) {
            self.view_controller
                .state
                .lock()
                .set_partial_twist_drag_state(None);
        }

        let view_ctrl = &mut self.view_controller;

        let scroll_delta = ui.input(|input| input.raw_scroll_delta);
        let mut mouse_pos: Option<cgmath::Point2<f32>> = None;
        if r.hovered() && self.drag_state.is_none() {
            let cam = &mut view_ctrl.camera;
            cam.zoom *= (scroll_delta.y / 100.0).exp2();
            cam.zoom = cam.zoom.clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(8.0));
            if let Some(pos) = r.hover_pos() {
                let mouse_pos_ndc = (pos - r.rect.center()) * 2.0 / r.rect.size();
                mouse_pos = Some(cgmath::point2(mouse_pos_ndc.x, -mouse_pos_ndc.y));
            }
        }

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

        let mut renderer = self.renderer.lock();

        if renderer.vertex_3d_positions().is_none() {
            // Redraw each frame until the image is stable and we have
            // computed 3D vertex positions.
            ui.ctx().request_repaint();
        }

        // Update hover state.
        view_ctrl.set_hover_state((|| {
            // IIFE to mimic try_block
            let p = mouse_pos?.to_vec();
            let s = view_ctrl.camera.xy_scale().ok()?;
            let screen_space_mouse_pos = cgmath::point2(p.x / s.x, p.y / s.y);

            let vertex_3d_positions = renderer.vertex_3d_positions()?;

            if self.drag_state.is_some() {
                return None;
            }

            view_ctrl.compute_hover_state(screen_space_mouse_pos, &vertex_3d_positions, prefs)
        })());

        view_ctrl.update_styles(prefs);

        // Check for twist clicks.
        if r.clicked() {
            view_ctrl.do_sticker_click(Sign::Neg);
        }
        if r.secondary_clicked() {
            view_ctrl.do_sticker_click(Sign::Pos);
        }

        let dark_mode = ui.ctx().style().visuals.dark_mode;
        let background_color = crate::util::color_to_u8x3(prefs.styles.background_color(dark_mode));
        let internals_color = crate::util::color_to_u8x3(prefs.styles.internals_color);

        view_ctrl.camera = Camera {
            prefs: view_prefs.clone(),
            target_size,
            rot: view_ctrl.camera.rot.clone(),
            zoom: view_ctrl.camera.zoom,
        };

        let draw_params = DrawParams {
            cam: view_ctrl.camera.clone(),

            mouse_pos: mouse_pos.unwrap_or_else(cgmath::Point2::origin).into(),

            background_color,
            internals_color,
            piece_styles: view_ctrl.styles.values(&prefs.styles),
            piece_transforms: view_ctrl.state.lock().piece_transforms().map_ref(
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

        let project_point = |p: &Vector| {
            let ndc = view_ctrl.camera.project_point(p)?;
            let egui_pos = egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
            Some(r.rect.lerp_inside(egui_pos))
        };
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

        if r.is_pointer_button_down_on() {
            if let Some(hov) = view_ctrl.hover_state() {
                self.last_click = Some(hov);
            }
        }
        if let Some(hov) = &self.last_click {
            if let Some(p) = project_point(&hov.position) {
                painter.circle_filled(p, 3.0, egui::Color32::WHITE);
            }
        }

        r
    }

    fn update_drag_state(&mut self, ui: &egui::Ui, r: &egui::Response) {
        let Some(hover_pos) = r.hover_pos() else {
            return;
        };
        let Some(drag_origin) = self.drag_init_pos else {
            return;
        };

        let puzzle = self.puzzle();
        let cam = &self.view_controller.camera;

        let min_size = r.rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta_this_frame = r.drag_delta() * DRAG_SPEED / min_size.abs();
        let raw_drag_delta = hover_pos - drag_origin;
        // Convert to higher precision before dividing.
        let scaled_drag_x = drag_delta_this_frame.x as Float / cam.zoom.at_least(1.0) as Float;
        let scaled_drag_y = drag_delta_this_frame.y as Float / cam.zoom.at_least(1.0) as Float;

        match &self.drag_state {
            None => (),

            Some(DragType::ViewRot) => {
                let view_ctrl = &mut self.view_controller;

                let mut z_axis = 2;
                if ui.input(|input| input.modifiers.shift) {
                    z_axis += 1;
                }
                if ui.input(|input| input.modifiers.alt) {
                    z_axis += 2;
                };
                let ndim = view_ctrl.puzzle().ndim();
                view_ctrl.camera.rot =
                    pga::Motor::from_angle_in_axis_plane(ndim, 0, z_axis, -scaled_drag_x)
                        * pga::Motor::from_angle_in_axis_plane(ndim, 1, z_axis, scaled_drag_y)
                        * &view_ctrl.camera.rot;
            }

            Some(DragType::PreTwist) => {
                let threshold_sq = crate::TWIST_DRAG_THRESHOLD * crate::TWIST_DRAG_THRESHOLD;
                if raw_drag_delta.length_sq() > threshold_sq {
                    // Begin the twist!
                    match puzzle.ndim() {
                        3 => {
                            if let Some(state) = self.parallel_drag_state(raw_drag_delta, r.rect) {
                                let a = state.start.clone();
                                let b = &state.start + &state.delta;
                                let target = state.normal.cross_product_3d(&state.delta);
                                let axis = puzzle
                                    .axes
                                    .iter()
                                    .filter_map(|(axis, info)| {
                                        let piece = state.initial_hover_state.piece;
                                        let layers = self
                                            .view_controller
                                            .state
                                            .lock()
                                            .puzzle()
                                            .compute_minimum_layer_mask(axis, piece)?;
                                        let score = target.dot(info.vector.normalize()?).abs();
                                        if !is_approx_positive(&score) {
                                            return None;
                                        }
                                        Some((axis, layers, score))
                                    })
                                    .max_by_key(|(_, _, score)| FloatOrd(*score));
                                if let Some((axis, layers, _)) = axis {
                                    self.drag_state = Some(DragType::Twist { axis, layers })
                                } else {
                                    self.drag_state = None;
                                }
                                self.queued_arrows.push([a, b]);
                            }
                        }
                        4 => {}
                        _ => (), // TODO: don't even get this far for other dimensions
                    }
                }
            }
            Some(DragType::Twist { axis, layers }) => {
                match puzzle.ndim() {
                    3 => {
                        if let Some(state) = self.parallel_drag_state(raw_drag_delta, r.rect) {
                            let axis_vector = &self.puzzle().axes[*axis].vector;
                            let Some(v1) = state.normal.cross_product_3d(axis_vector).normalize()
                            else {
                                return;
                            };
                            let Some(v2) = axis_vector.cross_product_3d(&v1).normalize() else {
                                return;
                            };
                            let m = pga::Motor::from_angle_in_normalized_plane(
                                3,
                                &v2,
                                &v1,
                                v1.dot(&state.delta), // TODO: scale by torque (i.e., radius)
                            );
                            self.view_controller
                                .state
                                .lock()
                                .set_partial_twist_drag_state(Some((*axis, *layers, m)));
                        }
                    }
                    4 => {}
                    _ => (), // TODO: don't even get this far for other dimensions
                }
            }
        }
    }

    fn parallel_drag_state(
        &self,
        raw_drag_delta: egui::Vec2,
        rect: egui::Rect,
    ) -> Option<ParallelDrag<'_>> {
        let view_ctrl = &self.view_controller;

        // TODO: why 2.5?
        let delta = raw_drag_delta / rect.size() * egui::vec2(2.5, -2.5) * crate::TWIST_DRAG_SPEED;
        let delta_2d = cgmath::vec2(delta.x, delta.y);
        let initial_hover_state = self.drag_init_hover_state.as_ref()?;
        // Get the 3D position where the drag started.
        let drag_start = &initial_hover_state.position;
        // Get the tangent vectors at that position.
        let [u, v] = [
            &initial_hover_state.u_tangent,
            &initial_hover_state.v_tangent,
        ];
        // Project those tangent vectors onto the screen.
        let u_2d = view_ctrl.camera.project_vector(&drag_start, u)?;
        let v_2d = view_ctrl.camera.project_vector(&drag_start, v)?;
        // Convert the drag delta into the basis formed using the projected
        // tangent vectors, then use that to reconstruct the 3D vector.
        let screen_to_uv = cgmath::Matrix2::from_cols(u_2d, v_2d).invert()?;
        let delta_uv = screen_to_uv * delta_2d;
        let delta_3d = u * delta_uv.x as _ + v * delta_uv.y as _;

        Some(ParallelDrag {
            initial_hover_state,
            normal: u.cross_product_3d(v),
            start: drag_start.clone(),
            delta: match delta_3d.normalize() {
                Some(v) => v * delta_2d.magnitude() as _,
                None => vector![],
            },
        })
    }
}

#[derive(Debug, Clone)]
enum DragType {
    ViewRot,
    PreTwist,
    Twist {
        axis: hyperpuzzle::Axis,
        layers: LayerMask,
    },
}

struct ParallelDrag<'a> {
    initial_hover_state: &'a HoverState,
    normal: Vector,
    start: Vector,
    delta: Vector,
}

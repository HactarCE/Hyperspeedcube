use std::sync::Arc;

use hypermath::prelude::*;
use hyperpuzzle::{PieceMask, Puzzle};
use parking_lot::Mutex;

use crate::gfx::*;
use crate::gui::App;
use crate::preferences::{Preferences, PuzzleViewPreferencesSet};
use crate::puzzle::{DragState, PieceStyleState, PuzzleSimulation, PuzzleView, PuzzleViewInput};

/// Whether to send the mouse position to the GPU. This is useful for debugging
/// purposes, but causes the puzzle to redraw every frame that the mouse moves,
/// even when not necessary.
const SEND_CURSOR_POS: bool = false;

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
        app.set_active_puzzle_view(puzzle_view);
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

    pub wants_focus: bool,
}
impl PuzzleWidget {
    pub(crate) fn new(
        lib: &hyperpuzzle::Library,
        gfx: &Arc<GraphicsState>,
        prefs: &Preferences,
        puzzle_id: &str,
    ) -> Option<Self> {
        let start_time = instant::Instant::now();
        let result = lib.build_puzzle(puzzle_id).take_result_blocking();
        match result {
            Err(e) => {
                log::error!("{e:?}");
                None
            }
            Ok(p) => {
                log::info!("Built {:?} in {:?}", p.name, start_time.elapsed());
                log::info!("Updated active puzzle");
                let sim = &Arc::new(Mutex::new(PuzzleSimulation::new(&p, prefs)));
                Some(Self::with_sim(gfx, prefs, sim))
            }
        }
    }
    pub(crate) fn with_sim(
        gfx: &Arc<GraphicsState>,
        prefs: &Preferences,
        sim: &Arc<Mutex<PuzzleSimulation>>,
    ) -> Self {
        let view = PuzzleView::new(prefs, sim);
        let puzzle = view.puzzle();
        let renderer = Arc::new(Mutex::new(PuzzleRenderer::new(gfx, &puzzle)));
        Self {
            view,
            renderer,

            queued_arrows: vec![],

            wants_focus: false,
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
    /// Returns the view preferences set to use for the puzzle.
    pub fn view_prefs_set(&self) -> PuzzleViewPreferencesSet {
        PuzzleViewPreferencesSet::from_ndim(self.puzzle().ndim())
    }

    /// Reloads the active puzzle. Returns `true` if the reload was successful.
    pub fn reload(&mut self, lib: &hyperpuzzle::Library, prefs: &Preferences) -> bool {
        crate::reload_user_puzzles();
        let current_puzzle = self.puzzle();
        let gfx = Arc::clone(&self.renderer.lock().gfx);
        if let Some(new_puzzle_view) = Self::new(lib, &gfx, prefs, &current_puzzle.id) {
            *self = new_puzzle_view;
            true
        } else {
            false
        }
    }

    /// Draws the puzzle in the UI and handles input.
    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
        let puzzle = self.puzzle();

        // Allocate space in the UI.
        let (egui_rect, target_size) = crate::gui::util::rounded_pixel_rect(
            ui,
            ui.available_rect_before_wrap(),
            self.view.camera.prefs().downscale_rate,
        );
        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());

        // Request focus on click.
        if r.is_pointer_button_down_on() {
            r.request_focus();
            self.wants_focus = true;
        }

        // egui reports `r.dragged()` whenever the mouse is held, even if it
        // didn't move, so we manually keep track of whether the mouse has
        // moved.
        if r.drag_delta() != egui::Vec2::ZERO && self.view.drag_state().is_none() {
            let is_primary = ui.input(|input| input.pointer.primary_down());
            if is_primary && self.view.puzzle_hover_state().is_some() {
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
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta); // TODO: make raw vs. smooth a setting
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
                cam.zoom *= (scroll_delta.y / 500.0).exp2();
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

            if ui.input(|input| input.key_pressed(egui::Key::F5)) {
                if crate::LIBRARY.with(|lib| self.reload(lib, prefs)) {
                    // Don't even try to redraw the puzzle. Just wait for the
                    // next frame.
                    return r;
                }
            }
        }

        let renderer = self.renderer.lock();

        // Redraw each frame until the image is stable and we have computed 3D
        // vertex positions.
        if renderer.puzzle_vertex_3d_positions.get().is_none()
            || renderer.gizmo_vertex_3d_positions.get().is_none()
        {
            ui.ctx().request_repaint();
        }

        self.view.update(PuzzleViewInput {
            cursor_pos,
            target_size,
            puzzle_vertex_3d_positions: renderer.puzzle_vertex_3d_positions.get(),
            gizmo_vertex_3d_positions: renderer.gizmo_vertex_3d_positions.get(),
            prefs,
            exceeded_twist_drag_threshold,
            show_sticker_hover: ui.input(|input| input.modifiers.shift),
        });

        // Check for twist clicks.
        if r.clicked() {
            self.view.do_click_twist(Sign::Neg);
        }
        if r.secondary_clicked() {
            self.view.do_click_twist(Sign::Pos);
        }

        let dark_mode = ui.visuals().dark_mode;
        let background_color = prefs.styles.background_color(dark_mode).rgb;
        let internals_color = prefs.styles.internals_color.rgb;

        let draw_params = DrawParams {
            ndim: puzzle.ndim(),
            cam: self.view.camera.clone(),

            cursor_pos: cursor_pos.map(|p| p.into()).filter(|_| SEND_CURSOR_POS),
            is_dragging_view: match self.view.drag_state() {
                Some(DragState::ViewRot { .. }) => true,
                Some(DragState::Canceled | DragState::PreTwist | DragState::Twist) | None => false,
            },

            background_color,
            internals_color,
            sticker_colors: {
                puzzle
                    .colors
                    .list
                    .iter()
                    .map(|(id, color_info)| {
                        self.view
                            .colors
                            .value
                            .get(&color_info.name)
                            .and_then(|c| Some(prefs.colors.get_color(c.as_ref()?)?.rgb))
                            .unwrap_or_else(|| sample_rainbow(id.0 as usize, puzzle.colors.len()))
                    })
                    .collect()
            },
            piece_styles: self.view.styles.values(&prefs.styles),
            piece_transforms: self.view.sim.lock().piece_transforms().map_ref(
                |_piece, transform| transform.euclidean_rotation_matrix().at_ndim(puzzle.ndim()),
            ),
        };

        // Draw puzzle.
        let painter = ui.painter_at(r.rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            r.rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&renderer.gfx),
                renderer: Arc::clone(&self.renderer),
                draw_params,
            },
        ));

        self.queued_arrows.extend(self.view.drag_delta_3d());

        let project_point = |p: &Vector| {
            let ndc = self.view.camera.project_point_to_ndc(p)?;
            let egui_pos = egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
            Some(r.rect.lerp_inside(egui_pos))
        };
        // TODO: proper overlay system
        if cfg!(not(debug_assertions)) || true {
            self.queued_arrows.clear();
        }
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

        let to_egui = |screen_space: cgmath::Vector4<f32>| {
            let ndc = self
                .view
                .camera
                .project_3d_screen_space_to_ndc(screen_space)
                .unwrap_or(cgmath::Point2::new(f32::NAN, f32::NAN));
            r.rect
                .lerp_inside(egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5))
        };

        // Draw gizmos (TODO: move to GPU?)
        let strong_color = egui::Color32::LIGHT_BLUE;
        let weak_color = strong_color.linear_multiply(0.05);
        let stroke_weak = egui::Stroke::new(2.0, weak_color);
        let stroke_strong = egui::Stroke::new(2.0, strong_color);
        let fill = weak_color;
        if let Some(gizmo_vertex_3d_positions) = renderer.gizmo_vertex_3d_positions.get() {
            if let Some(hover) = self.view.gizmo_hover_state() {
                let twist = puzzle.gizmo_twists[hover.gizmo_face];
                let axis = puzzle.twists[twist].axis;
                let other_faces_on_same_gizmo = puzzle
                    .gizmo_twists
                    .iter_filter(|_gizmo_face, &twist| puzzle.twists[twist].axis == axis);

                for face in other_faces_on_same_gizmo {
                    let edge_id_range = &puzzle.mesh.gizmo_edge_ranges[face];
                    for edge_id in edge_id_range.clone() {
                        let edge = puzzle.mesh.edges[edge_id as usize]
                            .map(|i| gizmo_vertex_3d_positions[i as usize]);
                        painter.line_segment(edge.map(to_egui), stroke_weak);
                    }
                }

                let tri_id_range = &puzzle.mesh.gizmo_triangle_ranges[hover.gizmo_face];
                for tri_id in tri_id_range.clone() {
                    let tri = puzzle.mesh.triangles[tri_id as usize]
                        .map(|i| gizmo_vertex_3d_positions[i as usize]);
                    painter.add(egui::Shape::convex_polygon(
                        tri.into_iter().map(to_egui).collect(),
                        fill,
                        egui::Stroke::NONE,
                    ));
                }
                let edge_id_range = &puzzle.mesh.gizmo_edge_ranges[hover.gizmo_face];
                for edge_id in edge_id_range.clone() {
                    let edge = puzzle.mesh.edges[edge_id as usize]
                        .map(|i| gizmo_vertex_3d_positions[i as usize]);
                    painter.line_segment(edge.map(to_egui), stroke_strong);
                }
            }
        }

        // TODO: draw debug plane??
        // let group = hypershape::CoxeterGroup::new_linear(&[5, 3]).unwrap();
        // for mirror in group.mirrors() {
        //     let pole = mirror.hyperplane().unwrap().pole();
        //     let basis =
        //         pga::Blade::from_hyperplane(puzzle.ndim(), &mirror.hyperplane().unwrap()).basis();
        //     basis[0]
        // }

        // (|| {
        //     // TODO: reject polygons whose 3D normal vectors are nearly parallel
        //     //       with the screen.
        //     let [a, b, c, d] = [
        //         project_point(&vector![1.0, -1.0, -1.0])?,
        //         project_point(&vector![1.0, 1.0, -1.0])?,
        //         project_point(&vector![1.0, 1.0, 1.0])?,
        //         project_point(&vector![1.0, -1.0, 1.0])?,
        //     ];
        //     for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
        //         painter.line_segment([p, q]);
        //     }
        //     painter.add(egui::Shape::convex_polygon(
        //         vec![a, b, c, d],
        //         egui::Color32::LIGHT_BLUE.gamma_multiply(0.2),
        //         egui::Stroke::NONE,
        //     ));
        //     Some(())
        // })();

        r
    }
}

fn sample_rainbow(i: usize, n: usize) -> [u8; 3] {
    colorous::RAINBOW.eval_rational(i, n).into_array()
}

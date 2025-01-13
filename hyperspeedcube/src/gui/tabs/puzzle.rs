use std::fmt;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use egui::mutex::RwLock;
use egui::Widget;
use eyre::{bail, OptionExt, Result};
use hyperdraw::*;
use hypermath::prelude::*;
use hyperprefs::{AnimationPreferences, Preferences, PuzzleViewPreferencesSet, StyleColorMode};
use hyperpuzzle::{
    GizmoFace, LayerMask, Puzzle, PuzzleBuildStatus, PuzzleBuildTask, PuzzleResult,
    ScrambleProgress,
};
use hyperpuzzle_log::Solve;
use hyperpuzzle_view::{DragState, HoverMode, PuzzleSimulation, PuzzleView, PuzzleViewInput};
use image::ImageBuffer;
use parking_lot::Mutex;
use web_time::Instant;

use crate::gui::components::color_assignment_popup;
use crate::gui::util::EguiTempValue;
use crate::gui::App;
use crate::L;

/// Whether to send the mouse position to the GPU. This is useful for debugging
/// purposes, but causes the puzzle to redraw every frame that the mouse moves,
/// even when not necessary.
const SEND_CURSOR_POS: bool = false;

/// Whether to show the 3D mouse drag vector on the puzzle. This is useful for
/// debugging purposes.
const SHOW_DRAG_VECTOR: bool = false;

pub fn show(ui: &mut egui::Ui, app: &mut App, puzzle_widget: &Arc<Mutex<PuzzleWidget>>) {
    let (r, changed);
    {
        let mut puzzle_widget_guard = puzzle_widget.lock();
        r = ui
            .scope(|ui| puzzle_widget_guard.ui(ui, &mut app.prefs, &app.animation_prefs.value))
            .response;
        changed = std::mem::take(&mut puzzle_widget_guard.puzzle_changed);
    }

    if changed {
        app.notify_active_puzzle_changed();
    }
}

// TODO: refactor this
fn show_puzzle_load_hint(
    ui: &mut egui::Ui,
    puzzle_widget: &mut PuzzleWidget,
    prefs: &mut Preferences,
) -> egui::Response {
    show_centered_with_sizing_pass(ui, true, true, |ui| {
        ui.spacing_mut().button_padding *= 4.0;
        ui.spacing_mut().item_spacing *= 4.0;
        ui.heading("Load a puzzle");
        show_centered_with_sizing_pass(ui, true, false, |ui| {
            ui.horizontal(|ui| {
                if ui.button("3D Rubik's Cube").clicked() {
                    puzzle_widget.load_puzzle("ft_cube:3", prefs);
                }
                if ui.button("4D Rubik's Cube").clicked() {
                    puzzle_widget.load_puzzle("ft_hypercube:3", prefs);
                }
            });
        });
        if ui.button("See the full list").clicked() {
            todo!("open & focus puzzle list")
        }
    })
    .response
}

fn show_centered_with_sizing_pass<R>(
    ui: &mut egui::Ui,
    horizontal: bool,
    vertical: bool,
    mut f: impl FnMut(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let total_rect = ui.max_rect();
    let mut r = ui.scope_builder(
        egui::UiBuilder::new()
            .layout(egui::Layout::top_down(egui::Align::LEFT))
            .sizing_pass()
            .invisible(),
        &mut f,
    );
    let size = r.response.rect.size();
    if !ui.is_sizing_pass() {
        let mut desired_rect = egui::Rect::from_center_size(total_rect.center(), size);
        let go_back = total_rect.min - desired_rect.min;
        if !horizontal {
            desired_rect.translate(egui::vec2(go_back.x, 0.0));
        }
        if !vertical {
            desired_rect.translate(egui::vec2(0.0, go_back.y));
        }
        r = ui.allocate_new_ui(
            egui::UiBuilder::new()
                .layout(egui::Layout::top_down(egui::Align::Center))
                .max_rect(desired_rect),
            f,
        );
    }
    r
}

pub struct PuzzleWidget {
    contents: PuzzleWidgetContents,
    loading: Option<PuzzleWidgetLoading>,

    gfx: Arc<GraphicsState>,
    egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
    egui_texture_id: Option<egui::TextureId>,

    queued_arrows: Vec<[Vector; 2]>,

    pub wants_focus: bool,
    pub puzzle_changed: bool,
}
impl Drop for PuzzleWidget {
    fn drop(&mut self) {
        if let Some(id) = self.egui_texture_id {
            self.egui_wgpu_renderer.write().free_texture(&id);
        }
    }
}
impl fmt::Debug for PuzzleWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleWidget")
            .field("contents", &self.contents)
            .field("loading", &self.loading)
            .field("egui_texture_id", &self.egui_texture_id)
            .field("queued_arrows", &self.queued_arrows)
            .field("wants_focus", &self.wants_focus)
            .finish_non_exhaustive()
    }
}
impl PuzzleWidget {
    pub(crate) fn new(
        gfx: &Arc<GraphicsState>,
        egui_wgpu_renderer: &Arc<RwLock<eframe::egui_wgpu::Renderer>>,
    ) -> Self {
        Self {
            contents: PuzzleWidgetContents::None,
            loading: None,

            gfx: Arc::clone(gfx),
            egui_wgpu_renderer: Arc::clone(egui_wgpu_renderer),
            egui_texture_id: None,

            queued_arrows: vec![],

            wants_focus: false,
            puzzle_changed: true,
        }
    }

    pub(crate) fn load_puzzle(&mut self, puzzle_id: &str, prefs: &mut Preferences) {
        self.load(puzzle_id.to_owned(), None, prefs);
    }
    pub(crate) fn load_solve(&mut self, solve: Arc<Solve>, prefs: &mut Preferences) {
        let id = solve.puzzle.id.clone();
        self.load(id, Some(solve), prefs);
    }
    fn load(&mut self, puzzle_id: String, solve: Option<Arc<Solve>>, prefs: &mut Preferences) {
        match hyperpuzzle_library::LIBRARY.with(|lib| lib.build_puzzle(&puzzle_id)) {
            PuzzleResult::Ok(puzzle) => match solve {
                Some(solve) => {
                    let thread_handle = std::thread::spawn(move || {
                        Ok(PuzzleSimulation::deserialize(&puzzle, &solve))
                    });
                    self.loading = Some(PuzzleWidgetLoading::LoadingFile {
                        puzzle_id,
                        thread_handle,
                    });
                }
                None => self.set_sim(&Arc::new(Mutex::new(PuzzleSimulation::new(&puzzle))), prefs),
            },
            PuzzleResult::Building { waiter, status } => {
                self.loading = Some(PuzzleWidgetLoading::BuildingPuzzle {
                    puzzle_id,
                    status,
                    solve_to_load: solve,
                });
            }
            PuzzleResult::Err => {
                if self.contents.puzzle_id().as_ref() != Some(&puzzle_id) {
                    let gfx = &self.gfx;
                    let sim = Arc::new(Mutex::new(PuzzleSimulation::new(
                        &hyperpuzzle::PLACEHOLDER_PUZZLE,
                    )));
                    self.contents = PuzzleWidgetContents::Placeholder {
                        puzzle_id: puzzle_id.to_string(),
                        view: PuzzleView::new(gfx, &sim, prefs),
                    };
                    self.puzzle_changed = true;
                }
            }
        }
    }
    fn set_sim(&mut self, sim: &Arc<Mutex<PuzzleSimulation>>, prefs: &mut Preferences) {
        self.contents = PuzzleWidgetContents::Puzzle(PuzzleView::new(&self.gfx, sim, prefs));
        self.loading = None;
        self.puzzle_changed = true;
    }

    pub(crate) fn title(&self) -> String {
        match &self.loading {
            Some(PuzzleWidgetLoading::BuildingPuzzle { puzzle_id, .. })
            | Some(PuzzleWidgetLoading::LoadingFile { puzzle_id, .. }) => {
                L.tabs.titles.puzzle.loading.with(puzzle_id)
            }
            None => match &self.contents {
                PuzzleWidgetContents::None => L.tabs.titles.puzzle.empty.to_string(),
                PuzzleWidgetContents::Puzzle(puzzle_view) => puzzle_view.puzzle().name.clone(),
                PuzzleWidgetContents::Placeholder { puzzle_id, .. } => {
                    L.tabs.titles.puzzle.error.with(puzzle_id)
                }
            },
        }
    }

    pub fn view(&self) -> Option<&PuzzleView> {
        match &self.contents {
            PuzzleWidgetContents::Puzzle(puzzle_view) => Some(puzzle_view),
            _ => None,
        }
    }
    pub fn view_mut(&mut self) -> Option<&mut PuzzleView> {
        match &mut self.contents {
            PuzzleWidgetContents::Puzzle(puzzle_view) => Some(puzzle_view),
            _ => None,
        }
    }
    /// Returns the puzzle simulation.
    pub fn sim(&self) -> Option<Arc<Mutex<PuzzleSimulation>>> {
        Some(Arc::clone(&self.view()?.sim))
    }
    /// Returns the puzzle type.
    pub fn puzzle(&self) -> Option<Arc<Puzzle>> {
        Some(self.view()?.puzzle())
    }

    /// Reloads all files and the current puzzle.
    pub fn reload(&mut self, prefs: &mut Preferences) {
        // TODO: keybind should be global, not just in puzzle view
        hyperpuzzle_library::load_puzzles();
        if let Some(id) = self.contents.puzzle_id() {
            self.load_puzzle(&id, prefs);
        }
    }

    /// Draws the puzzle in the UI and handles input.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        prefs: &mut Preferences,
        animation: &AnimationPreferences,
    ) {
        let scramble_progress = self.sim().and_then(|sim| sim.lock().scramble_progress());
        let loading_something = self.loading.is_some() || scramble_progress.is_some();

        let rect = ui.available_rect_before_wrap();

        ui.scope(|ui| {
            if loading_something {
                ui.disable();
                ui.multiply_opacity(0.5);
            }
            self.show_puzzle_view(ui, prefs, animation);
        });

        let mut loading_header = None;
        let mut loading_progress = None;
        if let Some(loading) = self.loading.take() {
            crate::gui::util::centered_popup_area(ui.ctx(), rect, unique_id!(), |ui| {
                match loading {
                    PuzzleWidgetLoading::BuildingPuzzle {
                        puzzle_id,
                        status,
                        solve_to_load,
                    } => {
                        let task = match status {
                            Some(s) => s.task,
                            None => Default::default(),
                        };
                        loading_header = Some(match task {
                            PuzzleBuildTask::Initializing => L.puzzle_view.initializing,
                            PuzzleBuildTask::GeneratingSpec => L.puzzle_view.generating_spec,
                            PuzzleBuildTask::Building => L.puzzle_view.building,
                            PuzzleBuildTask::Finalizing => L.puzzle_view.finalizing,
                        });
                        self.load(puzzle_id, solve_to_load, prefs);
                        ui.ctx().request_repaint_after_secs(0.2); // try again soon
                    }
                    PuzzleWidgetLoading::LoadingFile {
                        puzzle_id,
                        thread_handle,
                    } => {
                        loading_header = Some(L.puzzle_view.loading_log);
                        match thread_handle.is_finished() {
                            true => {
                                match thread_handle.join() {
                                    Ok(result) => match result {
                                        Ok(sim) => self.set_sim(&Arc::new(Mutex::new(sim)), prefs),
                                        Err(e) => self.loading = None, // TODO: report error
                                    },
                                    Err(e) => self.loading = None, // TODO: report error
                                }
                            }
                            false => {
                                self.loading = Some(PuzzleWidgetLoading::LoadingFile {
                                    puzzle_id,
                                    thread_handle,
                                });
                            }
                        }
                    }
                }
                ui.ctx().request_repaint();
            });
        } else if let Some(scramble_progress) = scramble_progress {
            crate::gui::util::centered_popup_area(ui.ctx(), rect, unique_id!(), |ui| {
                let (done, total) = scramble_progress.fraction();
                loading_header = Some(L.puzzle_view.scrambling);
                loading_progress = Some(done as f32 / total as f32);
                ui.ctx().request_repaint();
            });
        } else if matches!(self.contents, PuzzleWidgetContents::None) {
            show_puzzle_load_hint(ui, self, prefs);
        }

        if let Some(header_text) = loading_header {
            crate::gui::util::centered_popup_area(ui.ctx(), rect, unique_id!(), |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.heading(header_text);
                });
                if let Some(progress) = loading_progress {
                    egui::ProgressBar::new(progress)
                        .desired_width(200.0) // reasonable width
                        .show_percentage()
                        .ui(ui);
                }
            });
        }
    }

    fn show_puzzle_view(
        &mut self,
        ui: &mut egui::Ui,
        prefs: &mut Preferences,
        animation: &AnimationPreferences,
    ) {
        let Some(view) = self.contents.as_view_mut() else {
            return;
        };
        let puzzle = view.puzzle();

        // Allocate space in the UI.
        let (egui_rect, target_size) = crate::gui::util::rounded_pixel_rect(
            ui,
            ui.available_rect_before_wrap(),
            view.camera.prefs().downscale_rate,
        );
        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());

        // egui reports `r.dragged()` whenever the mouse is held, even if it
        // didn't move, so we manually keep track of whether the mouse has
        // moved.
        if r.drag_delta() != egui::Vec2::ZERO && view.drag_state().is_none() {
            let is_primary = ui.input(|input| input.pointer.primary_down());
            let puzzle_supports_drag_twists = puzzle.ndim() == 3;
            if is_primary && puzzle_supports_drag_twists && view.puzzle_hover_state().is_some() {
                view.set_drag_state(DragState::PreTwist);
            } else {
                view.set_drag_state(DragState::ViewRot { z_axis: 2 });
            }
        }
        // Confirm drag on mouse button release.
        if !r.dragged() {
            view.confirm_drag();
        }
        // Cancel drag on ESC key press.
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            view.cancel_drag();
        }

        let modifiers = ui.input(|input| input.modifiers);

        // Change which axis we're rotating depending on modifiers.
        if matches!(view.drag_state(), Some(DragState::ViewRot { .. })) {
            let mut z_axis = 2;
            if modifiers.shift {
                z_axis += 1;
            }
            if modifiers.alt {
                z_axis += 2;
            }
            if modifiers.ctrl {
                z_axis += 4;
            }
            view.set_drag_state(DragState::ViewRot { z_axis });
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
                let s = view.camera.xy_scale().ok()?;
                Some(cgmath::point2(ndc.x / s.x, ndc.y / s.y))
            })();

            if view.drag_state().is_none() {
                // Adjust camera zoom using scroll wheel.
                let cam = &mut view.camera;
                cam.zoom *= (scroll_delta.y / 500.0).exp2();
                cam.zoom = cam.zoom.clamp(2.0_f32.powi(-6), 2.0_f32.powi(8));
            }
        }

        if r.has_focus() && ui.input(|input| input.key_pressed(egui::Key::F5)) {
            self.reload(prefs);
            // Don't even try to redraw the puzzle. Just wait for the next
            // frame.
            return;
        }

        // Redraw each frame until the image is stable and we have computed 3D
        // vertex positions.
        if view.renderer.puzzle_vertex_3d_positions.get().is_none()
            || view.renderer.gizmo_vertex_3d_positions.get().is_none()
        {
            ui.ctx().request_repaint();
        }

        view.update(
            PuzzleViewInput {
                cursor_pos,
                target_size,
                puzzle_vertex_3d_positions: view.renderer.puzzle_vertex_3d_positions.get(),
                gizmo_vertex_3d_positions: view.renderer.gizmo_vertex_3d_positions.get(),
                exceeded_twist_drag_threshold,
                hover_mode: match ui.input(|input| input.modifiers.shift) {
                    true => Some(HoverMode::Piece),
                    false => Some(HoverMode::TwistGizmo),
                },
            },
            prefs,
            animation,
        );

        // Click = twist
        let mut layers = LayerMask::EMPTY;
        for (i, k) in [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
            egui::Key::Num7,
            egui::Key::Num8,
            egui::Key::Num9,
            egui::Key::Num0,
        ]
        .into_iter()
        .enumerate()
        {
            if ui.input(|input| input.key_down(k)) {
                layers |= LayerMask::from(i as u8);
            }
        }
        if layers == LayerMask::EMPTY {
            layers = LayerMask::default();
        }
        if r.clicked() && modifiers.is_none() {
            view.do_click_twist(layers, Sign::Neg);
        }
        if r.secondary_clicked() && modifiers.is_none() {
            view.do_click_twist(layers, Sign::Pos);
        }

        // Ctrl+shift+click = edit sticker color
        let editing_color = EguiTempValue::new(ui);
        let mut is_first_frame = false;
        if r.secondary_clicked() && modifiers.command && modifiers.shift && !modifiers.alt {
            if let Some(hov) = view.puzzle_hover_state() {
                if let Some(sticker) = hov.sticker {
                    ui.memory_mut(|mem| mem.open_popup(editing_color.id));
                    editing_color.set(Some(puzzle.stickers[sticker].color));
                    is_first_frame = true;
                }
            }
        }
        if ui.memory(|mem| mem.is_popup_open(editing_color.id)) {
            let mut area = egui::Area::new(editing_color.id.with("area"))
                .order(egui::Order::Middle)
                .constrain_to(ui.ctx().available_rect())
                .movable(true);
            if let Some(pos) = r.interact_pointer_pos().filter(|_| is_first_frame) {
                area = area.current_pos(pos);
            }
            let area_response = area.show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    color_assignment_popup(ui, view, &prefs.color_palette, editing_color.get());
                });
            });

            // Allow drags but not clicks
            let any_cursor_input_outside_puzzle =
                crate::gui::util::clicked_elsewhere(ui, &area_response.response)
                    && crate::gui::util::clicked_elsewhere(ui, &r);
            let any_click_inside_puzzle =
                r.clicked() || r.secondary_clicked() || r.middle_clicked();
            let clicked_elsewhere = any_cursor_input_outside_puzzle || any_click_inside_puzzle;
            if (clicked_elsewhere && !is_first_frame)
                || ui.input(|input| input.key_pressed(egui::Key::Escape))
            {
                ui.memory_mut(|mem| mem.close_popup());
            }
        }

        // Ensure that the color scheme is valid. Ignore whether it actually got
        // modified.
        let _ = prefs
            .color_palette
            .ensure_color_scheme_is_valid_for_color_system(&mut view.colors.value, &puzzle.colors);

        let color_map = view.temp_colors.as_ref().unwrap_or(&view.colors.value);
        let sticker_colors = puzzle
            .colors
            .list
            .iter_values()
            .map(|color_info| prefs.color_palette.get(color_map.get(&color_info.name)?))
            .map(|maybe_rgb| maybe_rgb.unwrap_or_default().rgb)
            .collect();
        view.temp_colors = None; // Remove temporary colors

        let draw_params = DrawParams {
            ndim: puzzle.ndim(),
            cam: view.camera.clone(),

            cursor_pos: cursor_pos.filter(|_| SEND_CURSOR_POS),
            is_dragging_view: match view.drag_state() {
                Some(DragState::ViewRot { .. }) => true,
                Some(DragState::Canceled | DragState::PreTwist | DragState::Twist) | None => false,
            },

            internals_color: prefs.styles.internals_color.rgb,
            sticker_colors,
            piece_styles: view.styles.values(prefs),
            piece_transforms: view
                .sim
                .lock()
                .piece_transforms()
                .map_ref(|_piece, transform| {
                    transform.euclidean_rotation_matrix().at_ndim(puzzle.ndim())
                }),
        };

        if draw_params.any_animated() {
            ui.ctx().request_repaint();
        }

        // Draw puzzle.
        let painter = ui.painter_at(r.rect);
        let dark_mode = ui.visuals().dark_mode;
        let background_color = prefs.background_color(dark_mode).to_egui_color32();
        ui.painter().rect_filled(r.rect, 0.0, background_color);

        match self.update_puzzle_texture(&draw_params) {
            Ok(texture_id) => egui::Image::new((texture_id, r.rect.size())).paint_at(ui, r.rect),
            Err(e) => log::error!("{e}"),
        }
        // Reborrow after calling `self.update_puzzle_texture()` method.
        let Some(view) = self.contents.as_view_mut() else {
            return;
        };

        if SHOW_DRAG_VECTOR {
            self.queued_arrows.extend(view.drag_delta_3d());
        }

        let project_point = |p: &Vector| {
            let ndc = view.camera.project_point_to_ndc(p)?;
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

        let to_egui = |screen_space: cgmath::Vector4<f32>| {
            let ndc = view
                .camera
                .project_3d_screen_space_to_ndc(screen_space)
                .unwrap_or(cgmath::Point2::new(f32::NAN, f32::NAN));
            r.rect
                .lerp_inside(egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5))
        };

        // Draw gizmos (TODO: move to GPU?)
        if let Some(gizmo_vertex_3d_positions) = view.renderer.gizmo_vertex_3d_positions.get() {
            if let Some(axis) = view.temp_gizmo_highlight.take() {
                for (gizmo_face, &twist) in &puzzle.gizmo_twists {
                    if puzzle.twists[twist].axis == axis {
                        show_gizmo_face(
                            &puzzle,
                            gizmo_face,
                            &gizmo_vertex_3d_positions,
                            &painter,
                            to_egui,
                            false,
                        );
                    }
                }
            } else if let Some(hover) = view.gizmo_hover_state().filter(|_| view.show_gizmo_hover) {
                show_gizmo_face(
                    &puzzle,
                    hover.gizmo_face,
                    &gizmo_vertex_3d_positions,
                    &painter,
                    to_egui,
                    true,
                );
            }
        }

        egui::Area::new(unique_id!())
            .constrain_to(egui_rect)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                ui.set_width(egui_rect.width());
                ui.label(format!("Solved: {}", view.sim.lock().is_solved()));
            });

        // TODO: draw debug plane??
        // let group = hypershape::CoxeterGroup::new_linear(&[5, 3]).unwrap();
        // for mirror in group.mirrors() {
        //     let pole = mirror.hyperplane().unwrap().pole();
        //     let basis =
        //         pga::Blade::from_hyperplane(puzzle.ndim(),
        // &mirror.hyperplane().unwrap()).basis();     basis[0]
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

        // Request focus on click.
        if r.is_pointer_button_down_on() {
            r.request_focus(); // TODO: what does this do
            self.wants_focus = true;
        }
    }

    fn update_puzzle_texture(&mut self, draw_params: &DrawParams) -> Result<egui::TextureId> {
        let renderer = &mut self
            .contents
            .as_view_mut()
            .ok_or_eyre("no puzzle view")?
            .renderer;

        let output_texture = &renderer.draw_puzzle(draw_params)?.texture;

        // egui expects sRGB colors in the shader, so we have to read the
        // sRGB texture as though it were linear to prevent the GPU from
        // doing gamma conversion.
        let texture_view = output_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(output_texture.format().remove_srgb_suffix()),
            ..Default::default()
        });
        let mut egui_wgpu_renderer = self.egui_wgpu_renderer.write();
        let gfx = &renderer.gfx;
        match self.egui_texture_id {
            Some(egui_texture_id) => {
                egui_wgpu_renderer.update_egui_texture_from_wgpu_texture(
                    &gfx.device,
                    &texture_view,
                    renderer.filter_mode,
                    egui_texture_id,
                );
                Ok(egui_texture_id)
            }
            None => {
                let egui_texture_id = egui_wgpu_renderer.register_native_texture(
                    &gfx.device,
                    &texture_view,
                    renderer.filter_mode,
                );
                self.egui_texture_id = Some(egui_texture_id);
                Ok(egui_texture_id)
            }
        }
    }
}

fn show_gizmo_face(
    puzzle: &Puzzle,
    gizmo_face: GizmoFace,
    gizmo_vertex_3d_positions: &[cgmath::Vector4<f32>],
    painter: &egui::Painter,
    project_to_egui: impl Fn(cgmath::Vector4<f32>) -> egui::Pos2,
    show_other_faces_on_same_gizmo: bool,
) {
    let strong_color = egui::Color32::LIGHT_BLUE;
    let weak_color = strong_color.linear_multiply(0.05);
    let stroke_weak = egui::Stroke::new(2.0, weak_color);
    let stroke_strong = egui::Stroke::new(2.0, strong_color);
    let fill = weak_color;

    let twist = puzzle.gizmo_twists[gizmo_face];
    let axis = puzzle.twists[twist].axis;
    let other_faces_on_same_gizmo = puzzle
        .gizmo_twists
        .iter_filter(|_gizmo_face, &twist| puzzle.twists[twist].axis == axis);

    if show_other_faces_on_same_gizmo {
        for face in other_faces_on_same_gizmo {
            let edge_id_range = &puzzle.mesh.gizmo_edge_ranges[face]; // TODO: fix crash here
            for edge_id in edge_id_range.clone() {
                let edge = puzzle.mesh.edges[edge_id as usize]
                    .map(|i| gizmo_vertex_3d_positions[i as usize]);
                painter.line_segment(edge.map(&project_to_egui), stroke_weak);
            }
        }
    }

    let tri_id_range = &puzzle.mesh.gizmo_triangle_ranges[gizmo_face];
    for tri_id in tri_id_range.clone() {
        let tri =
            puzzle.mesh.triangles[tri_id as usize].map(|i| gizmo_vertex_3d_positions[i as usize]);
        painter.add(egui::Shape::convex_polygon(
            tri.into_iter().map(&project_to_egui).collect(),
            fill,
            egui::Stroke::NONE,
        ));
    }
    let edge_id_range = &puzzle.mesh.gizmo_edge_ranges[gizmo_face];
    for edge_id in edge_id_range.clone() {
        let edge =
            puzzle.mesh.edges[edge_id as usize].map(|i| gizmo_vertex_3d_positions[i as usize]);
        painter.line_segment(edge.map(&project_to_egui), stroke_strong);
    }
}

#[derive(Debug, Default)]
pub enum PuzzleWidgetContents {
    /// No puzzle view.
    #[default]
    None,
    /// Ordinary puzzle view.
    Puzzle(PuzzleView),
    /// Placeholder puzzle view, with the ID of the puzzle that tried to load.
    Placeholder { puzzle_id: String, view: PuzzleView },
}
impl PuzzleWidgetContents {
    fn as_view(&self) -> Option<&PuzzleView> {
        match self {
            PuzzleWidgetContents::None => None,
            PuzzleWidgetContents::Puzzle(view) => Some(view),
            PuzzleWidgetContents::Placeholder { view, .. } => Some(view),
        }
    }
    fn as_view_mut(&mut self) -> Option<&mut PuzzleView> {
        match self {
            PuzzleWidgetContents::None => None,
            PuzzleWidgetContents::Puzzle(view) => Some(view),
            PuzzleWidgetContents::Placeholder { view, .. } => Some(view),
        }
    }
    fn puzzle_id(&self) -> Option<String> {
        match self {
            PuzzleWidgetContents::None => None,
            PuzzleWidgetContents::Puzzle(view) => Some(view.puzzle().id.clone()),
            PuzzleWidgetContents::Placeholder { puzzle_id, .. } => Some(puzzle_id.clone()),
        }
    }
}

#[derive(Debug)]
pub enum PuzzleWidgetLoading {
    /// Waiting for a puzzle to build.
    BuildingPuzzle {
        puzzle_id: String,
        status: Option<PuzzleBuildStatus>,
        solve_to_load: Option<Arc<Solve>>,
    },
    /// Waiting for a log file to load.
    LoadingFile {
        puzzle_id: String,
        thread_handle: JoinHandle<Result<PuzzleSimulation, ()>>,
    },
}

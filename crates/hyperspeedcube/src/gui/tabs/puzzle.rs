use std::fmt;
use std::sync::Arc;
use std::thread::JoinHandle;

use egui::Widget;
use egui::mutex::RwLock;
use eyre::{OptionExt, Result};
use hyperdraw::*;
use hypermath::prelude::*;
use hyperprefs::{AnimationPreferences, ColorScheme, Preferences};
use hyperpuzzle_core::{
    Axis, BuildTask, Color, ColorSystem, GizmoFace, LayerMask, NdEuclidPuzzleGeometry,
    NdEuclidPuzzleStateRenderData, PieceMask, Progress, Puzzle, Redirectable,
};
use hyperpuzzle_log::Solve;
use hyperpuzzle_view::{
    DragState, HoverMode, NdEuclidViewState, PuzzleSimulation, PuzzleView, PuzzleViewInput,
};
use parking_lot::Mutex;

use crate::L;
use crate::gui::App;
use crate::gui::components::color_assignment_popup;
use crate::gui::util::EguiTempValue;

/// Whether to send the mouse position to the GPU. This is useful for debugging
/// purposes, but causes the puzzle to redraw every frame that the mouse moves,
/// even when not necessary.
const SEND_CURSOR_POS: bool = false;

/// Whether to show the 3D mouse drag vector on the puzzle. This is useful for
/// debugging purposes.
const SHOW_DRAG_VECTOR: bool = false;

pub fn show(ui: &mut egui::Ui, app: &mut App, puzzle_widget: &Arc<Mutex<PuzzleWidget>>) {
    let changed;
    {
        let mut puzzle_widget_guard = puzzle_widget.lock();
        puzzle_widget_guard.ui(ui, &mut app.prefs, &app.animation_prefs.value);
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
            desired_rect = desired_rect.translate(egui::vec2(go_back.x, 0.0));
        }
        if !vertical {
            desired_rect = desired_rect.translate(egui::vec2(0.0, go_back.y));
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
        let cache_entry = hyperpuzzle::catalog().build_puzzle(&puzzle_id);
        let cache_entry_guard = cache_entry.lock();
        match &*cache_entry_guard {
            hyperpuzzle_core::CacheEntry::NotStarted => {
                self.loading = Some(PuzzleWidgetLoading::BuildingPuzzle {
                    puzzle_id,
                    progress: None,
                    solve_to_load: solve,
                });
            }
            hyperpuzzle_core::CacheEntry::Building { progress, .. } => {
                self.loading = Some(PuzzleWidgetLoading::BuildingPuzzle {
                    puzzle_id,
                    progress: Some(Arc::clone(progress)),
                    solve_to_load: solve,
                });
            }
            hyperpuzzle_core::CacheEntry::Ok(Redirectable::Redirect(new_id)) => {
                self.loading = Some(PuzzleWidgetLoading::BuildingPuzzle {
                    puzzle_id: new_id.clone(),
                    progress: None,
                    solve_to_load: solve,
                });
            }
            hyperpuzzle_core::CacheEntry::Ok(Redirectable::Direct(puzzle)) => match solve {
                Some(solve) => {
                    let puzzle = Arc::clone(puzzle);
                    let thread_handle = std::thread::spawn(move || {
                        Ok(PuzzleSimulation::deserialize(&puzzle, &solve))
                    });
                    self.loading = Some(PuzzleWidgetLoading::LoadingFile {
                        puzzle_id,
                        thread_handle,
                    });
                }
                None => self.set_sim(&Arc::new(Mutex::new(PuzzleSimulation::new(puzzle))), prefs),
            },
            hyperpuzzle_core::CacheEntry::Err(_) => {
                // The error should've already been reported;
                // we don't need to report it every frame.
                if self.contents.puzzle_id().as_ref() != Some(&puzzle_id) {
                    let gfx = &self.gfx;
                    let sim = Arc::new(Mutex::new(PuzzleSimulation::new(
                        &hyperpuzzle_core::PLACEHOLDER_PUZZLE,
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
                PuzzleWidgetContents::Puzzle(puzzle_view) => puzzle_view.puzzle().meta.name.clone(),
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
        // TODO: make sure that the old catalog actually gets dropped
        hyperpuzzle::load_global_catalog();
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
                        progress: status,
                        solve_to_load,
                    } => {
                        let task = match status {
                            Some(s) => s.lock().task,
                            None => Default::default(),
                        };
                        loading_header = Some(match task {
                            BuildTask::Initializing => L.puzzle_view.initializing,
                            BuildTask::GeneratingSpec => L.puzzle_view.generating_spec,
                            BuildTask::BuildingColors => L.puzzle_view.building_colors,
                            BuildTask::BuildingPuzzle => L.puzzle_view.building_puzzle,
                            BuildTask::Finalizing => L.puzzle_view.finalizing,
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

        let (r, target_size) = allocate_puzzle_response(ui, view.downscale_rate());

        if r.has_focus() && ui.input(|input| input.key_pressed(egui::Key::F5)) {
            self.reload(prefs);
            // Don't even try to redraw the puzzle. Just wait for the next
            // frame.
            return;
        }

        let exceeded_twist_drag_threshold = ui
            .input(|input| {
                let delta = input.pointer.press_origin()? - input.pointer.interact_pos()?;
                Some(delta.length() >= crate::TWIST_DRAG_THRESHOLD)
            })
            .unwrap_or(false);

        // Compute NDC cursor position.
        let mut ndc_cursor_pos: Option<egui::Vec2> =
            r.hover_pos().or(r.interact_pointer_pos()).map(|egui_pos| {
                // Convert to normalized device coordinates (-1 to +1).
                let mut ndc = (egui_pos - r.rect.center()) * 2.0 / r.rect.size();
                ndc.y = -ndc.y;
                ndc
            });

        let input = PuzzleViewInput {
            ndc_cursor_pos: ndc_cursor_pos.map(|pos| pos.into()),
            target_size,
            is_dragging: r.dragged(),
            exceeded_twist_drag_threshold,
            hover_mode: match ui.input(|input| input.modifiers.shift) {
                true => Some(HoverMode::Piece),
                false => Some(HoverMode::TwistGizmo),
            },
        };
        view.update(input, prefs, animation);

        let sim = Arc::clone(&view.sim);

        let color_map = view.temp_colors.as_ref().unwrap_or(&view.colors.value);
        let sticker_colors = puzzle
            .colors
            .list
            .iter_values()
            .map(|color_info| prefs.color_palette.get(color_map.get(&color_info.name)?))
            .map(|maybe_rgb| maybe_rgb.unwrap_or_default().rgb)
            .collect();
        view.temp_colors = None; // Remove temporary colors

        let piece_styles = view.styles.values(prefs);

        let show_gizmo_hover = view.show_gizmo_hover;

        let temp_gizmo_highlight = view.temp_gizmo_highlight.take();

        let response;
        if let Some(nd_euclid) = view.nd_euclid_mut() {
            response = Some(show_nd_euclid_puzzle_view(
                ui,
                &r,
                prefs,
                nd_euclid,
                &sim,
                sticker_colors,
                piece_styles,
                show_gizmo_hover,
                temp_gizmo_highlight,
                &mut self.queued_arrows,
            ));
        } else {
            response = None;
        }

        if let Some(response) = response {
            // Color edit popup
            show_color_edit_popup(ui, &r, response.color_to_edit, view, prefs);

            if let Some(texture_view) = response.texture_view {
                register_or_update_egui_texture(
                    &self.gfx.device,
                    texture_view,
                    response.filter_mode,
                    &mut self.egui_texture_id,
                    &mut self.egui_wgpu_renderer.write(),
                );
            }
            if let Some(texture_id) = self.egui_texture_id {
                egui::Image::new((texture_id, r.rect.size())).paint_at(ui, r.rect);
            }
        }

        egui::Area::new(unique_id!())
            .constrain_to(r.rect)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                ui.set_width(r.rect.width());
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
}

fn register_or_update_egui_texture(
    device: &wgpu::Device,
    texture_view: wgpu::TextureView,
    filter_mode: wgpu::FilterMode,
    cached_egui_texture_id: &mut Option<egui::TextureId>,
    egui_wgpu_renderer: &mut eframe::egui_wgpu::Renderer,
) {
    match *cached_egui_texture_id {
        Some(egui_texture_id) => egui_wgpu_renderer.update_egui_texture_from_wgpu_texture(
            device,
            &texture_view,
            filter_mode,
            egui_texture_id,
        ),
        None => {
            *cached_egui_texture_id = Some(egui_wgpu_renderer.register_native_texture(
                device,
                &texture_view,
                filter_mode,
            ));
        }
    }
}

fn show_gizmo_face(
    puzzle: &Puzzle,
    geom: &NdEuclidPuzzleGeometry,
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

    let twist = geom.gizmo_twists[gizmo_face];
    let axis = puzzle.twists[twist].axis;
    let other_faces_on_same_gizmo = geom
        .gizmo_twists
        .iter_filter(|_gizmo_face, &twist| puzzle.twists[twist].axis == axis);

    if show_other_faces_on_same_gizmo {
        for face in other_faces_on_same_gizmo {
            let edge_id_range = &geom.mesh.gizmo_edge_ranges[face]; // TODO: fix crash here
            for edge_id in edge_id_range.clone() {
                let edge = geom.mesh.edges[edge_id as usize]
                    .map(|i| gizmo_vertex_3d_positions[i as usize]);
                painter.line_segment(edge.map(&project_to_egui), stroke_weak);
            }
        }
    }

    let tri_id_range = &geom.mesh.gizmo_triangle_ranges[gizmo_face];
    for tri_id in tri_id_range.clone() {
        let tri =
            geom.mesh.triangles[tri_id as usize].map(|i| gizmo_vertex_3d_positions[i as usize]);
        painter.add(egui::Shape::convex_polygon(
            tri.into_iter().map(&project_to_egui).collect(),
            fill,
            egui::Stroke::NONE,
        ));
    }
    let edge_id_range = &geom.mesh.gizmo_edge_ranges[gizmo_face];
    for edge_id in edge_id_range.clone() {
        let edge = geom.mesh.edges[edge_id as usize].map(|i| gizmo_vertex_3d_positions[i as usize]);
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
            PuzzleWidgetContents::Puzzle(view) => Some(view.puzzle().meta.id.clone()),
            PuzzleWidgetContents::Placeholder { puzzle_id, .. } => Some(puzzle_id.clone()),
        }
    }
}

#[derive(Debug)]
pub enum PuzzleWidgetLoading {
    /// Waiting for a puzzle to build.
    BuildingPuzzle {
        puzzle_id: String,
        progress: Option<Arc<Mutex<Progress>>>,
        solve_to_load: Option<Arc<Solve>>,
    },
    /// Waiting for a log file to load.
    LoadingFile {
        puzzle_id: String,
        thread_handle: JoinHandle<Result<PuzzleSimulation, ()>>,
    },
}

fn show_nd_euclid_puzzle_view(
    ui: &mut egui::Ui,
    r: &egui::Response,
    prefs: &mut Preferences,
    nd_euclid: &mut NdEuclidViewState,
    sim: &Arc<Mutex<PuzzleSimulation>>,
    sticker_colors: Vec<[u8; 3]>,
    piece_styles: Vec<(PieceStyleValues, PieceMask)>,
    show_gizmo_hover: bool,
    temp_gizmo_highlight: Option<Axis>,
    queued_arrows: &mut Vec<[Vector; 2]>,
) -> PuzzleViewResponse {
    let mut ret = PuzzleViewResponse::default();

    let puzzle = Arc::clone(sim.lock().puzzle_type());
    let geom = Arc::clone(&nd_euclid.geom);
    let ndim = geom.ndim();

    if r.hovered() || r.is_pointer_button_down_on() {
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta); // TODO: make raw vs. smooth a setting
        if nd_euclid.drag_state.is_none() {
            // Adjust camera zoom using scroll wheel.
            let cam = &mut nd_euclid.camera;
            cam.zoom *= (scroll_delta.y / 500.0).exp2();
            cam.zoom = cam.zoom.clamp(2.0_f32.powi(-6), 2.0_f32.powi(8));
        }
    }

    // egui reports `r.dragged()` whenever the mouse is held, even if it
    // didn't move, so we manually keep track of whether the mouse has
    // moved.
    if r.drag_delta() != egui::Vec2::ZERO && nd_euclid.drag_state.is_none() {
        let is_primary = ui.input(|input| input.pointer.primary_down());
        let puzzle_supports_drag_twists = ndim == 3;
        if is_primary && puzzle_supports_drag_twists && nd_euclid.puzzle_hover_state.is_some() {
            nd_euclid.drag_state = Some(DragState::PreTwist);
        } else {
            nd_euclid.drag_state = Some(DragState::ViewRot { z_axis: 2 });
        }
    }
    // Confirm drag on mouse button release.
    if !r.dragged() {
        nd_euclid.confirm_drag(sim);
    }
    // Cancel drag on ESC key press.
    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        nd_euclid.cancel_drag(sim);
    }

    let modifiers = ui.input(|input| input.modifiers);

    // Change which axis we're rotating depending on modifiers.
    if matches!(nd_euclid.drag_state, Some(DragState::ViewRot { .. })) {
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
        nd_euclid.drag_state = Some(DragState::ViewRot { z_axis });
    }

    // Redraw each frame until the image is stable and we have computed 3D
    // vertex positions.
    let renderer = &nd_euclid.renderer;
    if renderer.puzzle_vertex_3d_positions.get().is_none()
        || renderer.gizmo_vertex_3d_positions.get().is_none()
    {
        ui.ctx().request_repaint();
    }

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
        nd_euclid.do_click_twist(&mut sim.lock(), layers, Sign::Neg);
    }
    if r.secondary_clicked() && modifiers.is_none() {
        nd_euclid.do_click_twist(&mut sim.lock(), layers, Sign::Pos);
    }

    // Ctrl+shift+click = edit sticker color
    if r.secondary_clicked() && modifiers.command && modifiers.shift && !modifiers.alt {
        if let Some(hov) = nd_euclid.puzzle_hover_state() {
            if let Some(sticker) = hov.sticker {
                ret.color_to_edit = Some(puzzle.stickers[sticker].color);
            }
        }
    }

    let cam = nd_euclid.transient_camera(sim);
    let effects = sim.lock().special_effects();

    let piece_transforms;
    {
        let sim = sim.lock();
        let render_data = sim.unwrap_render_data::<NdEuclidPuzzleStateRenderData>();
        piece_transforms = render_data
            .piece_transforms
            .map_ref(|_piece, transform| transform.euclidean_rotation_matrix().at_ndim(ndim));
    }

    let mut draw_params = DrawParams {
        ndim,
        cam,

        cursor_pos: nd_euclid.cursor_pos.filter(|_| SEND_CURSOR_POS),
        is_dragging_view: match nd_euclid.drag_state {
            Some(DragState::ViewRot { .. }) => true,
            Some(DragState::Canceled | DragState::PreTwist | DragState::Twist) | None => false,
        },

        internals_color: prefs.styles.internals_color.rgb,
        sticker_colors,
        piece_styles,
        piece_transforms,

        effects,
    };

    if draw_params.any_animated() {
        ui.ctx().request_repaint();
    }

    // Draw puzzle.
    let painter = ui.painter_at(r.rect);
    let dark_mode = ui.visuals().dark_mode;
    let background_color = prefs.background_color(dark_mode).to_egui_color32();
    ui.painter().rect_filled(r.rect, 0.0, background_color);

    match nd_euclid.renderer.draw_puzzle(&draw_params) {
        Ok(out) => {
            // egui expects sRGB colors in the shader, so we have to read the
            // sRGB texture as though it were linear to prevent the GPU from
            // doing gamma conversion.
            ret.texture_view = Some(out.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(out.texture.format().remove_srgb_suffix()),
                ..Default::default()
            }));
        }
        Err(e) => log::error!("{e}"),
    };

    if SHOW_DRAG_VECTOR {
        queued_arrows.extend(nd_euclid.drag_delta_3d());
    }

    let project_point = |p: &Vector| {
        let ndc = draw_params.cam.project_point_to_ndc(p)?;
        let egui_pos = egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
        Some(r.rect.lerp_inside(egui_pos))
    };
    for [start, end] in std::mem::take(queued_arrows) {
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
        let ndc = draw_params
            .cam
            .project_3d_screen_space_to_ndc(screen_space)
            .unwrap_or(cgmath::Point2::new(f32::NAN, f32::NAN));
        r.rect
            .lerp_inside(egui::vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5))
    };

    // Draw gizmos (TODO: move to GPU?)
    let gizmo_painter = egui::Painter::new(
        painter.ctx().clone(),
        egui::LayerId::new(egui::Order::Middle, "twist_gizmos".into()),
        painter.clip_rect(),
    );
    if let Some(gizmo_vertex_3d_positions) = nd_euclid.renderer.gizmo_vertex_3d_positions.get() {
        if let Some(axis) = temp_gizmo_highlight {
            for (gizmo_face, &twist) in &geom.gizmo_twists {
                if puzzle.twists[twist].axis == axis {
                    show_gizmo_face(
                        &puzzle,
                        &geom,
                        gizmo_face,
                        &gizmo_vertex_3d_positions,
                        &gizmo_painter,
                        to_egui,
                        false,
                    );
                }
            }
        } else if let Some(hover) = &nd_euclid.gizmo_hover_state().filter(|_| show_gizmo_hover) {
            show_gizmo_face(
                &puzzle,
                &geom,
                hover.gizmo_face,
                &gizmo_vertex_3d_positions,
                &gizmo_painter,
                to_egui,
                true,
            );
        }
    };

    ret
}

fn show_color_edit_popup(
    ui: &mut egui::Ui,
    r: &egui::Response,
    color_to_edit: Option<Color>,
    view: &mut PuzzleView,
    prefs: &Preferences,
) {
    let puzzle = view.puzzle();

    let editing_color = EguiTempValue::new(ui);
    let mut is_first_frame = false;

    if let Some(color) = color_to_edit {
        ui.memory_mut(|mem| mem.open_popup(editing_color.id));
        editing_color.set(Some(color));
        is_first_frame = true;
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
                && crate::gui::util::clicked_elsewhere(ui, r);
        let any_click_inside_puzzle = r.clicked() || r.secondary_clicked() || r.middle_clicked();
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
}

fn allocate_puzzle_response(ui: &mut egui::Ui, downscale_rate: u32) -> (egui::Response, [u32; 2]) {
    // Allocate space in the UI.
    let (egui_rect, target_size) =
        crate::gui::util::rounded_pixel_rect(ui, ui.available_rect_before_wrap(), downscale_rate);
    let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());
    (r, target_size)
}

#[derive(Debug, Default)]
struct PuzzleViewResponse {
    color_to_edit: Option<Color>,
    texture_view: Option<wgpu::TextureView>,
    filter_mode: wgpu::FilterMode,
}

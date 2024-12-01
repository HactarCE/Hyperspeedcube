use std::fmt;
use std::sync::Arc;

use egui::mutex::RwLock;
use eyre::Result;
use hyperdraw::*;
use hypermath::prelude::*;
use hyperprefs::{AnimationPreferences, Preferences, PuzzleViewPreferencesSet};
use hyperpuzzle::{GizmoFace, Puzzle};
use image::ImageBuffer;
use parking_lot::Mutex;
use web_time::Instant;

use crate::gui::components::color_assignment_popup;
use crate::gui::util::EguiTempValue;
use crate::gui::App;
use crate::puzzle::{DragState, HoverMode, PuzzleSimulation, PuzzleView, PuzzleViewInput};
use crate::L;

/// Whether to send the mouse position to the GPU. This is useful for debugging
/// purposes, but causes the puzzle to redraw every frame that the mouse moves,
/// even when not necessary.
const SEND_CURSOR_POS: bool = false;

pub fn show(ui: &mut egui::Ui, app: &mut App, puzzle_view: &Arc<Mutex<Option<PuzzleWidget>>>) {
    let r = match &mut *puzzle_view.lock() {
        Some(puzzle_view) => puzzle_view.ui(ui, &mut app.prefs, &app.animation_prefs.value),
        None => {
            // Hint to the user to load a puzzle.
            ui.centered_and_justified(|ui| {
                ui.label(L.puzzle_view.select_a_puzzle);
            })
            .response
        }
    };

    if r.gained_focus() {
        app.set_active_puzzle_view(puzzle_view);
    }
}

pub struct PuzzleWidget {
    /// View into a puzzle simulation.
    pub view: PuzzleView,

    renderer: PuzzleRenderer,
    egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
    egui_texture_id: Option<egui::TextureId>,

    queued_arrows: Vec<[Vector; 2]>,

    pub wants_focus: bool,
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
            .field("view", &self.view)
            .field("egui_texture_id", &self.egui_texture_id)
            .field("queued_arrows", &self.queued_arrows)
            .field("wants_focus", &self.wants_focus)
            .finish_non_exhaustive()
    }
}
impl PuzzleWidget {
    pub(crate) fn new(
        lib: &hyperpuzzle::Library,
        gfx: &Arc<GraphicsState>,
        egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
        prefs: &mut Preferences,
        puzzle_id: &str,
    ) -> Option<Self> {
        let start_time = Instant::now();
        let result = lib.build_puzzle(puzzle_id).take_result_blocking();
        match result {
            Err(e) => {
                log::error!("{e:?}");
                None
            }
            Ok(p) => {
                log::info!("Built {:?} in {:?}", p.name, start_time.elapsed());
                log::info!("Updated active puzzle");
                let sim = &Arc::new(Mutex::new(PuzzleSimulation::new(&p)));
                Some(Self::with_sim(gfx, egui_wgpu_renderer, sim, prefs))
            }
        }
    }
    pub(crate) fn with_sim(
        gfx: &Arc<GraphicsState>,
        egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
        sim: &Arc<Mutex<PuzzleSimulation>>,
        prefs: &mut Preferences,
    ) -> Self {
        let puz = Arc::clone(sim.lock().puzzle_type());
        let view_preset = prefs[PuzzleViewPreferencesSet::from_ndim(puz.ndim())]
            .load_last_loaded(&L.presets.default_preset_name);
        let color_scheme = prefs
            .color_schemes
            .get_mut(&puz.colors)
            .schemes
            .load_last_loaded(&L.presets.default_preset_name);

        let view = PuzzleView::new(sim, prefs, view_preset.clone(), color_scheme.clone());
        let puzzle = view.puzzle();
        let renderer = PuzzleRenderer::new(gfx, &puzzle);
        Self {
            view,

            renderer,
            egui_wgpu_renderer,
            egui_texture_id: None,

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
        Arc::clone(self.sim().lock().puzzle_type())
    }

    /// Reloads the active puzzle. Returns `true` if the reload was successful.
    pub fn reload(&mut self, lib: &hyperpuzzle::Library, prefs: &mut Preferences) -> bool {
        crate::load_user_puzzles();
        let current_puzzle = self.puzzle();
        let gfx = Arc::clone(&self.renderer.gfx);
        let egui_wgpu_renderer = Arc::clone(&self.egui_wgpu_renderer);
        if let Some(mut new_puzzle_view) =
            Self::new(lib, &gfx, egui_wgpu_renderer, prefs, &current_puzzle.id)
        {
            // Copy view and color settings from the current puzzle view.
            new_puzzle_view.view.camera.view_preset = self.view.camera.view_preset.clone();
            new_puzzle_view.view.colors = self.view.colors.clone();

            *self = new_puzzle_view;
            true
        } else {
            false
        }
    }

    /// Draws the puzzle in the UI and handles input.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        prefs: &mut Preferences,
        animation: &AnimationPreferences,
    ) -> egui::Response {
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
            let puzzle_supports_drag_twists = puzzle.ndim() == 3;
            if is_primary && puzzle_supports_drag_twists && self.view.puzzle_hover_state().is_some()
            {
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

        let modifiers = ui.input(|input| input.modifiers);

        // Change which axis we're rotating depending on modifiers.
        if matches!(self.view.drag_state(), Some(DragState::ViewRot { .. })) {
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

        if r.has_focus()
            && ui.input(|input| input.key_pressed(egui::Key::F5))
            && crate::LIBRARY.with(|lib| self.reload(lib, prefs))
        {
            // Don't even try to redraw the puzzle. Just wait for the
            // next frame.
            return r;
        }

        // Redraw each frame until the image is stable and we have computed 3D
        // vertex positions.
        if self.renderer.puzzle_vertex_3d_positions.get().is_none()
            || self.renderer.gizmo_vertex_3d_positions.get().is_none()
        {
            ui.ctx().request_repaint();
        }

        self.view.update(
            PuzzleViewInput {
                cursor_pos,
                target_size,
                puzzle_vertex_3d_positions: self.renderer.puzzle_vertex_3d_positions.get(),
                gizmo_vertex_3d_positions: self.renderer.gizmo_vertex_3d_positions.get(),
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
        if r.clicked() && modifiers.is_none() {
            self.view.do_click_twist(Sign::Neg);
        }
        if r.secondary_clicked() && modifiers.is_none() {
            self.view.do_click_twist(Sign::Pos);
        }

        // Ctrl+shift+click = edit sticker color
        let editing_color = EguiTempValue::new(ui);
        let mut is_first_frame = false;
        if r.secondary_clicked() && modifiers.command && modifiers.shift && !modifiers.alt {
            if let Some(hov) = self.view.puzzle_hover_state() {
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
                    color_assignment_popup(
                        ui,
                        &mut self.view,
                        &prefs.color_palette,
                        editing_color.get(),
                    );
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
            .ensure_color_scheme_is_valid_for_color_system(
                &mut self.view.colors.value,
                &puzzle.colors,
            );

        let color_map = self
            .view
            .temp_colors
            .as_ref()
            .unwrap_or(&self.view.colors.value);
        let sticker_colors = puzzle
            .colors
            .list
            .iter_values()
            .map(|color_info| prefs.color_palette.get(color_map.get(&color_info.name)?))
            .map(|maybe_rgb| maybe_rgb.unwrap_or_default().rgb)
            .collect();
        self.view.temp_colors = None; // Remove temporary colors

        let draw_params = DrawParams {
            ndim: puzzle.ndim(),
            cam: self.view.camera.clone(),

            cursor_pos: cursor_pos.filter(|_| SEND_CURSOR_POS),
            is_dragging_view: match self.view.drag_state() {
                Some(DragState::ViewRot { .. }) => true,
                Some(DragState::Canceled | DragState::PreTwist | DragState::Twist) | None => false,
            },

            internals_color: prefs.styles.internals_color.rgb,
            sticker_colors,
            piece_styles: self.view.styles.values(prefs),
            piece_transforms: self.view.sim.lock().piece_transforms().map_ref(
                |_piece, transform| transform.euclidean_rotation_matrix().at_ndim(puzzle.ndim()),
            ),
        };

        // Draw puzzle.
        let painter = ui.painter_at(r.rect);
        let dark_mode = ui.visuals().dark_mode;
        let background_color = prefs.background_color(dark_mode).to_egui_color32();
        ui.painter().rect_filled(r.rect, 0.0, background_color);

        match self.update_puzzle_texture(&draw_params) {
            Ok(texture_id) => egui::Image::new((texture_id, r.rect.size())).paint_at(ui, r.rect),
            Err(e) => log::error!("{e}"),
        }

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
        if let Some(gizmo_vertex_3d_positions) = self.renderer.gizmo_vertex_3d_positions.get() {
            if let Some(axis) = self.view.temp_gizmo_highlight.take() {
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
            } else if let Some(hover) = self
                .view
                .gizmo_hover_state()
                .filter(|_| self.view.show_gizmo_hover)
            {
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

    fn update_puzzle_texture(&mut self, draw_params: &DrawParams) -> Result<egui::TextureId> {
        let output_texture = &self.renderer.draw_puzzle(&draw_params)?.texture;

        // egui expects sRGB colors in the shader, so we have to read the
        // sRGB texture as though it were linear to prevent the GPU from
        // doing gamma conversion.
        let texture_view = output_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(output_texture.format().remove_srgb_suffix()),
            ..Default::default()
        });
        let mut egui_wgpu_renderer = self.egui_wgpu_renderer.write();
        let gfx = &self.renderer.gfx;
        match self.egui_texture_id {
            Some(egui_texture_id) => {
                egui_wgpu_renderer.update_egui_texture_from_wgpu_texture(
                    &gfx.device,
                    &texture_view,
                    self.renderer.filter_mode,
                    egui_texture_id,
                );
                Ok(egui_texture_id)
            }
            None => {
                let egui_texture_id = egui_wgpu_renderer.register_native_texture(
                    &gfx.device,
                    &texture_view,
                    self.renderer.filter_mode,
                );
                self.egui_texture_id = Some(egui_texture_id);
                Ok(egui_texture_id)
            }
        }
    }

    pub fn screenshot(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        self.renderer.screenshot(width, height)
    }
}

fn show_gizmo_face(
    puzzle: &Puzzle,
    gizmo_face: GizmoFace,
    gizmo_vertex_3d_positions: &Vec<cgmath::Vector4<f32>>,
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

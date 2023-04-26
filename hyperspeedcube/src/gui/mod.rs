use anyhow::Result;
use egui_dock::NodeIndex;
use itertools::Itertools;
use ndpuzzle::{
    collections::VectorHashMap,
    geometry::{SchlafliSymbol, ShapeArena},
    math::{cga::Isometry, Matrix, Vector, VectorRef},
    puzzle::Mesh,
    vector,
};
use parking_lot::Mutex;
use std::sync::Arc;

macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod menu_bar;
mod tabs;

pub use crate::app::App;
use crate::render::{GraphicsState, PuzzleViewRenderState};

pub struct AppUi {
    dock_tree: egui_dock::Tree<Tab>,
}

impl AppUi {
    pub(crate) fn new(
        gfx: &crate::render::GraphicsState,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> Self {
        let mut dock_tree = egui_dock::Tree::new(vec![
            Tab::PuzzleView(Arc::new(Mutex::new(PuzzleView::new(gfx, egui_renderer)))),
            // Tab::Puzzle("3x3x3x3".to_string()),
            // Tab::Puzzle("3x3x3".to_string()),
            // Tab::Puzzle("Curvy Copter".to_string()),
        ]);
        let [_left, right] = dock_tree.split_right(
            NodeIndex::root(),
            0.70,
            vec![Tab::PuzzleSetup(PuzzleSetup::default())],
        );
        dock_tree.split_below(right, 0.5, vec![Tab::ViewSettings]);
        AppUi { dock_tree }
    }

    pub fn build(&mut self, ctx: &egui::Context, app: &mut App) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(app.prefs.colors.background))
            .show(ctx, |ui| {
                egui_dock::DockArea::new(&mut self.dock_tree)
                    .style(
                        egui_dock::StyleBuilder::from_egui(&ui.style())
                            .expand_tabs(true)
                            .build(),
                    )
                    .show(ctx, app);
            });

        if let Some((_rect, Tab::PuzzleView(puzzle_view))) = self.dock_tree.find_active_focused() {
            app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        }

        // key_combo_popup::build(ctx, app);
    }

    pub(crate) fn render_puzzle_views(
        &mut self,
        gfx: &crate::render::GraphicsState,
        egui_ctx: &egui::Context,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("puzzle_command_encoder"),
            });

        for tab in self.dock_tree.tabs() {
            if let Tab::PuzzleView(puzzle_view) = tab {
                puzzle_view.lock().render_and_update_texture(
                    gfx,
                    egui_ctx,
                    egui_renderer,
                    &mut encoder,
                );
            }
        }

        gfx.queue.submit([encoder.finish()]);
    }
}

impl egui_dock::TabViewer for App {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}

#[derive(Debug)]
pub enum Tab {
    PuzzleView(Arc<Mutex<PuzzleView>>),
    PuzzleSetup(PuzzleSetup),
    ViewSettings,
}
impl Tab {
    fn title(&self) -> egui::WidgetText {
        match self {
            Tab::PuzzleView(_) => "Unknown Puzzle".into(),
            Tab::PuzzleSetup(_) => "Puzzle Setup".into(),
            Tab::ViewSettings => "View Settings".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::PuzzleView(puzzle_view) => {
                if puzzle_view.lock().ui(ui) {
                    app.active_puzzle_view = Arc::downgrade(puzzle_view);
                }
            }
            Tab::PuzzleSetup(puzzle_setup) => puzzle_setup.ui(ui, app),
            Tab::ViewSettings => {
                if let Some(puzzle_view) = app.active_puzzle_view.upgrade() {
                    let mut puzzle_view_mutex_guard = puzzle_view.lock();
                    let state = &mut puzzle_view_mutex_guard.puzzle_view_render_state;

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.facet_shrink)
                                .clamp_range(0.0..=0.9)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Facet shrink");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.sticker_shrink)
                                .clamp_range(0.0..=0.9)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Sticker shrink");
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.piece_explode)
                                .clamp_range(0.0..=1.0)
                                .speed(0.005)
                                .fixed_decimals(2),
                        );
                        ui.label("Piece explode");
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.fov_3d)
                                .clamp_range(-120.0..=120.0)
                                .speed(0.5)
                                .fixed_decimals(0)
                                .suffix("°"),
                        );
                        ui.label("3D FOV");
                    });

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.fov_4d)
                                .clamp_range(-1.0..=120.0)
                                .speed(0.5)
                                .fixed_decimals(0)
                                .suffix("°"),
                        );
                        ui.label("4D FOV");
                    });
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct PuzzleView {
    puzzle_view_render_state: crate::render::PuzzleViewRenderState,
    texture_id: egui::TextureId,
    rect: egui::Rect,
}
impl PuzzleView {
    fn new(gfx: &GraphicsState, egui_renderer: &mut egui_wgpu::Renderer) -> Self {
        let texture_id = egui_renderer.register_native_texture(
            &gfx.device,
            &gfx.dummy_texture_view(),
            wgpu::FilterMode::Linear,
        );

        let mesh = Mesh::new_example_mesh().unwrap();

        PuzzleView {
            puzzle_view_render_state: PuzzleViewRenderState::new(gfx, &mesh),
            texture_id,
            rect: egui::Rect::NOTHING,
        }
    }
    fn set_mesh(&mut self, gfx: &GraphicsState, mesh: &Mesh) {
        self.puzzle_view_render_state = PuzzleViewRenderState::new(gfx, mesh);
    }
    fn ui(&mut self, ui: &mut egui::Ui) -> bool {
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
        self.puzzle_view_render_state.zoom *= (scroll_delta.y / 100.0).exp2();

        self.puzzle_view_render_state.rot = Isometry::from_angle_in_axis_plane(0, 2, -drag_delta.x)
            * Isometry::from_angle_in_axis_plane(1, 2, drag_delta.y)
            * &self.puzzle_view_render_state.rot;

        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
            true
        } else {
            false
        }
    }

    fn render_and_update_texture(
        &mut self,
        gfx: &GraphicsState,
        egui_ctx: &egui::Context,
        egui_renderer: &mut egui_wgpu::Renderer,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let new_texture = self.puzzle_view_render_state.draw_puzzle(
            gfx,
            encoder,
            (self.rect.width() as u32, self.rect.height() as u32),
        );

        // Draw puzzle if necessary.
        if let Some(texture) = new_texture {
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

#[derive(Debug)]
pub struct PuzzleSetup {
    ndim: u8,
    schlafli: String,
    seeds: Vec<Vector>,
    do_twist_cuts: bool,
    cut_depth: f32,

    error_string: Option<String>,
}
impl Default for PuzzleSetup {
    fn default() -> Self {
        Self {
            ndim: 3,
            schlafli: "4,3".to_string(),
            seeds: vec![vector![0.0, 0.0, 1.0]],
            do_twist_cuts: false,
            cut_depth: 0.9,

            error_string: None,
        }
    }
}
impl PuzzleSetup {
    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.ndim)
                    .clamp_range(2..=8)
                    .speed(0.05),
            );
            ui.label("Dimensions");
        });

        ui.strong("Schlafli symbol");
        ui.text_edit_singleline(&mut self.schlafli);
        let ndim = self.ndim;
        ui.separator();
        ui.strong("Seeds");
        let seeds_len = self.seeds.len();
        self.seeds.retain_mut(|v| {
            let mut keep = true;
            ui.horizontal(|ui| {
                ui.add_enabled_ui(seeds_len > 1, |ui| {
                    keep &= !ui.button("-").clicked();
                });
                vector_edit(ui, v, ndim);
            });
            keep
        });
        if ui.button("+").clicked() {
            self.seeds.push(vector![0.0, 0.0, 1.0]);
        }
        ui.separator();

        ui.checkbox(&mut self.do_twist_cuts, "Twist cuts");
        ui.add_enabled_ui(self.do_twist_cuts, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.cut_depth)
                        .clamp_range(0.0..=1.0)
                        .speed(0.05),
                );
                ui.label("Cut depth");
            })
        });

        let active_view = app.active_puzzle_view.upgrade();
        ui.add_enabled_ui(active_view.is_some(), |ui| {
            if ui.button("Generate!").clicked() {
                self.error_string = None;
                match self.try_generate_mesh() {
                    Ok(mesh) => {
                        active_view
                            .as_ref()
                            .unwrap()
                            .lock()
                            .set_mesh(&app.gfx, &mesh);
                    }
                    Err(e) => self.error_string = Some(e.to_string()),
                }
            }
        });

        ui.separator();
        if let Some(s) = &self.error_string {
            ui.colored_label(egui::Color32::RED, s);
        }
    }

    fn try_generate_mesh(&self) -> Result<Mesh> {
        let s = SchlafliSymbol::from_string(&self.schlafli);
        let m = Matrix::from_cols(s.mirrors().iter().rev().map(|v| &v.0))
            .inverse()
            .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
            .transpose();
        let g = s.group()?;

        let mut arena = ShapeArena::new_euclidean_cga(self.ndim);

        let mut f = 0;
        let mut seen = VectorHashMap::new();
        for elem in g.elements() {
            for seed in &self.seeds {
                let v = g[elem].transform_vector(seed);
                if seen.insert(v.clone(), ()).is_none() {
                    arena.carve_plane(&v, v.mag(), f)?;
                    f += 1;
                }
            }
        }

        if self.do_twist_cuts {
            let mut seen = VectorHashMap::new();
            for elem in g.elements() {
                let v = g[elem].transform_vector(&self.seeds[0]);
                if seen.insert(v.clone(), ()).is_none() {
                    arena.slice_plane(&v, v.mag() * self.cut_depth)?;
                }
            }
        }

        Mesh::from_arena(&arena)
    }
}

fn vector_edit(ui: &mut egui::Ui, v: &mut Vector, ndim: u8) {
    v.resize(ndim);
    ui.horizontal(|ui| {
        for i in 0..ndim {
            ui.add(
                egui::DragValue::new(&mut v[i])
                    .speed(0.01)
                    .fixed_decimals(1),
            )
            .on_hover_text(format!("Dim {i}"));
        }
    });
}

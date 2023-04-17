use egui_dock::NodeIndex;
use ndpuzzle::math::cga::Isometry;
use parking_lot::Mutex;

macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod menu_bar;
mod tabs;

pub use crate::app::App;

pub struct AppUi {
    dock_tree: egui_dock::Tree<Tab>,
}

impl AppUi {
    pub(crate) fn new(
        gfx: &crate::render::GraphicsState,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> Self {
        let mut dock_tree = egui_dock::Tree::new(vec![
            Tab::PuzzleView(Mutex::new(PuzzleView::new(gfx, egui_renderer))),
            // Tab::Puzzle("3x3x3x3".to_string()),
            // Tab::Puzzle("3x3x3".to_string()),
            // Tab::Puzzle("Curvy Copter".to_string()),
        ]);
        dock_tree.split_right(NodeIndex::root(), 0.70, vec![Tab::PuzzleList, Tab::Colors]);
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
    PuzzleView(Mutex<PuzzleView>),
    PuzzleList,
    View,
    Colors,
}
impl Tab {
    fn title(&self) -> egui::WidgetText {
        match self {
            Tab::PuzzleView(puzzle_name) => "Unknown Puzzle".into(),
            Tab::PuzzleList => "Puzzles".into(),
            Tab::View => "View".into(),
            Tab::Colors => "Colors".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::PuzzleView(puzzle_view) => {
                puzzle_view.lock().ui(ui);
            }
            Tab::PuzzleList => {
                let layout = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.with_layout(layout, |ui| {
                    ui.collapsing("3D cubic", |ui| {
                        ui.with_layout(layout, |ui| {
                            for i in 1..=9 {
                                ui.button(format!("{i}x{i}x{i}"));
                            }
                        });
                    });
                    ui.collapsing("4D hypercubic", |ui| {
                        ui.with_layout(layout, |ui| {
                            for i in 1..=9 {
                                ui.button(format!("{i}x{i}x{i}x{i}"));
                            }
                        });
                    });
                    ui.button(format!("Megaminx"));
                    ui.button(format!("120-cell"));
                });
            }
            Tab::View => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("View settings"));
                });
            }
            Tab::Colors => {
                ui.centered_and_justified(|ui| {
                    ui.label(format!("Color settings"));
                });
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
    fn new(gfx: &crate::render::GraphicsState, egui_renderer: &mut egui_wgpu::Renderer) -> Self {
        let texture_id = egui_renderer.register_native_texture(
            &gfx.device,
            &gfx.dummy_texture_view(),
            wgpu::FilterMode::Linear,
        );

        PuzzleView {
            puzzle_view_render_state: crate::render::PuzzleViewRenderState::new(
                gfx,
                &ndpuzzle::puzzle::Mesh::new_example_mesh().unwrap(),
            ),
            texture_id,
            rect: egui::Rect::NOTHING,
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui) {
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

        let mut r = ui.put(
            egui_rect,
            egui::Image::new(self.texture_id, egui_rect.size())
                .sense(egui::Sense::click_and_drag()),
        );

        self.puzzle_view_render_state.rot =
            Isometry::from_angle_in_axis_plane(0, 2, r.drag_delta().x * -0.01)
                * Isometry::from_angle_in_axis_plane(1, 2, r.drag_delta().y * 0.01)
                * &self.puzzle_view_render_state.rot;
    }

    fn render_and_update_texture(
        &mut self,
        gfx: &crate::render::GraphicsState,
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

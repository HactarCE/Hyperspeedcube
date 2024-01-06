use std::sync::Arc;

use egui_dock::NodeIndex;
use parking_lot::Mutex;

macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

#[macro_use]
mod util;
mod components;
mod ext;
mod menu_bar;
mod tabs;

pub use tabs::{PuzzleView, Tab};

pub use crate::app::App;

pub struct AppUi {
    dock_state: egui_dock::DockState<Tab>,
}

impl AppUi {
    pub(crate) fn new(egui_renderer: &mut egui_wgpu::Renderer, app: &mut App) -> Self {
        let puzzle_view = Arc::new(Mutex::new(PuzzleView::new(&app.gfx, egui_renderer)));
        app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        let dock_state = egui_dock::DockState::new(vec![
            Tab::PuzzleView(puzzle_view),
            Tab::PuzzleLibrary,
            Tab::ViewSettings,
            Tab::InteractionSettings,
            Tab::AppearanceSettings,
            Tab::LuaLogs,
            Tab::PuzzleInfo,
        ]);
        AppUi { dock_state }
    }

    pub fn build(&mut self, ctx: &egui::Context, app: &mut App) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, app));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(app.prefs.colors.background))
            .show(ctx, |ui| {
                let mut style = egui_dock::Style::from_egui(ui.style());
                style.tab_bar.fill_tab_bar = true;
                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show(ctx, app);
            });

        if let Some((_rect, Tab::PuzzleView(puzzle_view))) = self.dock_state.find_active_focused() {
            app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        }

        // key_combo_popup::build(ctx, app);
    }

    pub(crate) fn render_puzzle_views(
        &mut self,
        gfx: &crate::render::GraphicsState,
        egui_ctx: &egui::Context,
        egui_renderer: &mut egui_wgpu::Renderer,
        app: &App,
    ) {
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("puzzle_command_encoder"),
            });

        for (_, tab) in self.dock_state.iter_all_tabs() {
            if let Tab::PuzzleView(puzzle_view) = tab {
                let mut puzzle_view = puzzle_view.lock();
                let view_prefs = puzzle_view
                    .puzzle
                    .as_ref()
                    .map(|puzzle_type| app.prefs.view(&puzzle_type).clone())
                    .unwrap_or_default();
                puzzle_view.render_and_update_texture(
                    gfx,
                    egui_ctx,
                    egui_renderer,
                    &mut encoder,
                    view_prefs,
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

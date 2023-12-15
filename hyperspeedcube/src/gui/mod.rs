use std::sync::Arc;

use egui_dock::NodeIndex;
use parking_lot::Mutex;

macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

mod menu_bar;
mod tabs;

pub use tabs::{PuzzleView, Tab};

pub use crate::app::App;

pub struct AppUi {
    dock_tree: egui_dock::Tree<Tab>,
}

impl AppUi {
    pub(crate) fn new(egui_renderer: &mut egui_wgpu::Renderer, app: &mut App) -> Self {
        let puzzle_view = Arc::new(Mutex::new(PuzzleView::new(&app.gfx, egui_renderer)));
        app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        let mut dock_tree = egui_dock::Tree::new(vec![
            Tab::PuzzleView(puzzle_view),
            // Tab::Puzzle("3x3x3x3".to_string()),
            // Tab::Puzzle("3x3x3".to_string()),
            // Tab::Puzzle("Curvy Copter".to_string()),
        ]);
        let [_left, right] = dock_tree.split_right(
            NodeIndex::root(),
            0.70,
            vec![
                // Tab::PuzzleSetup(PuzzleSetup::default()),
                // Tab::PolytopeTree(PolytopeTree::default()),
                // Tab::PuzzleLibraryDemo,
                Tab::PuzzleLibrary { log_lines: vec![] },
                Tab::ViewSettings,
            ],
        );
        dock_tree.split_below(right, 0.5, vec![Tab::PuzzleInfo]);
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
                let mut style = egui_dock::Style::from_egui(ui.style());
                style.tabs.fill_tab_bar = true;
                egui_dock::DockArea::new(&mut self.dock_tree)
                    .style(style)
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

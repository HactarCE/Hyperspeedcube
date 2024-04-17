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
    app: App,
    dock_state: egui_dock::DockState<Tab>,
}

impl AppUi {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize app state.
        let initial_file = std::env::args().nth(1).map(std::path::PathBuf::from);
        let mut app = App::new(cc, initial_file);

        let puzzle_view = Arc::new(Mutex::new(None));
        app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        let mut dock_state = egui_dock::DockState::new(vec![Tab::PuzzleView(puzzle_view)]);
        let main = NodeIndex::root();
        let surface = dock_state.main_surface_mut();
        let [main, left] = surface.split_left(main, 0.2, vec![Tab::PuzzleLibrary]);
        surface.split_below(left, 0.7, vec![Tab::PuzzleInfo]);
        surface.split_right(
            main,
            0.6,
            vec![
                Tab::LuaLogs,
                Tab::PuzzleControls,
                Tab::AppearanceSettings,
                Tab::InteractionSettings,
                Tab::ViewSettings,
            ],
        );

        AppUi { app, dock_state }
    }

    pub fn build(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, &mut self.app));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        let dark_mode = ctx.style().visuals.dark_mode;
        let background_color = self.app.prefs.styles.background_color(dark_mode);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(background_color))
            .show(ctx, |ui| {
                let mut style = egui_dock::Style::from_egui(ui.style());
                style.tab_bar.fill_tab_bar = true;
                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show(ctx, &mut self.app);
            });

        // Animate puzzle views.
        for (_, tab) in self.dock_state.iter_all_tabs() {
            if let Tab::PuzzleView(puzzle_view) = tab {
                if let Some(puzzle_view) = &*puzzle_view.lock() {
                    let mut controller = puzzle_view.controller().lock();
                    let needs_redraw = controller.update_geometry(&self.app.prefs);
                    if needs_redraw {
                        // TODO: only request redraw for visible puzzles
                        ctx.request_repaint();
                    }
                }
            }
        }

        if let Some((_rect, Tab::PuzzleView(puzzle_view))) = self.dock_state.find_active_focused() {
            self.app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        }

        // key_combo_popup::build(ctx, app);
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

use std::sync::Arc;

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
        // Initialize puzzle library.
        crate::LIBRARY.with(|lib| {
            lib.set_log_line_handler(Box::new(|log_line| {
                crate::LIBRARY_LOG_LINES.lock().push(log_line);
            }));
        });

        // Initialize app state.
        let initial_file = std::env::args().nth(1).map(std::path::PathBuf::from);
        let mut app = App::new(cc, initial_file);

        let puzzle_view = Arc::new(Mutex::new(PuzzleView::new(&app.gfx)));
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

        AppUi { app, dock_state }
    }

    pub fn build(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, &mut self.app));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.app.prefs.colors.background))
            .show(ctx, |ui| {
                let mut style = egui_dock::Style::from_egui(ui.style());
                style.tab_bar.fill_tab_bar = true;
                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show(ctx, &mut self.app);
            });

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

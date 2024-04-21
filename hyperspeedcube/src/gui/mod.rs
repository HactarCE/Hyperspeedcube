use std::sync::Arc;

use egui_dock::{NodeIndex, SurfaceIndex, TabIndex};
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

        // Initialize puzzle library.
        crate::load_built_in_puzzles();
        if let Some(task) = crate::reload_user_puzzles() {
            task.take_result_blocking(); // TODO: load puzzle library async
        }

        // Initialize UI.
        let puzzle_view = Arc::new(Mutex::new(None));
        app.active_puzzle_view = Arc::downgrade(&puzzle_view);
        let mut dock_state =
            egui_dock::DockState::new(vec![Tab::PuzzleView(puzzle_view), Tab::LuaLogs]);
        let main = NodeIndex::root();
        let surface = dock_state.main_surface_mut();
        let [main, left] =
            surface.split_left(main, 0.2, vec![Tab::PuzzleLibrary, Tab::PuzzleControls]);
        surface.split_below(left, 0.7, vec![Tab::PuzzleInfo]);
        surface.split_right(main, 0.6, vec![Tab::View]);

        crate::LIBRARY.with(|lib| app.load_puzzle(lib, "3x3x3"));

        AppUi { app, dock_state }
    }

    pub fn build(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, self));

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
                style.overlay.overlay_type = egui_dock::OverlayType::HighlightedAreas;
                let mut tab_viewer = TabViewer {
                    app: &mut self.app,
                    added_nodes: vec![],
                };
                egui_dock::DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show_add_buttons(true)
                    .show_inside(ui, &mut tab_viewer);
                for index in tab_viewer.added_nodes {
                    self.dock_state.set_focused_node_and_surface(index);
                    self.dock_state
                        .push_to_focused_leaf(Tab::PuzzleView(Arc::new(Mutex::new(None))));
                }
                if self.dock_state.iter_all_tabs().next().is_none() {
                    self.dock_state
                        .push_to_first_leaf(Tab::PuzzleView(Arc::new(Mutex::new(None))));
                }
            });

        if ctx.input(|input| input.key_pressed(egui::Key::A)) {}

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

        // TODO: key combo popup
        // key_combo_popup::build(ctx, app);
    }

    fn iter_tabs(&self) -> impl '_ + Iterator<Item = ((SurfaceIndex, NodeIndex, TabIndex), &Tab)> {
        self.dock_state
            .iter_surfaces()
            .enumerate()
            .flat_map(|(i, surface)| {
                let i = SurfaceIndex(i);
                surface.iter_nodes().enumerate().flat_map(move |(j, node)| {
                    let j = NodeIndex(j);
                    node.iter_tabs().enumerate().map(move |(k, tab)| {
                        let k = TabIndex(k);
                        ((i, j, k), tab)
                    })
                })
            })
    }

    pub fn find_tab(&self, tab: &Tab) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        self.iter_tabs()
            .find(|&(_, t)| t == tab)
            .map(|(index, _)| index)
    }

    pub fn has_tab(&self, tab: &Tab) -> bool {
        self.find_tab(tab).is_some()
    }

    pub fn open_tab(&mut self, tab: &Tab) {
        match self.find_tab(tab) {
            // surface, node, tab
            Some((s, n, t)) => {
                self.dock_state.set_focused_node_and_surface((s, n));
                self.dock_state.set_active_tab((s, n, t));
            }
            None => {
                self.dock_state.push_to_focused_leaf(tab.clone());
            }
        }
    }

    pub fn close_tab(&mut self, tab: &Tab) {
        if let Some(index) = self.find_tab(tab) {
            self.dock_state.remove_tab(index);
        }
    }
}

struct TabViewer<'a> {
    app: &'a mut App,
    added_nodes: Vec<(SurfaceIndex, NodeIndex)>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self.app)
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        match tab {
            Tab::PuzzleView(puz) => egui::Id::new(Arc::as_ptr(puz)),
            _ => egui::Id::new(tab.title().text()),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }

    fn on_add(&mut self, surface: SurfaceIndex, node: NodeIndex) {
        self.added_nodes.push((surface, node))
    }
}

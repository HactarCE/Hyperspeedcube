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
mod markdown;
mod menu_bar;
mod tabs;

pub use tabs::{PuzzleWidget, Tab};

pub use crate::app::App;
use crate::L;

pub struct AppUi {
    pub app: App,
    dock_state: egui_dock::DockState<Tab>,
}

impl AppUi {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize app state.
        let initial_file = std::env::args().nth(1).map(std::path::PathBuf::from);
        let mut app = App::new(cc, initial_file);

        // Initialize puzzle library.
        hyperpuzzle_library::load_built_in_puzzles();
        hyperpuzzle_library::load_user_puzzles();

        // Override UI style.
        cc.egui_ctx.style_mut(|style| {
            style.spacing.scroll = egui::style::ScrollStyle::solid();
        });

        // Initialize UI.
        let puzzle_view = Arc::new(Mutex::new(None));
        app.set_active_puzzle_view(&puzzle_view);
        let mut dock_state = egui_dock::DockState::new(vec![Tab::PuzzleView(puzzle_view)]);
        let main = NodeIndex::root();
        let surface = dock_state.main_surface_mut();
        let [main, left] =
            surface.split_left(main, 0.15, vec![Tab::PuzzleLibrary, Tab::PuzzleControls]);
        surface.split_below(left, 0.7, vec![Tab::PuzzleInfo]);
        let [_main, right] = surface.split_right(
            main,
            0.8,
            vec![Tab::PieceFilters, Tab::DevTools, Tab::Styles, Tab::View],
        );
        surface.split_below(right, 0.6, vec![Tab::LuaLogs]);

        app.load_puzzle("ft_cube:3");

        AppUi { app, dock_state }
    }

    pub fn build(&mut self, ctx: &egui::Context) {
        set_middle_click_delete(ctx, self.app.prefs.interaction.middle_click_delete);

        let dark_mode = ctx.style().visuals.dark_mode;
        let background_color = self.app.prefs.background_color(dark_mode);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, self));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill({
                let [r, g, b] = background_color.rgb;
                egui::Color32::from_rgb(r, g, b)
            }))
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

        // Animate puzzle views.
        let mut puzzle_view_to_focus = None;
        for (i, tab) in self.dock_state.iter_all_tabs() {
            if let Tab::PuzzleView(puzzle_widget) = tab {
                if let Some(puzzle_view) = &mut *puzzle_widget.lock() {
                    if puzzle_view.wants_focus {
                        puzzle_view.wants_focus = false;
                        puzzle_view_to_focus = Some(i);
                    }
                    let mut sim = puzzle_view.sim().lock();
                    let needs_redraw = sim.step(&self.app.animation_prefs.value);
                    if needs_redraw {
                        // TODO: only request redraw for visible puzzles
                        ctx.request_repaint();
                    }
                }
            }
        }
        if let Some(i) = puzzle_view_to_focus {
            self.dock_state.set_focused_node_and_surface(i);
        }

        if let Some((_rect, Tab::PuzzleView(puzzle_view))) = self.dock_state.find_active_focused() {
            self.app.set_active_puzzle_view(puzzle_view);
        }

        // Submit wgpu commands before egui does.
        self.app.gfx.submit();

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

    pub fn set_tab_state(&mut self, tab: &Tab, new_state: bool) {
        match new_state {
            true => self.open_tab(tab),
            false => self.close_tab(tab),
        }
    }

    /// Helper method wrapper around [`App::confirm_discard_changes()`].
    pub(crate) fn confirm_discard(&mut self, description: &str) -> bool {
        self.app.confirm_discard_changes(description)
    }
}

struct TabViewer<'a> {
    app: &'a mut App,
    added_nodes: Vec<(SurfaceIndex, NodeIndex)>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self.app);
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
        self.added_nodes.push((surface, node));
    }
}

fn middle_clicked(ui: &egui::Ui, r: &egui::Response) -> bool {
    r.middle_clicked() && get_middle_click_delete(ui)
        || ui.input(|input| input.modifiers.alt && !input.modifiers.command) && r.clicked()
}
fn get_middle_click_delete(ui: &egui::Ui) -> bool {
    ui.data(|data| data.get_temp(middle_click_delete_id()))
        .unwrap_or_default()
}
fn set_middle_click_delete(ctx: &egui::Context, middle_click_delete: bool) {
    ctx.data_mut(|data| data.insert_temp(middle_click_delete_id(), middle_click_delete));
}
fn middle_click_delete_id() -> egui::Id {
    unique_id!()
}

fn middle_click_to_delete_text(ui: &mut egui::Ui) -> String {
    let input_text = if get_middle_click_delete(ui) {
        L.inputs.middle_click_or_alt_click
    } else {
        L.inputs.alt_click
    };
    L.click_to.delete.with(input_text)
}
fn md_middle_click_to_delete(ui: &mut egui::Ui) -> egui::Response {
    let text = middle_click_to_delete_text(ui);
    markdown::md(ui, text)
}

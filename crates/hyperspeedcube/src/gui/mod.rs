use std::sync::{Arc, mpsc};

use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::{NodeIndex, SurfaceIndex, TabIndex};
use markdown::md;

// TODO: use `#[track_caller]` with `std::panic::Location`?
macro_rules! unique_id {
    ($($args:tt)*) => {
        egui::Id::new((file!(), line!(), column!(), $($args)*))
    };
}

#[macro_use]
mod util;
mod components;
mod ext;
mod icons;
mod markdown;
mod menu_bar;
mod modals;
mod tabs;

pub use tabs::{PuzzleWidget, Query, Tab, about_text};
use util::EguiTempFlag;

use crate::L;
pub use crate::app::App;

pub struct AppUi {
    pub app: App,
    dock_state: egui_dock::DockState<Tab>,
    raw_winit_event_rx: mpsc::Receiver<winit::event::WindowEvent>,
}

impl AppUi {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize app state.
        let initial_file = std::env::args().nth(1).map(std::path::PathBuf::from);
        let mut app = App::new(cc, initial_file);

        // Initialize puzzle catalog.
        hyperpuzzle::load_global_catalog();

        // Override UI style.
        cc.egui_ctx.style_mut(|style| {
            style.spacing.scroll = egui::style::ScrollStyle::solid();
        });

        // Initialize UI.
        let puzzle_widget = app.new_puzzle_widget();
        app.update_active_puzzle(&puzzle_widget);
        app.load_puzzle("ft_cube:4");
        let mut dock_state = egui_dock::DockState::new(vec![Tab::Puzzle(puzzle_widget)]);
        let main = NodeIndex::root();
        let surface = dock_state.main_surface_mut();
        let [main, left] =
            surface.split_left(main, 0.15, vec![Tab::PuzzleCatalog, Tab::PuzzleControls]);
        surface.split_below(left, 0.6, vec![Tab::HpsLogs, Tab::PuzzleInfo]);
        let [_main, right] = surface.split_right(
            main,
            0.65,
            vec![
                Tab::PieceFilters,
                Tab::Colors,
                Tab::Styles,
                Tab::View,
                Tab::Animations,
                Tab::Interaction,
            ],
        );

        // Install WindowEvent hook (workaround to get raw keyboard events).
        let (raw_winit_event_tx, raw_winit_event_rx) = std::sync::mpsc::channel();
        egui_winit::install_windowevent_hook(raw_winit_event_tx);

        AppUi {
            app,
            dock_state,
            raw_winit_event_rx,
        }
    }

    pub fn build(&mut self, ctx: &egui::Context) {
        self.app.key_events = self
            .raw_winit_event_rx
            .try_iter()
            .filter_map(|ev| match ev {
                winit::event::WindowEvent::KeyboardInput {
                    event,
                    is_synthetic,
                    ..
                } => Some(event),
                _ => None,
            })
            .collect();

        set_middle_click_delete(ctx, self.app.prefs.interaction.middle_click_delete);

        if !self.app.prefs.eula {
            egui::Modal::new(unique_id!()).show(ctx, |ui| {
                md(ui, L.eula);
                let flag = EguiTempFlag::new(ui);
                let mut flag_value = flag.get();
                ui.checkbox(&mut flag_value, L.eula_checkbox);
                match flag_value {
                    true => flag.set(),
                    false => flag.reset(),
                };
                ui.add_enabled_ui(flag_value, |ui| {
                    if ui.button("Ok").clicked() {
                        self.app.prefs.eula = true;
                    }
                })
            });
        }

        let dark_mode = ctx.style().visuals.dark_mode;
        let background_color = self.app.prefs.background_color(dark_mode);

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| menu_bar::build(ui, self));

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label("todo");
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill({
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
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show_inside(ui, &mut tab_viewer);
                for index in tab_viewer.added_nodes {
                    self.dock_state.set_focused_node_and_surface(index);
                    self.dock_state
                        .push_to_focused_leaf(Tab::Puzzle(self.app.new_puzzle_widget()));
                }
                if self.dock_state.iter_all_tabs().next().is_none() {
                    self.dock_state
                        .push_to_first_leaf(Tab::Puzzle(self.app.new_puzzle_widget()));
                }

                modals::solve_complete::show(ui, &mut self.app);
            });

        // Animate puzzle views.
        let mut puzzle_widget_to_focus = None;
        for (i, tab) in self.dock_state.iter_all_tabs() {
            if let Tab::Puzzle(puzzle_widget) = tab {
                let mut puzzle_widget = puzzle_widget.lock();
                if puzzle_widget.wants_focus {
                    puzzle_widget.wants_focus = false;
                    puzzle_widget_to_focus = Some(i);
                }
                if let Some(sim) = puzzle_widget.sim() {
                    let mut sim = sim.lock();
                    // TODO: step once per simulation, not once per view
                    let needs_redraw = sim.step(&self.app.animation_prefs.value);
                    if needs_redraw {
                        // TODO: only request redraw for visible puzzles
                        ctx.request_repaint();
                    }
                }
            }
        }
        if let Some(i) = puzzle_widget_to_focus {
            self.dock_state.set_focused_node_and_surface(i);
        }

        if let Some((_rect, Tab::Puzzle(p))) = self.dock_state.find_active_focused()
            && !self.app.active_puzzle.contains(p)
        {
            self.app.update_active_puzzle(p);
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
            Tab::Puzzle(puz) => egui::Id::new(Arc::as_ptr(puz)),
            _ => egui::Id::new(tab.title().text()),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> OnCloseResponse {
        OnCloseResponse::Close
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

use std::{borrow::Cow, collections::VecDeque, sync::Arc};

use hyperprefs::{ModifiedPreset, SidebarStyle};
use itertools::Itertools;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    L,
    gui::{AppUi, PuzzleWidget, Tab, markdown::md, tabs::UtilityTab},
};

const ROOT_NODE: egui_dock::NodeIndex = egui_dock::NodeIndex::root();

pub fn build_layout_presets_ui(ui: &mut egui::Ui, app_ui: &mut AppUi) {
    let mut is_ui_layout_window_visible = app_ui.is_ui_layout_window_visible;
    egui::Window::new(L.presets.ui_layout.saved_presets)
        .open(&mut is_ui_layout_window_visible)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(24.0, 24.0))
        .title_bar(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            let serialized_dock_state =
                match super::layout::serialize_dock_state(ui.ctx(), &app_ui.dock_state) {
                    Ok(serialized) => serialized,
                    Err(e) => {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            format!("error serializing dock state: {e}"),
                        );
                        return;
                    }
                };
            let current_layout = hyperprefs::Layout {
                dock_state: Some(serialized_dock_state),
                sidebar_style: app_ui.sidebar_style,
                sidebar_utility: serde_json::to_string(&app_ui.sidebar_utility).ok(),
            };
            let mut current = ModifiedPreset {
                base: app_ui
                    .app
                    .prefs
                    .layout
                    .last_loaded_or_default(hyperprefs::DEFAULT_PRESET_NAME),
                value: current_layout.clone(),
            };
            let mut changed = false;

            crate::gui::components::PresetsUi {
                id: unique_id!(),
                presets: &mut app_ui.app.prefs.layout,
                current: &mut current,
                changed: &mut changed,
                text: &L.presets.ui_layout,
                autosave: false,
                vscroll: false,
                help_contents: None,
                extra_validation: None,
            }
            .show(ui, None, |mut prefs_ui| {
                prefs_ui.collapsing(L.prefs.layout.sidebar.title, |mut prefs_ui| {
                    let sidebar_utility_name: Cow<'_, str> = match app_ui.sidebar_utility {
                        Some(utility_tab) => {
                            L.prefs.layout.sidebar.open.with(utility_tab.title()).into()
                        }
                        None => L.prefs.layout.sidebar.closed.into(),
                    };
                    md(prefs_ui.ui, sidebar_utility_name);
                    prefs_ui.selectable_values(
                        access!(.sidebar_style),
                        &[
                            (SidebarStyle::Disabled, L.prefs.layout.sidebar.disabled),
                            (SidebarStyle::IconsOnly, L.prefs.layout.sidebar.collapsed),
                            (SidebarStyle::IconsAndText, L.prefs.layout.sidebar.expanded),
                        ],
                    );
                });

                prefs_ui.collapsing(L.prefs.layout.dock_tree.title, |prefs_ui| {
                    md(prefs_ui.ui, dock_state_to_md_string(&app_ui.dock_state));
                });
            });

            app_ui.sidebar_style = current.value.sidebar_style;

            app_ui.app.prefs.needs_save |= changed;

            if changed && current.value != current_layout {
                app_ui.load_layout(&current.value, true);
            }
        });
    app_ui.is_ui_layout_window_visible = is_ui_layout_window_visible;
}

pub fn serialize_dock_state(
    ctx: &egui::Context,
    dock_state: &egui_dock::DockState<Tab>,
) -> serde_json::Result<String> {
    serde_json::to_string(&UiLayout::from_egui(ctx, dock_state))
}

pub fn deserialize_dock_state(
    s: &str,
    puzzle_views: Vec<Arc<Mutex<PuzzleWidget>>>,
    keep_extra_puzzle_views: bool,
) -> serde_json::Result<egui_dock::DockState<Tab>> {
    Ok(serde_json::from_str::<UiLayout>(s)?.restore(puzzle_views, keep_extra_puzzle_views))
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
struct UiLayout {
    main_surface: UiLayoutNode,
    windows: Vec<UiLayoutWindow>,
}
impl UiLayout {
    fn restore(
        &self,
        puzzle_views: Vec<Arc<Mutex<PuzzleWidget>>>,
        keep_extra_puzzle_views: bool,
    ) -> egui_dock::DockState<Tab> {
        let mut puzzle_views = puzzle_views.into_iter();

        // Restore main surface
        let mut dock_state = egui_dock::DockState::new(vec![]);
        let tree = dock_state.main_surface_mut();
        self.main_surface
            .restore(tree, ROOT_NODE, &mut puzzle_views);

        // Restore each window
        for window in &self.windows {
            window.restore(&mut dock_state, &mut puzzle_views);
        }

        // Add unused puzzle views
        if keep_extra_puzzle_views {
            let remaining_tabs = puzzle_views
                .map(|puzzle_view| Tab::Puzzle(Some(puzzle_view)))
                .collect_vec();
            if !remaining_tabs.is_empty() {
                dock_state.add_window(remaining_tabs);
            }
        }

        dock_state
    }

    fn from_egui(ctx: &egui::Context, dock_state: &egui_dock::DockState<Tab>) -> Self {
        Self {
            main_surface: UiLayoutNode::from_egui(dock_state.main_surface(), ROOT_NODE),
            windows: dock_state
                .iter_surfaces()
                .enumerate()
                .filter_map(|(i, surface)| match surface {
                    egui_dock::Surface::Window(tree, window_state) => {
                        let surf_index = egui_dock::SurfaceIndex(i);
                        let id = format!("window {surf_index:?}").into(); // must match egui_dock internals
                        let area_state = egui::AreaState::load(ctx, id);
                        Some(UiLayoutWindow {
                            pos: area_state.map(|s| s.left_top_pos().into()),
                            size: area_state.and_then(|s| s.size).map(|vec2| vec2.into()),
                            contents: UiLayoutNode::from_egui(tree, ROOT_NODE),
                        })
                    }
                    _ => None,
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct UiLayoutWindow {
    pos: Option<[f32; 2]>,
    size: Option<[f32; 2]>,
    contents: UiLayoutNode,
}
impl UiLayoutWindow {
    fn restore(
        &self,
        dock_state: &mut egui_dock::DockState<Tab>,
        puzzle_views: &mut impl Iterator<Item = Arc<Mutex<PuzzleWidget>>>,
    ) {
        let surface_index = dock_state.add_window(vec![]);
        let tree = &mut dock_state[surface_index];
        self.contents.restore(tree, ROOT_NODE, puzzle_views);
        let window_state = dock_state
            .get_window_state_mut(surface_index)
            .expect("expected window");
        if let Some(pos) = self.pos {
            window_state.set_position(pos.into());
        }
        if let Some(size) = self.size {
            window_state.set_size(size.into());
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
enum UiLayoutNode {
    #[default]
    Empty,
    Leaf {
        tabs: Vec<UiLayoutTab>,
        active_tab: usize,
    },
    SplitV {
        fraction: f32,
        top: Box<UiLayoutNode>,
        bottom: Box<UiLayoutNode>,
    },
    SplitH {
        fraction: f32,
        left: Box<UiLayoutNode>,
        right: Box<UiLayoutNode>,
    },
}
impl UiLayoutNode {
    fn restore(
        &self,
        tree: &mut egui_dock::Tree<Tab>,
        node_index: egui_dock::NodeIndex,
        puzzle_views: &mut impl Iterator<Item = Arc<Mutex<PuzzleWidget>>>,
    ) {
        match self {
            Self::Empty => {
                tree[node_index] = egui_dock::Node::Empty;
            }
            Self::Leaf { tabs, active_tab } => {
                tree[node_index] = egui_dock::Node::leaf_with(if tabs.is_empty() {
                    vec![Tab::default()]
                } else {
                    tabs.iter()
                        .map(|tab| match tab {
                            UiLayoutTab::Puzzle => Tab::Puzzle(puzzle_views.next()),
                            UiLayoutTab::Utility(utility_tab) => Tab::Utility(*utility_tab),
                        })
                        .collect()
                });
            }
            Self::SplitV {
                fraction,
                top,
                bottom,
            } => {
                tree.split_below(node_index, *fraction, vec![Tab::default()]);
                top.restore(tree, node_index.left(), puzzle_views);
                bottom.restore(tree, node_index.right(), puzzle_views);
            }
            Self::SplitH {
                fraction,
                left,
                right,
            } => {
                tree.split_right(node_index, *fraction, vec![Tab::default()]);
                left.restore(tree, node_index.left(), puzzle_views);
                right.restore(tree, node_index.right(), puzzle_views);
            }
        }
    }

    fn from_egui(tree: &egui_dock::Tree<Tab>, node_index: egui_dock::NodeIndex) -> Self {
        match &tree[node_index] {
            egui_dock::Node::Empty => Self::Empty,
            egui_dock::Node::Leaf(leaf_node) => Self::Leaf {
                tabs: leaf_node
                    .tabs
                    .iter()
                    .map(|tab| match tab {
                        Tab::Puzzle(_) => UiLayoutTab::Puzzle,
                        Tab::Utility(utility_tab) => UiLayoutTab::Utility(*utility_tab),
                    })
                    .collect(),
                active_tab: leaf_node.active.0,
            },
            egui_dock::Node::Vertical(split_node) => Self::SplitV {
                fraction: split_node.fraction,
                top: Box::new(Self::from_egui(tree, node_index.left())),
                bottom: Box::new(Self::from_egui(tree, node_index.right())),
            },
            egui_dock::Node::Horizontal(split_node) => Self::SplitH {
                fraction: split_node.fraction,
                left: Box::new(Self::from_egui(tree, node_index.left())),
                right: Box::new(Self::from_egui(tree, node_index.right())),
            },
        }
    }
}

fn dock_state_to_md_string(dock_state: &egui_dock::DockState<Tab>) -> String {
    let mut ret = String::new();
    for (i, surface) in dock_state.iter_surfaces().enumerate() {
        if egui_dock::SurfaceIndex(i).is_main() {
            ret += "* main surface\n";
        } else {
            ret += "* floating window\n";
        }
        if let Some(tree) = surface.node_tree() {
            write_md_subtree(&mut ret, 1, tree, ROOT_NODE);
        }
    }
    ret
}

fn write_md_subtree(
    ret: &mut String,
    indent: usize,
    tree: &egui_dock::Tree<Tab>,
    subtree_root: egui_dock::NodeIndex,
) {
    match &tree[subtree_root] {
        egui_dock::Node::Empty => write_md_bullet(ret, indent, L.prefs.layout.dock_tree.empty),
        egui_dock::Node::Leaf(leaf_node) => {
            write_md_bullet(ret, indent, L.prefs.layout.dock_tree.tab_group);
            for (i, tab) in leaf_node.tabs().iter().enumerate() {
                let mut title: Cow<'_, str> = match tab {
                    Tab::Puzzle(_) => L.prefs.layout.dock_tree.puzzle_view.into(),
                    Tab::Utility(utility_tab) => L
                        .prefs
                        .layout
                        .dock_tree
                        .utility_tab
                        .with(utility_tab.title())
                        .into(),
                };
                if i == leaf_node.active.0 {
                    *title.to_mut() += L.prefs.layout.dock_tree.active_suffix;
                }
                write_md_bullet(ret, indent + 1, &title);
            }
        }
        egui_dock::Node::Vertical(split_node) => {
            let percent = (split_node.fraction * 100.0).round().to_string();
            let text = L.prefs.layout.dock_tree.v_split.with(&percent);
            write_md_bullet(ret, indent, &text);
            write_md_subtree(ret, indent + 1, tree, subtree_root.left());
            write_md_subtree(ret, indent + 1, tree, subtree_root.right());
        }
        egui_dock::Node::Horizontal(split_node) => {
            let percent = (split_node.fraction * 100.0).round().to_string();
            let text = L.prefs.layout.dock_tree.h_split.with(&percent);
            write_md_bullet(ret, indent, &text);
            write_md_subtree(ret, indent + 1, tree, subtree_root.left());
            write_md_subtree(ret, indent + 1, tree, subtree_root.right());
        }
    }
}

fn write_md_bullet(ret: &mut String, indent: usize, contents: &str) {
    for _ in 0..indent {
        *ret += "  ";
    }
    *ret += "* ";
    *ret += contents;
    *ret += "\n";
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum UiLayoutTab {
    Puzzle,
    Utility(UtilityTab),
}

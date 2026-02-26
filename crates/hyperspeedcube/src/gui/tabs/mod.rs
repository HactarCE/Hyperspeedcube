use std::sync::Arc;

use parking_lot::Mutex;

mod about;
mod animation;
mod catalog;
mod colors;
mod debug;
mod dev_tools;
mod hps_logs;
mod image_generator;
mod interaction;
mod keybinds;
mod keybinds_reference;
mod macros;
mod modifier_keys;
mod mousebinds;
mod move_input;
mod piece_filters;
mod puzzle;
mod puzzle_info;
mod scrambler;
mod styles;
mod timeline;
mod timer;
mod view;

pub use about::about_text;
pub use catalog::Query;
pub use puzzle::PuzzleWidget;
use serde::{Deserialize, Serialize};

use super::App;
use crate::L;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UtilityTab {
    Catalog,
    PuzzleInfo,
    KeybindsReference,
    About,

    Colors,
    Styles,
    View,
    Animation,

    // Input
    Interaction,
    Keybinds,
    Mousebinds,

    // Tools
    ImageGenerator,
    Macros,
    MoveInput,
    PieceFilters,
    Scrambler,
    Timeline,
    Timer,

    HpsLogs,
    DevTools,

    #[allow(unused)]
    Debug,
}
impl UtilityTab {
    /// Returns the icon, menu name, and tab title.
    fn data(self) -> (egui::Image<'static>, &'static str, &'static str) {
        let l1 = &L.tabs.menu;
        let l2 = &L.tabs.titles;
        match self {
            Self::Catalog => (mdi!(FOLDER), l1.catalog, l2.catalog),
            Self::PuzzleInfo => (mdi!(INFORMATION_BOX), l1.puzzle_info, l2.puzzle_info),
            Self::KeybindsReference => (
                mdi!(KEYBOARD_VARIANT),
                l1.keybinds_reference,
                l2.keybinds_reference,
            ),
            Self::About => (mdi!(INFORMATION), l1.about, l2.about),

            Self::Colors => (mdi!(PALETTE), l1.colors, l2.colors),
            Self::Styles => (mdi!(PALETTE_SWATCH), l1.styles, l2.styles),
            Self::View => (mdi!(CAMERA), l1.view, l2.view),
            Self::Animation => (mdi!(MOTION), l1.animation, l2.animation),

            Self::Interaction => (mdi!(BUTTON_CURSOR), l1.interaction, l2.interaction),
            Self::Keybinds => (mdi!(KEYBOARD), l1.keybinds, l2.keybinds),
            Self::Mousebinds => (mdi!(MOUSE), l1.mousebinds, l2.mousebinds),

            Self::ImageGenerator => (mdi!(IMAGE), l1.image_generator, l2.image_generator),
            Self::Macros => (mdi!(SCRIPT_TEXT_PLAY), l1.macros, l2.macros),
            Self::MoveInput => (mdi!(FORM_TEXTBOX), l1.move_input, l2.move_input),
            Self::PieceFilters => (mdi!(FILTER), l1.piece_filters, l2.piece_filters),
            Self::Scrambler => (mdi!(SHUFFLE), l1.scrambler, l2.scrambler),
            Self::Timeline => (mdi!(CHART_TIMELINE), l1.timeline, l2.timeline),
            Self::Timer => (mdi!(TIMER), l1.timer, l2.timer),

            Self::HpsLogs => (mdi!(FILE_DOCUMENT), l1.hps_logs, l2.hps_logs),
            Self::DevTools => (mdi!(CODE_BLOCK_BRACES), l1.dev_tools, l2.dev_tools),

            Self::Debug => (mdi!(BUG), l1.debug, l2.debug),
        }
    }

    pub fn icon(self) -> egui::Image<'static> {
        self.data().0
    }

    pub fn menu_name(self) -> &'static str {
        self.data().1
    }

    pub fn title(self) -> &'static str {
        self.data().2
    }

    pub fn ui(self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Self::Catalog => catalog::show(ui, app),
            Self::PuzzleInfo => puzzle_info::show(ui, app),
            Self::KeybindsReference => keybinds_reference::show(ui, app),
            Self::About => about::show(ui, app),

            Self::Colors => colors::show(ui, app),
            Self::Styles => styles::show(ui, app),
            Self::View => view::show(ui, app),
            Self::Animation => animation::show(ui, app),

            Self::Interaction => interaction::show(ui, app),
            Self::Keybinds => keybinds::show(ui, app),
            Self::Mousebinds => mousebinds::show(ui, app),

            Self::ImageGenerator => image_generator::show(ui, app),
            Self::Macros => macros::show(ui, app),
            Self::MoveInput => move_input::show(ui, app),
            Self::PieceFilters => piece_filters::show(ui, app),
            Self::Scrambler => scrambler::show(ui, app),
            Self::Timeline => timeline::show(ui, app),
            Self::Timer => timer::show(ui, app),

            Self::HpsLogs => hps_logs::show(ui, app),
            Self::DevTools => dev_tools::show(ui, app),

            Self::Debug => debug::show(ui, app),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Tab {
    Puzzle(
        #[serde(skip_serializing, skip_deserializing, default)] Option<Arc<Mutex<PuzzleWidget>>>,
    ),
    Utility(UtilityTab),
}
impl Default for Tab {
    fn default() -> Self {
        Self::Puzzle(None)
    }
}
impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Puzzle(_), Self::Puzzle(_)) => true,
            (Self::Puzzle(_), _) => false,
            (Self::Utility(a), Self::Utility(b)) => a == b,
            (Self::Utility(_), _) => false,
        }
    }
}
impl Tab {
    pub fn menu_name(&self) -> &'static str {
        let l = &L.tabs.menu;
        match self {
            Tab::Puzzle(_) => l.puzzle,
            Tab::Utility(u) => u.menu_name(),
        }
    }

    pub fn title(&self) -> egui::WidgetText {
        let l = &L.tabs.titles;
        match self {
            Tab::Puzzle(None) => l.puzzle.empty.into(),
            Tab::Puzzle(Some(puzzle_widget)) => puzzle_widget.lock().title().into(),
            Tab::Utility(u) => u.title().into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::Puzzle(puzzle_widget) => puzzle::show(
                ui,
                app,
                puzzle_widget.get_or_insert_with(|| app.new_puzzle_widget()),
            ),
            Tab::Utility(u) => u.ui(ui, app),
        }
    }

    pub fn puzzle_widget(&self) -> Option<Arc<Mutex<PuzzleWidget>>> {
        match self {
            Tab::Puzzle(puzzle_widget) => puzzle_widget.clone(),
            Tab::Utility(_) => None,
        }
    }
    pub fn utility_tab(&self) -> Option<UtilityTab> {
        match self {
            Tab::Puzzle(_) => None,
            Tab::Utility(tab) => Some(*tab),
        }
    }
}

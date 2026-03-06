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
use crate::{L, gui::util::IconTint};

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

    #[allow(unused)] // only accessible with debug assertions enabled
    Debug,
}
impl UtilityTab {
    /// Returns the icon.
    pub fn icon(self, tint: impl IconTint, size: f32) -> egui::Image<'static> {
        match self {
            Self::Catalog => mdi!(tint, FOLDER, size),
            Self::PuzzleInfo => mdi!(tint, INFORMATION_BOX, size),
            Self::KeybindsReference => mdi!(tint, KEYBOARD_VARIANT, size),
            Self::About => mdi!(tint, INFORMATION, size),

            Self::Colors => mdi!(tint, PALETTE, size),
            Self::Styles => mdi!(tint, PALETTE_SWATCH, size),
            Self::View => mdi!(tint, CAMERA, size),
            Self::Animation => mdi!(tint, MOTION, size),

            Self::Interaction => mdi!(tint, BUTTON_CURSOR, size),
            Self::Keybinds => mdi!(tint, KEYBOARD, size),
            Self::Mousebinds => mdi!(tint, MOUSE, size),

            Self::ImageGenerator => mdi!(tint, IMAGE, size),
            Self::Macros => mdi!(tint, SCRIPT_TEXT_PLAY, size),
            Self::MoveInput => mdi!(tint, FORM_TEXTBOX, size),
            Self::PieceFilters => mdi!(tint, FILTER, size),
            Self::Scrambler => mdi!(tint, SHUFFLE, size),
            Self::Timeline => mdi!(tint, CHART_TIMELINE, size),
            Self::Timer => mdi!(tint, TIMER, size),

            Self::HpsLogs => mdi!(tint, FILE_DOCUMENT, size),
            Self::DevTools => mdi!(tint, CODE_BLOCK_BRACES, size),

            Self::Debug => mdi!(tint, BUG, size),
        }
    }

    /// Returns the menu name and tab title.
    fn strings(self) -> &'static crate::locales::Tab {
        match self {
            Self::Catalog => &L.tabs.catalog,
            Self::PuzzleInfo => &L.tabs.puzzle_info,
            Self::KeybindsReference => &L.tabs.keybinds_reference,
            Self::About => &L.tabs.about,

            Self::Colors => &L.tabs.colors,
            Self::Styles => &L.tabs.styles,
            Self::View => &L.tabs.view,
            Self::Animation => &L.tabs.animation,

            Self::Interaction => &L.tabs.interaction,
            Self::Keybinds => &L.tabs.keybinds,
            Self::Mousebinds => &L.tabs.mousebinds,

            Self::ImageGenerator => &L.tabs.image_generator,
            Self::Macros => &L.tabs.macros,
            Self::MoveInput => &L.tabs.move_input,
            Self::PieceFilters => &L.tabs.piece_filters,
            Self::Scrambler => &L.tabs.scrambler,
            Self::Timeline => &L.tabs.timeline,
            Self::Timer => &L.tabs.timer,

            Self::HpsLogs => &L.tabs.hps_logs,
            Self::DevTools => &L.tabs.dev_tools,

            Self::Debug => &L.tabs.debug,
        }
    }

    pub fn menu_name(self) -> &'static str {
        self.strings().menu
    }

    pub fn title(self) -> &'static str {
        self.strings().title
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
        match self {
            Tab::Puzzle(_) => L.tabs.puzzle.menu,
            Tab::Utility(u) => u.menu_name(),
        }
    }

    pub fn title(&self) -> egui::WidgetText {
        match self {
            Tab::Puzzle(None) => L.tabs.puzzle.title.into(),
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
            Tab::Utility(utility_tab) => {
                egui::Frame::new()
                    .inner_margin(4.0)
                    .show(ui, |ui| utility_tab.ui(ui, app));
            }
        }
    }

    pub fn puzzle_widget(&self) -> Option<&Arc<Mutex<PuzzleWidget>>> {
        match self {
            Tab::Puzzle(puzzle_widget) => puzzle_widget.as_ref(),
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

use std::sync::Arc;

use parking_lot::Mutex;

mod about;
mod animations;
mod camera;
mod colors;
mod debug;
mod dev_tools;
mod image_generator;
mod interaction;
mod keybinds;
mod keybinds_reference;
mod lua_logs;
mod macros;
mod modifier_keys;
mod mousebinds;
mod move_input;
mod piece_filters;
mod puzzle;
mod puzzle_controls;
mod puzzle_info;
mod puzzle_library;
mod scrambler;
mod styles;
mod timeline;
mod timer;
mod view;

pub use puzzle::PuzzleWidget;

use super::App;
use crate::L;

#[derive(Debug, Clone)]
pub enum Tab {
    Puzzle(Arc<Mutex<PuzzleWidget>>),
    PuzzleLibrary,
    PuzzleInfo,
    KeybindsReference,
    About,

    Colors,
    Styles,
    View,
    Animations,

    // Input
    Interaction,
    Keybinds,
    Mousebinds,

    // Tools
    Camera,
    ImageGenerator,
    Macros,
    ModifierKeys,
    MoveInput,
    PieceFilters,
    PuzzleControls,
    Scrambler,
    Timeline,
    Timer,

    LuaLogs,
    DevTools,

    #[allow(unused)]
    Debug,
}
impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Puzzle(_), Self::Puzzle(_)) => true,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl Tab {
    pub fn menu_name(&self) -> &'static str {
        let l = &L.tabs.menu;
        match self {
            Tab::Puzzle(_) => l.puzzle,
            Tab::PuzzleLibrary => l.puzzle_library,
            Tab::PuzzleInfo => l.puzzle_info,
            Tab::KeybindsReference => l.keybinds_reference,
            Tab::About => l.about,

            Tab::Colors => l.colors,
            Tab::Styles => l.styles,
            Tab::View => l.view,
            Tab::Animations => l.animations,

            Tab::Interaction => l.interaction,
            Tab::Keybinds => l.keybinds,
            Tab::Mousebinds => l.mousebinds,

            Tab::Camera => l.camera,
            Tab::ImageGenerator => l.image_generator,
            Tab::Macros => l.macros,
            Tab::ModifierKeys => l.modifier_keys,
            Tab::MoveInput => l.move_input,
            Tab::PieceFilters => l.piece_filters,
            Tab::PuzzleControls => l.puzzle_controls,
            Tab::Scrambler => l.scrambler,
            Tab::Timeline => l.timeline,
            Tab::Timer => l.timer,

            Tab::LuaLogs => l.lua_logs,
            Tab::DevTools => l.dev_tools,

            Tab::Debug => l.debug,
        }
    }

    pub fn title(&self) -> egui::WidgetText {
        let l = &L.tabs.titles;
        match self {
            Tab::Puzzle(puzzle_widget) => puzzle_widget.lock().title().into(),
            Tab::PuzzleLibrary => l.puzzle_library.into(),
            Tab::PuzzleInfo => l.puzzle_info.into(),
            Tab::KeybindsReference => l.keybinds_reference.into(),
            Tab::About => l.about.into(),

            Tab::Colors => l.colors.into(),
            Tab::Styles => l.styles.into(),
            Tab::View => l.view.into(),
            Tab::Animations => l.animations.into(),

            Tab::Interaction => l.interaction.into(),
            Tab::Keybinds => l.keybinds.into(),
            Tab::Mousebinds => l.mousebinds.into(),

            Tab::Camera => l.camera.into(),
            Tab::ImageGenerator => l.image_generator.into(),
            Tab::Macros => l.macros.into(),
            Tab::ModifierKeys => l.modifier_keys.into(),
            Tab::MoveInput => l.move_input.into(),
            Tab::PieceFilters => l.piece_filters.into(),
            Tab::PuzzleControls => l.puzzle_controls.into(),
            Tab::Scrambler => l.scrambler.into(),
            Tab::Timeline => l.timeline.into(),
            Tab::Timer => l.timer.into(),

            Tab::LuaLogs => l.lua_logs.into(),
            Tab::DevTools => l.dev_tools.into(),

            Tab::Debug => l.debug.into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::Puzzle(puzzle_widget) => puzzle::show(ui, app, puzzle_widget),
            Tab::PuzzleLibrary => puzzle_library::show(ui, app),
            Tab::PuzzleInfo => puzzle_info::show(ui, app),
            Tab::KeybindsReference => keybinds_reference::show(ui, app),
            Tab::About => about::show(ui, app),

            Tab::Colors => colors::show(ui, app),
            Tab::Styles => styles::show(ui, app),
            Tab::View => view::show(ui, app),
            Tab::Animations => animations::show(ui, app),

            Tab::Interaction => interaction::show(ui, app),
            Tab::Keybinds => keybinds::show(ui, app),
            Tab::Mousebinds => mousebinds::show(ui, app),

            Tab::Camera => camera::show(ui, app),
            Tab::ImageGenerator => image_generator::show(ui, app),
            Tab::Macros => macros::show(ui, app),
            Tab::ModifierKeys => modifier_keys::show(ui, app),
            Tab::MoveInput => move_input::show(ui, app),
            Tab::PieceFilters => piece_filters::show(ui, app),
            Tab::PuzzleControls => puzzle_controls::show(ui, app),
            Tab::Scrambler => scrambler::show(ui, app),
            Tab::Timeline => timeline::show(ui, app),
            Tab::Timer => timer::show(ui, app),

            Tab::LuaLogs => lua_logs::show(ui, app),
            Tab::DevTools => dev_tools::show(ui, app),

            Tab::Debug => debug::show(ui, app),
        }
    }
}

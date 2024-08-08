use std::sync::Arc;

use parking_lot::Mutex;

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
mod puzzle_controls;
mod puzzle_info;
mod puzzle_library;
mod puzzle_view;
mod scrambler;
mod styles;
mod timeline;
mod timer;
mod view;

pub use puzzle_view::PuzzleWidget;

use super::App;

pub fn ui_with_active_puzzle_view(
    ui: &mut egui::Ui,
    app: &mut App,
    f: impl FnOnce(&mut egui::Ui, &mut App, &mut PuzzleWidget),
) {
    if let Some(active_puzzle_view) = app.active_puzzle_view() {
        let mut puzzle_view_mutex_guard = active_puzzle_view.lock();
        if let Some(puzzle_view) = &mut *puzzle_view_mutex_guard {
            f(ui, app, puzzle_view);
            return;
        }
    }

    ui.label("No active puzzle");
}

#[derive(Debug, Clone)]
pub enum Tab {
    PuzzleView(Arc<Mutex<Option<PuzzleWidget>>>),
    PuzzleLibrary,
    PuzzleInfo,

    Colors,
    Styles,
    View,
    Animations,
    Interaction,

    // Input
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

    KeybindsReference,

    LuaLogs,
    DevTools,

    #[allow(unused)]
    Debug,
}
impl PartialEq for Tab {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PuzzleView(_), Self::PuzzleView(_)) => true,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
impl Tab {
    pub fn menu_name(&self) -> &'static str {
        match self {
            Tab::PuzzleView(_) => "New puzzle view",
            Tab::PuzzleLibrary => "Puzzle library",
            Tab::PuzzleInfo => "Puzzle info",
            Tab::KeybindsReference => "Keybinds reference",

            Tab::Colors => "Colors",
            Tab::Styles => "Styles",
            Tab::View => "View",
            Tab::Animations => "Animations",

            Tab::Interaction => "Interaction",
            Tab::Keybinds => "Keybinds",
            Tab::Mousebinds => "Mousebinds",

            Tab::Camera => "Camera",
            Tab::ImageGenerator => "Image generator",
            Tab::Macros => "Macros",
            Tab::ModifierKeys => "Modifier keys",
            Tab::MoveInput => "Move input",
            Tab::PieceFilters => "Piece filters",
            Tab::PuzzleControls => "Puzzle controls",
            Tab::Scrambler => "Custom scrambler",
            Tab::Timeline => "Timeline",
            Tab::Timer => "Timer",

            Tab::LuaLogs => "Lua logs",
            Tab::DevTools => "Developer tools",

            Tab::Debug => "Debug output",
        }
    }

    pub fn title(&self) -> egui::WidgetText {
        match self {
            Tab::PuzzleView(p) => match &*p.lock() {
                Some(p) => p.puzzle().name.clone().into(),
                None => "No Puzzle".into(),
            },
            Tab::PuzzleLibrary => "Puzzle Library".into(),
            Tab::PuzzleInfo => "Puzzle Info".into(),
            Tab::KeybindsReference => "Keybinds Reference".into(),

            Tab::Colors => "Colors".into(),
            Tab::Styles => "Styles".into(),
            Tab::View => "View".into(),
            Tab::Animations => "Animations".into(),

            Tab::Interaction => "Interaction".into(),
            Tab::Keybinds => "Keybinds".into(),
            Tab::Mousebinds => "Mousebinds".into(),

            Tab::Camera => "Camera".into(),
            Tab::ImageGenerator => "Image Generator".into(),
            Tab::Macros => "Macros".into(),
            Tab::ModifierKeys => "Modifier Keys".into(),
            Tab::MoveInput => "Move Input".into(),
            Tab::PieceFilters => "Piece Filters".into(),
            Tab::PuzzleControls => "Puzzle Controls".into(),
            Tab::Scrambler => "Scrambles".into(),
            Tab::Timeline => "Timeline".into(),
            Tab::Timer => "Timer".into(),

            Tab::LuaLogs => "Lua Logs".into(),
            Tab::DevTools => "Developer Tools".into(),

            Tab::Debug => "Debug Output".into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        match self {
            Tab::PuzzleView(puzzle_view) => puzzle_view::show(ui, app, puzzle_view),
            Tab::PuzzleLibrary => puzzle_library::show(ui, app),
            Tab::PuzzleInfo => puzzle_info::show(ui, app),

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

            Tab::KeybindsReference => keybinds_reference::show(ui, app),

            Tab::LuaLogs => lua_logs::show(ui, app),
            Tab::DevTools => dev_tools::show(ui, app),

            Tab::Debug => debug::show(ui, app),
        }
    }
}

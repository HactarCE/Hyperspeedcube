//! A keyboard-controlled speedcube simulator.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(missing_docs)] // TODO: remove

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use gui::AppUi;
use hyperpuzzle::Library;
use parking_lot::Mutex;

#[macro_use]
mod debug;
mod commands;
mod gui;
// mod logfile;
mod app;
mod gfx;
#[cfg_attr(not(target_arch = "wasm32"), path = "paths_local.rs")]
#[cfg_attr(target_arch = "wasm32", path = "paths_web.rs")]
mod paths;
mod preferences;
mod puzzle;
mod serde_impl;
mod util;

use paths::PATHS;

const TITLE: &str = "Hyperspeedcube";
const APP_ID: &str = "Hyperspeedcube";
const IS_OFFICIAL_BUILD: bool = std::option_env!("HSC_OFFICIAL_BUILD").is_some();
const ICON_32_PNG_DATA: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

thread_local! {
    static LIBRARY: hyperpuzzle::Library = Library::new();
}
lazy_static! {
    static ref LIBRARY_LOG_LINES: Mutex<Vec<hyperpuzzle::LuaLogLine>> = Mutex::new(vec![]);
}
static LUA_BUILTIN_DIR: include_dir::Dir<'_> = if crate::IS_OFFICIAL_BUILD {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lua")
} else {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/lua")
};

/// Number of points that the mouse must be dragged to twist the puzzle.
///
/// TODO: move this to preferences
const TWIST_DRAG_THRESHOLD: f32 = 5.0;

/// Speed multiplier for twists using mouse dragging.
///
/// TODO: reword this and move it to preferences
const TWIST_DRAG_SPEED: f32 = 2.0;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // Initialize logging.
    env_logger::builder().init();

    #[cfg(debug_assertions)]
    color_eyre::install().expect("error initializing panic handler");
    #[cfg(not(debug_assertions))]
    init_human_panic();

    #[cfg(feature = "deadlock_detection")]
    init_deadlock_detection();

    pollster::block_on(run())
}

#[cfg(target_arch = "wasm32")]
fn main() -> eframe::Result<()> {
    // Initialize logging.
    wasm_logger::init(wasm_logger::Config::default());

    // Log panics using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    wasm_bindgen_futures::spawn_local(run());
}

async fn run() -> eframe::Result<()> {
    let icon_data = eframe::icon_data::from_png_bytes(ICON_32_PNG_DATA)
        .expect("error loading application icon");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(crate::TITLE)
            .with_app_id(crate::APP_ID)
            .with_icon(icon_data)
            .with_maximized(true)
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(AppUi::new(cc))),
    )
}

impl eframe::App for AppUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Build all the UI.
        self.build(ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        let prefs = &mut self.app.prefs;
        if prefs.needs_save {
            prefs.save();
        }
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(5)
    }

    fn on_exit(&mut self) {
        let prefs = &mut self.app.prefs;
        if prefs.needs_save || prefs.needs_save_eventually {
            prefs.save();
        }
        prefs.block_on_final_save();
    }
}

fn load_built_in_puzzles() {
    // TODO: load puzzle library async
    let mut stack = vec![crate::LUA_BUILTIN_DIR.clone()];
    LIBRARY.with(|lib| {
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => {
                        stack.push(subdir.clone());
                    }
                    include_dir::DirEntry::File(file) => {
                        if file.path().extension().is_some_and(|ext| ext == "lua") {
                            let name = Library::relative_path_to_filename(file.path());
                            match file.contents_utf8() {
                                Some(contents) => lib.add_file(name, None, contents.to_string()),
                                None => {
                                    log::error!("Error loading built-in file {name}")
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

fn reload_user_puzzles() {
    let Some(paths) = &*crate::PATHS else {
        log::error!("Error locating Lua directory");
        return;
    };
    log::info!(
        "Loading Lua files from path {}",
        paths.lua_dir.to_string_lossy(),
    );
    // TODO: load puzzle library async
    LIBRARY.with(|lib| lib.load_directory(&paths.lua_dir).take_result_blocking());
}

fn open_dir(dir: &std::path::Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::error!("Error creating directory {dir:?}: {e}")
    }
    if let Err(e) = opener::open(dir) {
        log::error!("Error opening directory {dir:?}: {e}")
    }
}

#[cfg(not(debug_assertions))]
fn init_human_panic() {
    let human_panic_metadata = human_panic::Metadata {
        name: TITLE.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: env!("CARGO_PKG_AUTHORS").into(),
        homepage: env!("CARGO_PKG_REPOSITORY").into(),
    };

    let std_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let file_path = human_panic::handle_dump(&human_panic_metadata, info);
        human_panic::print_msg(file_path.as_ref(), &human_panic_metadata)
            .expect("human-panic: printing error message to console failed");

        rfd::MessageDialog::new()
            .set_title(&format!("{TITLE} crashed"))
            .set_description(&match file_path {
                Some(fp) => format!(
                    "A crash report has been saved to \"{}\"\n\n\
                     Please submit this to the developer",
                    fp.display(),
                ),
                None => format!("Error saving crash report"),
            })
            .set_level(rfd::MessageLevel::Error)
            .show();

        std_panic_hook(info);
    }));
}

/// Create a background thread that checks for deadlocks every 10 seconds.
#[cfg(feature = "deadlock_detection")]
fn init_deadlock_detection() {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let deadlocks = parking_lot::deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        log::error!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            log::error!("Deadlock #{}", i);
            for t in threads {
                log::error!("Thread Id {:#?}", t.thread_id());
                log::error!("{:#?}", t.backtrace());
            }
        }
    });
}

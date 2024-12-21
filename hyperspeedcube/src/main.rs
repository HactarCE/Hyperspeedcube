//! Multidimensional twisty puzzle simulator.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)] // TODO: remove

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use std::sync::Arc;

use gui::AppUi;
pub use hyperprefs::IS_OFFICIAL_BUILD;
use hyperpuzzle::Library;
use parking_lot::Mutex;

#[macro_use]
mod debug;
mod app;
mod commands;
mod gui;
mod locales;
mod util;

/// Strings for the current locale.
///
/// This can be made customizable in the future using the crate `atomic`.
const L: locales::Lang = locales::en::LANG;

const TITLE: &str = "Hyperspeedcube";
const APP_ID: &str = "Hyperspeedcube";
const ICON_32_PNG_DATA: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

thread_local! {
    static LIBRARY: hyperpuzzle::Library = Library::new();
}
lazy_static! {
    static ref LIBRARY_LOG_LINES: Mutex<Vec<hyperpuzzle::LuaLogLine>> = Mutex::new(vec![]);
    static ref PROGRAM: hyperpuzzle_log::Program = hyperpuzzle_log::Program {
        name: Some(TITLE.to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
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

/// Name of the default piece style.
pub const DEFAULT_STYLE_NAME: &str = "Default";

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
    wasm_logger::init(wasm_logger::Config::new(log::Level::Warn));

    // Log panics using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    wasm_bindgen_futures::spawn_local(run());
}

#[allow(clippy::arc_with_non_send_sync)]
async fn run() -> eframe::Result<()> {
    let icon_data = eframe::icon_data::from_png_bytes(ICON_32_PNG_DATA)
        .expect("error loading application icon");

    // Request WGPU features.
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    let old_device_descriptor_fn = Arc::clone(&wgpu_options.device_descriptor);
    wgpu_options.device_descriptor = Arc::new(move |adapter| {
        let mut device_descriptor = old_device_descriptor_fn(adapter);
        device_descriptor.required_features |= wgpu::Features::CLEAR_TEXTURE;

        // Mimic `egui_wgpu::WgpuConfiguration::default()` using default WebGL2
        // limits. This ensures that errors are caught during native
        // development, not at runtime on a system with only OpenGL or WebGL.
        // This way we only request the functionality that we need.
        let mut new_limits = wgpu::Limits::downlevel_webgl2_defaults();
        new_limits.max_texture_dimension_2d =
            device_descriptor.required_limits.max_texture_dimension_2d;

        // Increase limits as needed for puzzle rendering.
        new_limits.max_storage_buffers_per_shader_stage = 6; // default is 8
        new_limits.max_compute_invocations_per_workgroup = 64; // same as default
        new_limits.max_compute_workgroup_size_x = 64; // same as default
        new_limits.max_compute_workgroup_size_y = 1; // default is 256
        new_limits.max_compute_workgroup_size_z = 1; // default is 64
        new_limits.max_storage_buffer_binding_size = 128 << 20; // 128 MiB, same as default
        new_limits.max_compute_workgroups_per_dimension = 65535; // default is 65535

        device_descriptor.required_limits = new_limits;

        device_descriptor
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(crate::TITLE)
            .with_app_id(crate::APP_ID)
            .with_icon(icon_data)
            .with_maximized(true)
            .with_min_inner_size([400.0, 300.0]),
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        TITLE,
        native_options,
        Box::new(|cc| Ok(Box::new(AppUi::new(cc)))),
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
                                    log::error!("Error loading built-in file {name}");
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

fn load_user_puzzles() {
    let Ok(lua_dir) = hyperprefs::paths::lua_dir() else {
        log::error!("Error locating Lua directory");
        return;
    };
    log::info!("Loading Lua files from path {}", lua_dir.to_string_lossy());
    // TODO: load puzzle library async
    LIBRARY.with(|lib| lib.load_directory(lua_dir).take_result_blocking());
}

fn open_dir(dir: &std::path::Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::error!("Error creating directory {dir:?}: {e}");
    }
    if let Err(e) = opener::open(dir) {
        log::error!("Error opening directory {dir:?}: {e}");
    }
}

#[cfg(not(debug_assertions))]
fn init_human_panic() {
    let human_panic_metadata = human_panic::Metadata::new(TITLE, env!("CARGO_PKG_VERSION"))
        .authors(env!("CARGO_PKG_AUTHORS"))
        .homepage(env!("CARGO_PKG_REPOSITORY"));

    let std_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let file_path = human_panic::handle_dump(&human_panic_metadata, info);
        human_panic::print_msg(file_path.as_ref(), &human_panic_metadata)
            .expect("human-panic: printing error message to console failed");

        rfd::MessageDialog::new()
            .set_title(&L._crash._app_crashed.with(TITLE))
            .set_description(&match file_path {
                Some(fp) => L._crash._crash_report_saved.with(&fp.display().to_string()),
                None => L._crash._error_saving_crash_report.to_string(),
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

//! A keyboard-controlled speedcube simulator.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(
    clippy::cargo,
    clippy::doc_markdown,
    clippy::if_then_some_else_none,
    clippy::manual_let_else,
    clippy::semicolon_if_nothing_returned,
    clippy::semicolon_inside_block,
    clippy::too_many_lines,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_used,
    missing_docs,
    rust_2018_idioms
)]
#![allow(
    clippy::collapsible_match,
    clippy::match_like_matches_macro,
    clippy::multiple_crate_versions,
    clippy::single_match,
    clippy::useless_format,
    missing_docs // TODO: remove
)]

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
mod preferences;
// pub mod puzzle;
mod app;
mod render;
mod serde_impl;
mod util;

const TITLE: &str = "Hyperspeedcube";
const APP_ID: &str = "Hyperspeedcube";
const IS_OFFICIAL_BUILD: bool = std::option_env!("HSC_OFFICIAL_BUILD").is_some();
const ICON_32_PNG_DATA: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

thread_local! {
    static LIBRARY: Library = Library::new();
}
lazy_static! {
    static ref LIBRARY_LOG_LINES: Mutex<Vec<hyperpuzzle::LuaLogLine>> = Mutex::new(vec![]);
}
static LUA_BUILTIN_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lua");

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // Initialize logging.
    env_logger::builder().init();

    #[cfg(debug_assertions)]
    color_eyre::install().expect("error initializing panic handler");
    #[cfg(not(debug_assertions))]
    init_human_panic();

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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Build all the UI.
        self.build(ctx);
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

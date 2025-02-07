//! Multidimensional twisty puzzle simulator.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused)] // TODO: remove

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

#[macro_use]
mod debug;
mod app;
mod cli;
mod commands;
mod gui;
mod locales;
mod util;

pub use gui::about_text;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

/// Strings for the current locale.
///
/// This can be made customizable in the future using the crate `atomic`.
const L: locales::Lang = locales::en::LANG;

const TITLE: &str = "Hyperspeedcube";
const APP_ID: &str = "Hyperspeedcube";
const ICON_32_PNG_DATA: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

lazy_static! {
    static ref PROGRAM: hyperpuzzle_log::Program = hyperpuzzle_log::Program {
        name: Some(TITLE.to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };
}

/// Number of points that the mouse must be dragged to twist the puzzle.
///
/// TODO: move this to preferences
const TWIST_DRAG_THRESHOLD: f32 = 5.0;

/// Name of the default piece style.
pub const DEFAULT_STYLE_NAME: &str = "Default";

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eyre::Result<()> {
    use clap::Parser;

    let args = cli::Args::parse();

    if let Some(subcommand) = args.subcommand {
        color_eyre::install().expect("error initializing panic handler");
        cli::exec(subcommand)?;
        return Ok(());
    }

    // Initialize logging.
    env_logger::builder().init();

    #[cfg(debug_assertions)]
    color_eyre::install().expect("error initializing panic handler");
    #[cfg(not(debug_assertions))]
    std::panic::set_hook(Box::new(|panic_info| {
        let title = format!("{TITLE} crashed");
        let backtrace = std::backtrace::Backtrace::force_capture();
        let contents = format!("{title}\n\n{panic_info}\n\n{backtrace}");
        // IIFE to mimic try_block
        let fs_result = (|| {
            let dir = hyperpaths::crash_report_dir()?;
            std::fs::create_dir_all(dir)?;
            let filename = dir.join(
                format!("crash_{}.log", hyperpuzzle_core::Timestamp::now()).replace(':', "_"),
            );
            std::fs::write(&filename, &contents)?;
            eyre::Ok(filename)
        })();
        let msg = match fs_result {
            Ok(filename) => format!("Crash report saved to {}", filename.to_string_lossy()),
            Err(e) => format!("Error saving crash report to file: {e}\n\n{contents}"),
        };
        let description = "Please send this file to the developer along with \
                           a description of what you did to cause the crash";
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title(title)
            .set_description(format!("{msg}\n\n{description}"))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }));

    #[cfg(feature = "deadlock_detection")]
    init_deadlock_detection();

    Ok(pollster::block_on(run())?)
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
        wgpu_options: make_wgpu_configuration(),
        ..Default::default()
    };

    eframe::run_native(
        TITLE,
        native_options,
        Box::new(|cc| Ok(Box::new(gui::AppUi::new(cc)))),
    )
}

impl eframe::App for gui::AppUi {
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

fn open_dir(dir: &std::path::Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::error!("Error creating directory {dir:?}: {e}");
    }
    if let Err(e) = opener::open(dir) {
        log::error!("Error opening directory {dir:?}: {e}");
    }
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

fn make_wgpu_configuration() -> eframe::egui_wgpu::WgpuConfiguration {
    let mut wgpu_setup = eframe::egui_wgpu::WgpuSetupCreateNew::default();

    let old_device_descriptor_fn = std::sync::Arc::clone(&wgpu_setup.device_descriptor);

    // Request WGPU features.
    wgpu_setup.device_descriptor = std::sync::Arc::new(move |adapter| {
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

    eframe::egui_wgpu::WgpuConfiguration{
        wgpu_setup:eframe::egui_wgpu::WgpuSetup::CreateNew(wgpu_setup),
        ..Default::default()
    }
}

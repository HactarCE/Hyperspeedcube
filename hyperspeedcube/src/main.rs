//! A keyboard-controlled speedcube simulator.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::if_then_some_else_none, missing_docs)]
#![allow(
    clippy::collapsible_match,
    clippy::match_like_matches_macro,
    clippy::single_match,
    clippy::useless_format,
    missing_docs, // TODO: remove
)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use std::time::Instant;
use winit::event::{ElementState, Event, KeyboardInput, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Icon;

#[macro_use]
mod debug;
mod app;
mod commands;
mod gui;
mod logfile;
mod preferences;
pub mod puzzle;
mod render;
mod serde_impl;
mod util;

use app::App;

const TITLE: &str = "Hyperspeedcube";
const ICON_32: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");
const DEFAULT_PUZZLE: &str = "Default";

fn main() {
    // Initialize logging.
    env_logger::builder()
        .filter_module(
            "hyperspeedcube",
            if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Warn
            },
        )
        .filter_module(
            "ndpuzzle",
            if cfg!(debug_assertions) {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Warn
            },
        )
        .init();

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

    pollster::block_on(run());
}

async fn run() {
    // Initialize window.
    let event_loop = EventLoop::with_user_event();
    let window = winit::window::WindowBuilder::new()
        .with_title(crate::TITLE)
        .with_window_icon(load_application_icon())
        .build(&event_loop)
        .expect("failed to initialize window");

    // Initialize graphics state.
    let mut gfx = render::GraphicsState::new(&window).await;

    // Initialize egui.
    let window_size = window.inner_size();
    let mut egui = egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
        physical_width: window_size.width,
        physical_height: window_size.height,
        scale_factor: window.scale_factor(),
        font_definitions: egui::FontDefinitions::default(),
        style: egui::Style {
            spacing: egui::style::Spacing {
                interact_size: egui::style::Spacing::default().interact_size * egui::vec2(1.2, 1.0),
                ..Default::default()
            },
            ..Default::default()
        },
    });
    egui.context().set_visuals(match dark_light::detect() {
        dark_light::Mode::Light => egui::Visuals::light(),
        dark_light::Mode::Dark => egui::Visuals::dark(),
    });
    let mut egui_render_pass =
        egui_wgpu_backend::RenderPass::new(&gfx.device, gfx.config.format, 1);
    let puzzle_texture_id = egui_render_pass.egui_texture_from_wgpu_texture(
        &gfx.device,
        &gfx.dummy_texture_view(),
        wgpu::FilterMode::Linear,
    );

    // Initialize app state.
    let mut app = App::new(&event_loop);

    if app.prefs.show_welcome_at_startup {
        gui::windows::WELCOME.set_open(&egui.context(), true);
    }

    // Begin main loop.
    let start_time = Instant::now();
    let mut last_frame_time = Instant::now();
    event_loop.run(move |ev, _ev_loop, control_flow| {
        let mut event_has_been_captured = false;

        if *control_flow == ControlFlow::Exit {
            if app.prefs.needs_save {
                app.prefs.force_save();
            }
        }

        // Key release events should always be processed by the app to make sure
        // there's no stuck keys.
        let allow_egui_capture = match &ev {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Released,
                            ..
                        },
                    ..
                } => false,

                WindowEvent::ModifiersChanged(_) => false,

                _ => true,
            },

            _ => true,
        };

        // Prioritize sending events to the key combo popup.
        match &ev {
            Event::WindowEvent { window_id, event } if *window_id == window.id() => {
                gui::key_combo_popup_handle_event(&egui.context(), &mut app, event);
                event_has_been_captured |=
                    gui::key_combo_popup_captures_event(&egui.context(), event);
            }
            _ => (),
        }

        // If the key combo popup didn't capture the event, then let egui handle
        // it before anything else.
        if !event_has_been_captured {
            egui.handle_event(&ev);
            event_has_been_captured |= egui.captures_event(&ev) && allow_egui_capture;
        }

        // Handle events for the app.
        match ev {
            // Handle window events.
            Event::WindowEvent { window_id, event } if window_id == window.id() => match &event {
                WindowEvent::Resized(new_size) => gfx.resize(*new_size),
                WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => {
                    gfx.set_scale_factor(*scale_factor);
                    gfx.resize(**new_inner_size);
                }
                WindowEvent::ThemeChanged(theme) => egui.context().set_visuals(match theme {
                    winit::window::Theme::Light => egui::Visuals::light(),
                    winit::window::Theme::Dark => egui::Visuals::dark(),
                }),
                _ => {
                    if !event_has_been_captured {
                        app.handle_window_event(&event);
                    }

                    if matches!(
                        &event,
                        WindowEvent::KeyboardInput { .. } | WindowEvent::ModifiersChanged { .. }
                    ) {
                        egui.context().request_repaint();
                    }
                }
            },

            // Handle application-specific events.
            Event::UserEvent(event) => app.handle_app_event(event, control_flow),

            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once unless we manually
                // request it.
                window.request_redraw();
            }

            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Update delta time.
                {
                    let new_frame_time = Instant::now();
                    egui.update_time((new_frame_time - start_time).as_secs_f64());
                    app.frame(new_frame_time - last_frame_time);
                    last_frame_time = new_frame_time;
                }

                // Start egui frame.
                egui.begin_frame();

                // Build all the UI except the puzzle view in the center.
                gui::build(&egui.context(), &mut app, puzzle_texture_id);

                app.prefs.save_if_necessary();

                // Draw puzzle if necessary.
                if let Some(puzzle_texture) = app.draw_puzzle(&mut gfx) {
                    log::trace!("Repainting puzzle");

                    // Update texture for egui.
                    egui_render_pass
                        .update_egui_texture_from_wgpu_texture(
                            &gfx.device,
                            &puzzle_texture,
                            wgpu::FilterMode::Linear,
                            puzzle_texture_id,
                        )
                        .unwrap();

                    // Request a repaint.
                    egui.context().request_repaint();
                }

                let egui_output = egui.end_frame(Some(&window));

                if egui_output.needs_repaint {
                    let output_frame = match gfx.surface.get_current_texture() {
                        Ok(tex) => tex,
                        // Log other errors to the console.
                        Err(e) => {
                            match e {
                                // This error occurs when the app is minimized on
                                // Windows. Silently return here to prevent spamming
                                // the console with "The underlying surface has
                                // changed, and therefore the swap chain must be
                                // updated."
                                wgpu::SurfaceError::Outdated => (),
                                // Reconfigure the surface if lost.
                                wgpu::SurfaceError::Lost => gfx.resize(gfx.size),
                                // The system is out of memory, so quit.
                                wgpu::SurfaceError::OutOfMemory => {
                                    *control_flow = ControlFlow::Exit
                                }
                                // Log other errors.
                                _ => log::warn!("Dropped frame with error: {:?}", e),
                            }
                            return;
                        }
                    };

                    let paint_jobs = egui.context().tessellate(egui_output.shapes);
                    let mut encoder =
                        gfx.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("egui_command_encoder"),
                            });
                    let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
                        physical_width: gfx.config.width,
                        physical_height: gfx.config.height,
                        scale_factor: gfx.scale_factor as f32,
                    };
                    egui_render_pass
                        .add_textures(&gfx.device, &gfx.queue, &egui_output.textures_delta)
                        .unwrap();
                    egui_render_pass.update_buffers(
                        &gfx.device,
                        &gfx.queue,
                        &paint_jobs,
                        &screen_descriptor,
                    );
                    // Record all render passes.
                    egui_render_pass
                        .execute(
                            &mut encoder,
                            &output_frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                            &paint_jobs,
                            &screen_descriptor,
                            Some(wgpu::Color::BLACK),
                        )
                        .unwrap();
                    egui_render_pass
                        .remove_textures(egui_output.textures_delta)
                        .unwrap();
                    // Submit the commands.
                    gfx.queue.submit(std::iter::once(encoder.finish()));

                    // Present the frame.
                    output_frame.present();
                }
            }

            // Ignore other events.
            _ => (),
        };
    });
}

fn load_application_icon() -> Option<Icon> {
    match png::Decoder::new(crate::ICON_32).read_info() {
        Ok(mut reader) => match reader.output_color_type() {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                let mut img_data = vec![0_u8; reader.output_buffer_size()];
                if let Err(err) = reader.next_frame(&mut img_data) {
                    log::warn!("Failed to read icon data: {:?}", err);
                    return None;
                };
                let info = reader.info();
                match Icon::from_rgba(img_data, info.width, info.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        log::warn!("Failed to construct icon: {:?}", err);
                        None
                    }
                }
            }
            other => {
                log::warn!(
                    "Failed to load icon data due to unknown color format: {:?}",
                    other,
                );
                None
            }
        },
        Err(err) => {
            log::warn!("Failed to load icon data: {:?}", err);
            None
        }
    }
}

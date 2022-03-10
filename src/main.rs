//! A keyboard-controlled speedcube simulator.

#![allow(dead_code)]
// #![warn(missing_docs)]
#![allow(
    clippy::collapsible_match,
    clippy::match_like_matches_macro,
    clippy::single_match
)]

#[macro_use]
extern crate delegate;
#[macro_use]
extern crate enum_dispatch;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate strum;

use epi::NativeTexture;
use std::rc::Rc;
use std::time::Instant;
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::ControlFlow;

#[macro_use]
mod debug;
mod app;
mod colors;
mod commands;
mod framework;
mod gui;
mod preferences;
pub mod puzzle;
mod render;
mod serde_impl;

use app::App;
use framework::DISPLAY;
use preferences::Preferences;

const TITLE: &str = "Hyperspeedcube";
const ICON_32: &[u8] = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");

fn main() {
    let prefs = Preferences::load(None);

    let event_loop = framework::init(&prefs);

    // Initialize egui.
    let mut egui = egui_glium::EguiGlium::new(&DISPLAY);
    egui.egui_ctx.set_visuals(match dark_light::detect() {
        dark_light::Mode::Light => egui::Visuals::light(),
        dark_light::Mode::Dark => egui::Visuals::dark(),
    });

    // Initialize app state.
    let mut app = App::new(prefs, &event_loop);

    // Set up texture for rendering puzzle.
    let puzzle_texture_id = egui
        .painter
        .register_native_texture(Rc::clone(&render::cache::DUMMY_TEXTURE));
    let mut puzzle_texture_size = (1, 1);

    // Begin main loop.
    let mut next_frame_time = Instant::now();
    event_loop.run(move |ev, _ev_loop, control_flow| {
        let mut now = Instant::now();
        let mut do_frame = false;

        // Handle events.
        match ev {
            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached {
                    start: _,
                    requested_resume,
                } => {
                    now = requested_resume;
                    do_frame = true;
                }
                StartCause::Init => {
                    next_frame_time = now;
                    do_frame = true;
                }
                _ => (),
            },

            // Handle application-specific events.
            Event::UserEvent(event) => app.handle_app_event(event, control_flow),

            // Handle window events.
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::ThemeChanged(theme) => egui.egui_ctx.set_visuals(match theme {
                        winit::window::Theme::Light => egui::Visuals::light(),
                        winit::window::Theme::Dark => egui::Visuals::dark(),
                    }),
                    _ => (),
                }

                // Let the keybind popup and egui handle events.
                let consumed = gui::key_combo_popup_handle_event(&egui.egui_ctx, &mut app, &event)
                    || egui.on_event(&event);
                if !consumed {
                    app.handle_window_event(&event);
                }
            }

            // Ignore this event.
            _ => (),
        };

        if do_frame && next_frame_time <= now {
            let frame_duration = app.prefs.gfx.frame_duration();
            next_frame_time = now + frame_duration;
            if next_frame_time < Instant::now() {
                // Skip a frame (or several).
                next_frame_time = Instant::now() + frame_duration;
            }
            *control_flow = ControlFlow::WaitUntil(next_frame_time);

            app.frame(frame_duration);

            let egui_wants_repaint = egui.run(&DISPLAY, |ctx| {
                // Build most of the GUI.
                gui::build(ctx, &mut app);

                // Draw puzzle in central panel.
                egui::CentralPanel::default()
                    .frame(egui::Frame::none())
                    .show(ctx, |ui| {
                        ui.set_min_size(egui::vec2(10.0, 10.0)); // TODO: not working

                        let img_size = ui.available_size_before_wrap();
                        let new_puzzle_texture_size = (
                            (img_size.x * ui.ctx().pixels_per_point()) as u32,
                            (img_size.y * ui.ctx().pixels_per_point()) as u32,
                        );
                        if puzzle_texture_size != new_puzzle_texture_size {
                            puzzle_texture_size = new_puzzle_texture_size;
                            app.wants_repaint = true;
                        }
                        // egui uses the top left as (0, 0), but OpenGL uses the
                        // bottom left, so we have to invert the coordinates.
                        ui.add(
                            egui::Image::new(puzzle_texture_id, img_size).uv(egui::Rect {
                                min: egui::Pos2 { x: 0.0, y: 1.0 },
                                max: egui::Pos2 { x: 1.0, y: 0.0 },
                            }),
                        );
                    });
            });

            if app.prefs.needs_save {
                app.prefs.save();
            }

            if app.wants_repaint {
                let puzzle_texture = render::draw_puzzle(
                    &mut app,
                    puzzle_texture_size.0,
                    puzzle_texture_size.1,
                    egui.egui_ctx.pixels_per_point(),
                );
                egui.painter
                    .replace_native_texture(puzzle_texture_id, puzzle_texture);
            }

            if app.wants_repaint || egui_wants_repaint {
                let mut target = DISPLAY.draw();
                egui.paint(&DISPLAY, &mut target);
                target.finish().expect("failed to swap buffersr");
            }

            app.wants_repaint = false;
        }
    });
}

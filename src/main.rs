//! A keyboard-controlled speedcube simulator.

#![allow(dead_code)]
#![warn(missing_docs)]

#[macro_use]
extern crate delegate;
#[macro_use]
extern crate enum_dispatch;
#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;

use glium::glutin::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
    ContextBuilder,
};
use imgui::FontSource;
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Instant;

#[macro_use]
mod debug;

mod colors;
mod config;
mod gui;
mod input;
pub mod puzzle;
mod render;
mod serde_impl;

use config::get_config;
use puzzle::{Puzzle, PuzzleControllerTrait};

/// The title of the window.
const TITLE: &str = "Hyperspeedcube";

lazy_static! {
    static ref EVENTS_LOOP: SendWrapper<RefCell<Option<EventLoop<()>>>> =
        SendWrapper::new(RefCell::new(Some(EventLoop::new())));
    static ref DISPLAY: SendWrapper<glium::Display> = SendWrapper::new({
        let wb = WindowBuilder::new()
            .with_title(TITLE.to_owned())
            .with_window_icon(load_application_icon());
        let cb = ContextBuilder::new()
            .with_vsync(false)
            .with_multisampling(get_config().gfx.msaa as u16);
        glium::Display::new(wb, cb, EVENTS_LOOP.borrow().as_ref().unwrap())
            .expect("failed to initialize display")
    });
}

fn main() {
    // Initialize runtime data.
    let mut puzzle = Puzzle::default();
    let mut input_state = input::State::default();
    let mut events_buffer = VecDeque::new();

    // Initialize imgui.
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let mut platform = WinitPlatform::init(&mut imgui);
    let gl_window = DISPLAY.gl_window();
    let window = gl_window.window();
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Locked(1.0));
    let mut renderer = None; // We'll initialize it on the first frame.
    let mut old_scale_factor = 1.0;
    let mut old_font_size = 1.0;

    // Main loop
    let mut last_frame_time = Instant::now();
    let mut next_frame_time = Instant::now();
    EVENTS_LOOP
        .borrow_mut()
        .take()
        .unwrap()
        .run(move |event, _ev_loop, control_flow| {
            // Handle events.
            let mut now = Instant::now();
            let mut do_frame = false;

            match event.to_static() {
                Some(Event::NewEvents(cause)) => match cause {
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

                // The program is about to exit.
                Some(Event::LoopDestroyed) => (),

                // Queue the event to be handled next time we render
                // everything.
                Some(ev) => events_buffer.push_back(ev),

                // Ignore this event.
                None => (),
            };

            if do_frame && next_frame_time <= now {
                let frame_duration = get_config().gfx.frame_duration();
                next_frame_time = now + frame_duration;
                if next_frame_time < Instant::now() {
                    // Skip a frame (or several).
                    next_frame_time = Instant::now() + frame_duration;
                }
                *control_flow = ControlFlow::WaitUntil(next_frame_time);

                // Initialize imgui renderer before the first frame, or whenever
                // the scale factor or font size is updated.
                let renderer = {
                    // Has the scale factor changed?
                    let new_scale_factor = gl_window.window().scale_factor() as f32;
                    if old_scale_factor != new_scale_factor {
                        // If so, invalidate the renderer.
                        renderer = None;
                    }

                    // Has the font size changed?
                    let new_font_size = get_config().gfx.font_size;
                    if old_font_size != new_font_size && !get_config().gfx.lock_font_size {
                        // If so, invalidate the renderer.
                        renderer = None;
                    }

                    renderer.get_or_insert_with(|| {
                        imgui
                            .style_mut()
                            .scale_all_sizes(new_scale_factor / old_scale_factor);

                        imgui.fonts().clear();
                        imgui.fonts().add_font(&[FontSource::TtfData {
                            data: include_bytes!("../resources/font/NotoSans-Regular.ttf"),
                            size_pixels: (new_font_size * new_scale_factor).floor(),
                            config: None,
                        }]);

                        old_scale_factor = new_scale_factor;
                        old_font_size = new_font_size;

                        Renderer::init(&mut imgui, &**DISPLAY)
                            .expect("failed to initialize renderer")
                    })
                };

                // Prep imgui for event handling.
                let imgui_io = imgui.io_mut();
                platform
                    .prepare_frame(imgui_io, gl_window.window())
                    .expect("failed to start frame");

                if let Some(delta) = now.checked_duration_since(last_frame_time) {
                    imgui_io.update_delta_time(delta);
                    puzzle.advance(delta);
                }
                last_frame_time = now;

                // Prep the puzzle for event handling.
                let is_unsaved = puzzle.is_unsaved();
                let mut input_frame = input_state.frame(&mut puzzle, imgui_io);

                for ev in events_buffer.drain(..) {
                    // Let the keybind popup handle events.
                    if gui::keybind_popup_handle_event(&ev) {
                        continue;
                    }
                    // Let imgui handle events.
                    platform.handle_event(imgui_io, gl_window.window(), &ev);
                    // Handle events for the puzzle.
                    input_frame.handle_event(&ev);
                    // Handle events here.
                    match ev {
                        Event::WindowEvent { event, .. } => match event {
                            // Handle window close event.
                            WindowEvent::CloseRequested => {
                                if gui::confirm_discard_changes(is_unsaved, "quit") {
                                    *control_flow = ControlFlow::Exit;
                                }
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }

                // Finish handling events for the puzzle.
                input_frame.finish();

                // Prep imgui for rendering.
                let mouse_pos = imgui_io.mouse_pos;
                let ui = imgui.frame();
                let mut app = gui::AppState {
                    ui: &ui,
                    mouse_pos,
                    puzzle: &mut puzzle,
                    control_flow,
                };
                gui::build(&mut app);

                let mut target = DISPLAY.draw();

                // Render the puzzle.
                render::draw_puzzle(&mut target, &puzzle);

                // Render imgui.
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("error while rendering imgui");

                // Put it all on the screen.
                target.finish().expect("failed to swap buffers");
            }
        });
}

fn load_application_icon() -> Option<Icon> {
    let icon_png_data = include_bytes!("../resources/icon/hyperspeedcube_32x32.png");
    let png_decoder = png::Decoder::new(&icon_png_data[..]);
    match png_decoder.read_info() {
        Ok(mut reader) => match reader.output_color_type() {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                let mut img_data = vec![0_u8; reader.output_buffer_size()];
                if let Err(err) = reader.next_frame(&mut img_data) {
                    eprintln!("Failed to read icon data: {:?}", err);
                    return None;
                };
                let info = reader.info();
                match Icon::from_rgba(img_data, info.width, info.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        eprintln!("Failed to construct icon: {:?}", err);
                        None
                    }
                }
            }
            other => {
                eprintln!(
                    "Failed to load icon data due to unknown color format: {:?}",
                    other,
                );
                None
            }
        },
        Err(err) => {
            eprintln!("Failed to load icon data: {:?}", err);
            None
        }
    }
}

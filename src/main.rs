//! A keyboard-controlled speedcube simulator.

#![allow(dead_code)]
#![warn(missing_docs)]

#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;

use core::cell::RefCell;
use glium::glutin::{
    event::{ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use imgui::FontSource;
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use send_wrapper::SendWrapper;
use std::collections::VecDeque;
use std::time::Instant;

mod colors;
mod config;
pub mod puzzle;
mod render;
mod window;

use config::get_config;
use puzzle::traits::*;
use puzzle::{rubiks3d::twists, PuzzleEnum, PuzzleType};

/// The title of the window.
const TITLE: &str = "Keyboard Speedcube";

lazy_static! {
    static ref EVENTS_LOOP: SendWrapper<RefCell<Option<EventLoop<()>>>> =
        SendWrapper::new(RefCell::new(Some(EventLoop::new())));
    static ref DISPLAY: SendWrapper<glium::Display> = SendWrapper::new({
        let wb = WindowBuilder::new().with_title(TITLE.to_owned());
        let cb = ContextBuilder::new()
            .with_vsync(true)
            .with_multisampling(get_config().gfx.msaa as u16);
        glium::Display::new(wb, cb, EVENTS_LOOP.borrow().as_ref().unwrap())
            .expect("failed to initialize display")
    });
}

fn main() {
    // Initialize runtime data.
    let mut puzzle = PuzzleType::default().new();
    let mut events_buffer = VecDeque::new();

    // Initialize imgui.
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    let mut platform = WinitPlatform::init(&mut imgui);
    let gl_window = DISPLAY.gl_window();
    let window = gl_window.window();
    // Imgui DPI handling is a mess.
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Locked(1.0));

    // Initialize imgui fonts.
    let font_size = get_config().gfx.font_size as f32;
    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../resources/font/NotoSans-Regular.ttf"),
        size_pixels: font_size,
        config: None,
    }]);

    // Initialize imgui renderer.
    let mut renderer =
        Renderer::init(&mut imgui, &**DISPLAY).expect("failed to initialize renderer");

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

                for ev in events_buffer.drain(..) {
                    // Let imgui handle events.
                    platform.handle_event(imgui_io, gl_window.window(), &ev);
                    // Handle events ourself.
                    match ev {
                        Event::WindowEvent { event, .. } => match event {
                            // Handle window close event.
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                            WindowEvent::KeyboardInput { input, .. } => match input {
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(keycode),
                                    ..
                                } => {
                                    use VirtualKeyCode as Vk;
                                    match &mut puzzle {
                                        PuzzleEnum::Rubiks3D(cube) => match keycode {
                                            Vk::Escape => *control_flow = ControlFlow::Exit,
                                            Vk::U => cube.twist(*twists::R),
                                            Vk::E => cube.twist(twists::R.rev()),
                                            Vk::L => cube.twist(twists::R.fat()),
                                            Vk::M => cube.twist(twists::R.fat().rev()),
                                            Vk::N => cube.twist(*twists::U),
                                            Vk::T => cube.twist(twists::U.rev()),
                                            Vk::S => cube.twist(*twists::L),
                                            Vk::F => cube.twist(twists::L.rev()),
                                            Vk::V => cube.twist(twists::L.fat()),
                                            Vk::P => cube.twist(twists::L.fat().rev()),
                                            Vk::R => cube.twist(*twists::D),
                                            Vk::I => cube.twist(twists::D.rev()),
                                            Vk::H => cube.twist(*twists::F),
                                            Vk::D => cube.twist(twists::F.rev()),
                                            Vk::W => cube.twist(*twists::B),
                                            Vk::Y => cube.twist(twists::B.rev()),
                                            Vk::G | Vk::J => cube.twist(*twists::X),
                                            Vk::B | Vk::K => cube.twist(twists::X.rev()),
                                            Vk::O => cube.twist(*twists::Y),
                                            Vk::A => cube.twist(twists::Y.rev()),
                                            Vk::Semicolon => cube.twist(*twists::Z),
                                            Vk::Q => cube.twist(twists::Z.rev()),
                                            _ => (),
                                        },

                                        PuzzleEnum::Rubiks4D(cube) => match keycode {
                                            Vk::Escape => *control_flow = ControlFlow::Exit,
                                            _ => (),
                                        },
                                    }
                                }
                                _ => (),
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }

                // Prep imgui for rendering.
                let ui = imgui.frame();
                window::build(&ui, &mut puzzle);

                let mut target = DISPLAY.draw();

                // Render the puzzle.
                render::draw_puzzle(&mut target, &mut puzzle).expect("draw error");

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

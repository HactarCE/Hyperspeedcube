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
use send_wrapper::SendWrapper;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub mod puzzle;
mod render;

use puzzle::traits::*;
use puzzle::{rubiks3d::twists, PuzzleEnum, PuzzleType};

/// The title of the window.
const TITLE: &str = "Keyboard Speedcube";

const FPS: f64 = 60.0;

lazy_static! {
    static ref FRAME_DURATION: Duration = Duration::from_secs_f64(1.0 / FPS);
    static ref EVENTS_LOOP: SendWrapper<RefCell<Option<EventLoop<()>>>> =
        SendWrapper::new(RefCell::new(Some(EventLoop::new())));
    static ref DISPLAY: SendWrapper<glium::Display> = SendWrapper::new({
        let wb = WindowBuilder::new().with_title(TITLE.to_owned());
        let cb = ContextBuilder::new().with_vsync(true);
        glium::Display::new(wb, cb, EVENTS_LOOP.borrow().as_ref().unwrap())
            .expect("Failed to initialize display")
    });
}

fn main() {
    // Initialize runtime data.
    let mut puzzle = PuzzleType::Rubiks3D.new();
    let mut events_buffer = VecDeque::new();

    render::setup_puzzle(puzzle.puzzle_type());

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
                next_frame_time = now + *FRAME_DURATION;
                if next_frame_time < Instant::now() {
                    // Skip a frame (or several).
                    next_frame_time = Instant::now() + *FRAME_DURATION;
                }
                *control_flow = ControlFlow::WaitUntil(next_frame_time);

                if let Some(delta) = now.checked_duration_since(last_frame_time) {
                    puzzle.advance(delta);
                }
                last_frame_time = now;

                for ev in events_buffer.drain(..) {
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
                                    }
                                }
                                _ => (),
                            },
                            _ => (),
                        },
                        _ => (),
                    }
                }

                // Render the puzzle.
                let mut target = DISPLAY.draw();
                render::draw_puzzle(&mut target, &mut puzzle).expect("Draw error");
                target.finish().unwrap();
            }
        });
}

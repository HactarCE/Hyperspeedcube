//! A keyboard-controlled speedcube simulator.

#![allow(dead_code)]
#![warn(missing_docs)]

#[macro_use]
extern crate glium;
#[macro_use]
extern crate lazy_static;

use core::cell::RefCell;
use glium::glutin;
use send_wrapper::SendWrapper;
use std::time;

pub mod puzzle;
mod render;

use puzzle::traits::*;
use puzzle::{rubiks3d::twists, PuzzleEnum, PuzzleType};

/// The title of the window.
const TITLE: &str = "Keyboard Speedcube";

lazy_static! {
    static ref EVENTS_LOOP: SendWrapper<RefCell<glutin::EventsLoop>> =
        SendWrapper::new(RefCell::new(glutin::EventsLoop::new()));
    static ref DISPLAY: SendWrapper<glium::Display> = SendWrapper::new({
        let wb = glutin::WindowBuilder::new().with_title(TITLE.to_owned());
        let cb = glutin::ContextBuilder::new().with_vsync(true);
        glium::Display::new(wb, cb, &EVENTS_LOOP.borrow()).expect("Failed to initialize display")
    });
}

fn main() {
    let mut puzzle = PuzzleType::Rubiks3D.new();

    render::setup_puzzle(puzzle.puzzle_type());

    let mut last_frame = time::Instant::now();
    let mut closed = false;
    while !closed {
        // Handle events.
        EVENTS_LOOP.borrow_mut().poll_events(|ev| match ev {
            glutin::Event::WindowEvent { event, .. } => match event {
                // Handle window close event.
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::KeyboardInput { input, .. } => match input {
                    glutin::KeyboardInput {
                        state: glutin::ElementState::Pressed,
                        virtual_keycode: Some(keycode),
                        ..
                    } => {
                        use glutin::VirtualKeyCode as Vk;
                        match &mut puzzle {
                            PuzzleEnum::Rubiks3D(cube) => match keycode {
                                Vk::Escape => closed = true,
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
        });

        {
            let this_frame = time::Instant::now();
            puzzle.advance(this_frame - last_frame);
            last_frame = this_frame;
        }

        // Render the puzzle.
        let mut target = DISPLAY.draw();
        render::draw_puzzle(&mut target, &mut puzzle).expect("Draw error");
        target.finish().unwrap();
    }
}

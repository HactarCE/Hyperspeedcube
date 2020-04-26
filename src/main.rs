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

pub mod puzzle;
mod render;

use puzzle::traits::*;
use puzzle::{animator::Animator, rubiks3d::twists};

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
    let mut cube = Animator::<puzzle::Rubiks3D>::new();

    render::setup_puzzle::<puzzle::Rubiks3D>();

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
                        match keycode {
                            Vk::Escape => closed = true,
                            Vk::U => cube.twist(*twists::R),
                            Vk::E => cube.twist(twists::R.rev()),
                            Vk::N => cube.twist(*twists::U),
                            Vk::T => cube.twist(twists::U.rev()),
                            Vk::S => cube.twist(*twists::L),
                            Vk::F => cube.twist(twists::L.rev()),
                            Vk::R => cube.twist(*twists::D),
                            Vk::I => cube.twist(twists::D.rev()),
                            Vk::H => cube.twist(*twists::F),
                            Vk::D => cube.twist(twists::F.rev()),
                            Vk::W => cube.twist(*twists::B),
                            Vk::Y => cube.twist(twists::B.rev()),
                            _ => (),
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        });

        // Render the puzzle.
        let mut target = DISPLAY.draw();
        render::draw_puzzle(&mut target, &mut cube).expect("Draw error");
        target.finish().unwrap();
    }
}

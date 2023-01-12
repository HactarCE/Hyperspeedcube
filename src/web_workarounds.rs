//! Workarounds for winit not having great web support yet.
//!
//! https://github.com/rust-windowing/winit/issues?q=is%3Aissue+is%3Aopen+label%3A%22platform%3A+WebAssembly%22+

use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, ModifiersState, VirtualKeyCode, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

use crate::app::AppEvent;

pub(crate) struct WebWorkarounds {
    events: EventLoopProxy<AppEvent>,

    last_size: Option<PhysicalSize<u32>>,
    last_scale_factor: Option<f64>,

    left_modifiers: ModifiersState,
    right_modifiers: ModifiersState,
}
impl WebWorkarounds {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        Self {
            events: event_loop.create_proxy(),

            last_size: None,
            last_scale_factor: None,

            left_modifiers: ModifiersState::default(),
            right_modifiers: ModifiersState::default(),
        }
    }

    fn event(&mut self, e: impl Into<AppEvent>) {
        self.events
            .send_event(e.into())
            .expect("tried to send event but event loop doesn't exist");
    }

    pub(crate) fn generate_modifiers_changed_event(&mut self, ev: &Event<AppEvent>) {
        use VirtualKeyCode as Vk;

        let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } = ev else {
            return
        };

        let Some(keycode) = input.virtual_keycode else {return};

        let bit = match keycode {
            Vk::LShift | Vk::RShift => ModifiersState::SHIFT,
            Vk::LControl | Vk::RControl => ModifiersState::CTRL,
            Vk::LAlt | Vk::RAlt => ModifiersState::ALT,
            Vk::LWin | Vk::RWin => ModifiersState::LOGO,
            _ => return,
        };
        let mods = match keycode {
            Vk::LShift | Vk::LControl | Vk::LAlt | Vk::LWin => &mut self.left_modifiers,
            Vk::RShift | Vk::RControl | Vk::RAlt | Vk::RWin => &mut self.right_modifiers,
            _ => return,
        };

        match input.state {
            ElementState::Pressed => *mods |= bit,
            ElementState::Released => *mods &= !bit,
        }

        self.event(WindowEvent::ModifiersChanged(
            self.left_modifiers | self.right_modifiers,
        ))
    }

    pub(crate) fn generate_resize_event(&mut self, window: &Window) {
        // Winit 0.27 won't generate resize or scale changed events for us, so
        // we have to do it manually. Also, changing the scale factor while the
        // program is running breaks in nasty ways so just don't handle that at
        // all.
        //
        // Removing this is blocked on:
        // - https://github.com/rust-windowing/winit/issues/1661
        // - https://github.com/rust-windowing/winit/pull/2074

        let document = web_sys::window().unwrap().document().unwrap();

        let mut new_size = window.inner_size();
        let new_scale_factor = window.scale_factor();

        if self.last_scale_factor != Some(new_scale_factor) || self.last_size != Some(new_size) {
            self.last_scale_factor = Some(new_scale_factor);
            self.last_size = Some(new_size);

            // Emit an event so that the rest of the app can handle it normally.
            self.event(WindowEvent::Resized(new_size));

            // `window.inner_size()` tells us how big the canvas *can* be, but
            // not how big it *is*. Set the size of the canvas to what it should
            // be.
            window.set_inner_size(new_size);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum WebEvent {
    EmulateWindowEvent(WindowEvent<'static>),
}
impl From<WebEvent> for AppEvent {
    fn from(e: WebEvent) -> Self {
        Self::WebWorkaround(e)
    }
}
impl From<WindowEvent<'static>> for AppEvent {
    fn from(e: WindowEvent<'static>) -> Self {
        Self::WebWorkaround(WebEvent::EmulateWindowEvent(e))
    }
}

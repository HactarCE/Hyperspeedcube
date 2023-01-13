//! Workarounds for winit not having great web support yet.
//!
//! https://github.com/rust-windowing/winit/issues?q=is%3Aissue+is%3Aopen+label%3A%22platform%3A+WebAssembly%22+

use winit::dpi::{LogicalSize, PhysicalSize};
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
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>, window: &Window) -> Self {
        let events = event_loop.create_proxy();

        let mut ret = Self {
            events,

            last_size: None,
            last_scale_factor: None,

            left_modifiers: ModifiersState::default(),
            right_modifiers: ModifiersState::default(),
        };

        ret.generate_resize_event(window);

        ret
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

    pub(crate) fn generate_resize_event(&mut self, winit_window: &Window) {
        // Winit 0.27 won't generate resize or scale changed events for us, so
        // we have to do it manually. Also, changing the scale factor while the
        // program is running breaks in nasty ways so just don't handle that at
        // all.
        //
        // Removing this is blocked on:
        // - https://github.com/rust-windowing/winit/issues/1661
        // - https://github.com/rust-windowing/winit/pull/2074

        let web_window = web_sys::window().unwrap();
        let scale_factor = web_window.device_pixel_ratio();
        let logical_size = LogicalSize {
            width: web_window.inner_width().unwrap().as_f64().unwrap() as u32,
            height: web_window.inner_height().unwrap().as_f64().unwrap() as u32,
        };
        let physical_size = logical_size.to_physical(scale_factor);

        if self.last_scale_factor != Some(scale_factor) || self.last_size != Some(physical_size) {
            self.last_scale_factor = Some(scale_factor);
            self.last_size = Some(physical_size);

            // Emit an event so that the rest of the app can handle it normally.
            self.event(WindowEvent::Resized(physical_size));

            // `window.inner_size()` tells us how big the canvas *can* be, but
            // not how big it *is*. Set the size of the canvas to what it should
            // be.
            winit_window.set_inner_size(physical_size);
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

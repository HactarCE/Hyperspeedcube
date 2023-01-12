//! Workarounds for winit not having great web support yet.
//!
//! https://github.com/rust-windowing/winit/issues?q=is%3Aissue+is%3Aopen+label%3A%22platform%3A+WebAssembly%22+

use winit::dpi::PhysicalSize;
use winit::event::{ModifiersState, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

use crate::app::AppEvent;

pub(crate) struct WebWorkarounds {
    events: EventLoopProxy<AppEvent>,
    last_size: Option<PhysicalSize<u32>>,
    last_scale_factor: Option<f64>,
    modifiers_state: ModifiersState,
}
impl WebWorkarounds {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        Self {
            events: event_loop.create_proxy(),
            last_size: None,
            last_scale_factor: None,
            modifiers_state: ModifiersState::default(),
        }
    }

    fn event(&mut self, e: impl Into<AppEvent>) {
        self.events
            .send_event(e.into())
            .expect("tried to send event but event loop doesn't exist");
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
pub(crate) enum Event {
    EmulateWindowEvent(WindowEvent<'static>),
}
impl From<Event> for AppEvent {
    fn from(e: Event) -> Self {
        Self::WebWorkaround(e)
    }
}
impl From<WindowEvent<'static>> for AppEvent {
    fn from(e: WindowEvent<'static>) -> Self {
        Self::WebWorkaround(Event::EmulateWindowEvent(e))
    }
}

use std::path::PathBuf;
use std::sync::Weak;
use wgpu::TextureView;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::render::GraphicsState;

pub struct App {
    events: EventLoopProxy<AppEvent>,
    pub(crate) prefs: PrefsTemporary,

    active_view: Weak<ModelView>,
}

pub struct ModelView {}

impl App {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>, _initial_file: Option<PathBuf>) -> Self {
        Self {
            events: event_loop.create_proxy(),
            prefs: PrefsTemporary {
                needs_save: false,
                gfx: GfxPrefsTemporary {},
                colors: ColorsPrefsTemporary {
                    background: egui::Color32::BLACK,
                },
            },

            active_view: Weak::new(),
        }
    }

    pub(crate) fn handle_window_event(&mut self, ev: &WindowEvent) {
        // TODO
        match ev {
            WindowEvent::CloseRequested => self
                .events
                .send_event(AppEvent::Exit)
                .expect("failed to send event"),

            _ => (),
        }
    }

    pub(crate) fn handle_paste_event(&mut self, s: &str) {
        // TODO
    }

    pub(crate) fn handle_app_event(
        &mut self,
        ev: AppEvent,
        control_flow: &mut ControlFlow,
    ) -> AppEventResponse {
        match ev {
            AppEvent::Exit => *control_flow = ControlFlow::Exit,
        }
        AppEventResponse {
            copy_string: None,
            request_paste: false,
        }
    }

    pub(crate) fn draw_puzzle(&mut self, gfx: &GraphicsState) -> Option<TextureView> {
        // TODO
        None
    }

    pub(crate) fn frame(&mut self) {
        // TODO
    }
}

#[derive(Debug, Default, Clone)]
pub struct PuzzleTemporary {}
impl PuzzleTemporary {
    pub fn has_undo(&self) -> bool {
        false
    }
    pub fn has_redo(&self) -> bool {
        false
    }
}

#[derive(Debug, Default, Clone)]
pub struct PrefsTemporary {
    pub needs_save: bool,
    pub gfx: GfxPrefsTemporary,
    pub colors: ColorsPrefsTemporary,
}
impl PrefsTemporary {
    pub fn save(&mut self) {
        println!("TODO: save prefs");
    }
}
#[derive(Debug, Default, Clone)]
pub struct GfxPrefsTemporary {}
impl GfxPrefsTemporary {
    pub fn frame_duration(&self) -> instant::Duration {
        instant::Duration::from_secs_f64(1.0 / 60.0)
    }
}
#[derive(Debug, Default, Clone)]
pub struct ColorsPrefsTemporary {
    pub background: egui::Color32,
}
impl ColorsPrefsTemporary {}

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Exit,
}

#[derive(Debug, Default, Clone)]
#[must_use]
pub(crate) struct AppEventResponse {
    pub(crate) copy_string: Option<String>,
    pub(crate) request_paste: bool,
}

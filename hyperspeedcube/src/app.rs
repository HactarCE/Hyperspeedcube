use std::path::PathBuf;
use std::sync::{Arc, Weak};

use hyperpuzzle::Puzzle;
use parking_lot::Mutex;
use wgpu::TextureView;
use winit::event_loop::ControlFlow;

use crate::gfx::GraphicsState;
use crate::gui::PuzzleView;
use crate::preferences::Preferences;

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,

    pub(crate) active_puzzle_view: Weak<Mutex<Option<PuzzleView>>>,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, _initial_file: Option<PathBuf>) -> Self {
        Self {
            gfx: Arc::new(GraphicsState::new(
                cc.wgpu_render_state.as_ref().expect("no wgpu render state"),
            )),

            prefs: Preferences::load(None),

            active_puzzle_view: Weak::new(),
        }
    }

    pub(crate) fn active_puzzle_type(&self) -> Option<Arc<Puzzle>> {
        self.with_active_puzzle_view(|puzzle_view| puzzle_view.puzzle())
    }
    pub(crate) fn with_active_puzzle_view<R>(
        &self,
        f: impl FnOnce(&mut PuzzleView) -> R,
    ) -> Option<R> {
        let active_puzzle_view = self.active_puzzle_view.upgrade()?;
        let mut puzzle_view_mutex_guard = active_puzzle_view.lock();
        Some(f(puzzle_view_mutex_guard.as_mut()?))
    }

    pub(crate) fn reload_puzzle(&mut self) {
        crate::LIBRARY.with(|lib: &hyperpuzzle::Library| {
            let puzzle = self.active_puzzle_type()?;
            let file = lib.file_containing_puzzle(&puzzle.id)?;
            let path = file.path.as_ref()?;
            lib.read_file(file.name.clone(), path);
            lib.load_files().take_result_blocking();
            self.load_puzzle(lib, &puzzle.id);

            Some(())
        });
    }
    pub(crate) fn load_puzzle(&mut self, lib: &hyperpuzzle::Library, puzzle_id: &str) {
        let result = lib.build_puzzle(puzzle_id).take_result_blocking();
        match result {
            Err(e) => log::error!("{e:?}"),
            Ok(p) => {
                if let Some(puzzle_view) = self.active_puzzle_view.upgrade() {
                    log::info!("set active puzzle!");
                    *puzzle_view.lock() = Some(PuzzleView::new(&self.gfx, &p));
                } else {
                    log::warn!("no active puzzle view");
                }
            }
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

    pub(crate) fn request_redraw_puzzle(&mut self) {
        // TODO
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

use std::path::PathBuf;
use std::sync::{Arc, Weak};

use hyperpuzzle::Puzzle;
use parking_lot::Mutex;
use wgpu::TextureView;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::Window;

use crate::gfx::GraphicsState;
use crate::gui::PuzzleView;
use crate::preferences::Preferences;

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,

    pub(crate) active_puzzle_view: Weak<Mutex<PuzzleView>>,
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
        let puzzle_view = self.active_puzzle_view.upgrade()?;
        let puzzle_view_mutex_guard = puzzle_view.lock();
        Some(Arc::clone(puzzle_view_mutex_guard.puzzle.as_ref()?))
    }

    pub(crate) fn reload_puzzle(&mut self) {
        if let Some(puzzle_view) = self.active_puzzle_view.upgrade() {
            crate::LIBRARY.with(|lib| {
                let puzzle_view = puzzle_view.lock();
                let puzzle = puzzle_view.puzzle.as_ref()?;
                let puzzle_db = lib.puzzles();
                let puzzle_data = puzzle_db.get(&puzzle.id)?;
                drop(puzzle_view);
                let filename = puzzle_data.filename.clone();
                let file_path = lib.file_paths()[&filename].clone();
                let puzzle_id = puzzle_data.id.clone();
                drop(puzzle_db);
                lib.read_file(&file_path, filename);
                lib.load_files();
                self.load_puzzle(lib, &puzzle_id);

                Some(())
            });
        }
    }
    pub(crate) fn load_puzzle(&mut self, lib: &hyperpuzzle::Library, puzzle_id: &str) {
        let result = lib.build_puzzle(puzzle_id).take_result_blocking();
        match result {
            Err(e) => log::error!("{e:?}"),
            Ok(p) => {
                if let Some(puzzle_view) = self.active_puzzle_view.upgrade() {
                    log::info!("set active puzzle!");
                    puzzle_view.lock().set_puzzle(Arc::clone(&p));
                    puzzle_view.lock().puzzle = Some(p);
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

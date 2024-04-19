use std::path::PathBuf;
use std::sync::{Arc, Weak};

use hyperpuzzle::Puzzle;
use parking_lot::Mutex;

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

    /// Returns whether there is an active puzzle view. Do NOT rely on this
    /// being up-to-date; the result of this function may change by the time it
    /// returns.
    pub(crate) fn has_active_puzzle(&self) -> bool {
        self.active_puzzle_view.upgrade().is_some()
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
}

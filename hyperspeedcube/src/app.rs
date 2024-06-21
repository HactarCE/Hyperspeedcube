use std::path::PathBuf;
use std::sync::{Arc, Weak};

use hyperpuzzle::Puzzle;
use parking_lot::Mutex;

use crate::gfx::GraphicsState;
use crate::gui::PuzzleWidget;
use crate::preferences::{Preferences, PuzzleViewPreferencesSet};

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,

    active_puzzle_view: Weak<Mutex<Option<PuzzleWidget>>>,
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

    pub(crate) fn set_active_puzzle_view(
        &mut self,
        puzzle_view: &Arc<Mutex<Option<PuzzleWidget>>>,
    ) {
        self.active_puzzle_view = Arc::downgrade(puzzle_view);
        self.notify_active_puzzle_changed();
    }
    pub(crate) fn set_active_puzzle(&mut self, new_puzzle_view: Option<PuzzleWidget>) {
        match self.active_puzzle_view() {
            Some(puzzle_view) => {
                *puzzle_view.lock() = new_puzzle_view;
                self.notify_active_puzzle_changed();
            }
            None => log::warn!("No active puzzle view"),
        }
    }
    fn notify_active_puzzle_changed(&mut self) {
        if let Some(puzzle_view) = self.active_puzzle_view() {
            if let Some(p) = &*puzzle_view.lock() {
                self.prefs.latest_view_prefs_set =
                    PuzzleViewPreferencesSet::from_ndim(p.puzzle().ndim());
                self.prefs
                    .view_presets_mut()
                    .set_current_preset(p.view.camera.view_preset.clone());
                // TODO: add more presets here as relevant
                self.prefs.needs_save_eventually = true;
            }
        }
    }
    /// Returns whether there is an active puzzle view. Do NOT rely on this
    /// being up-to-date; the result of this function may change by the time it
    /// returns.
    pub(crate) fn has_active_puzzle_view(&self) -> bool {
        self.active_puzzle_view().is_some()
    }
    /// Returns whether the active puzzle view has a puzzle in it. Do NOT rely
    /// on this being up-to-date; the result of this function may change by the
    /// time it returns.
    pub(crate) fn has_active_puzzle(&self) -> bool {
        self.active_puzzle_type().is_some()
    }
    pub(crate) fn active_puzzle_type(&self) -> Option<Arc<Puzzle>> {
        self.with_active_puzzle_view(|puzzle_view| puzzle_view.puzzle())
    }
    pub(crate) fn active_puzzle_view(&self) -> Option<Arc<Mutex<Option<PuzzleWidget>>>> {
        self.active_puzzle_view.upgrade()
    }
    pub(crate) fn with_active_puzzle_view<R>(
        &self,
        f: impl FnOnce(&mut PuzzleWidget) -> R,
    ) -> Option<R> {
        let active_puzzle_view = self.active_puzzle_view()?;
        let mut puzzle_view_mutex_guard = active_puzzle_view.lock();
        Some(f(puzzle_view_mutex_guard.as_mut()?))
    }

    pub(crate) fn load_puzzle(&mut self, lib: &hyperpuzzle::Library, puzzle_id: &str) {
        if self.has_active_puzzle_view() {
            if let Some(new_puzzle_view) = PuzzleWidget::new(lib, &self.gfx, &self.prefs, puzzle_id)
            {
                self.set_active_puzzle(Some(new_puzzle_view))
            }
        }
    }
}

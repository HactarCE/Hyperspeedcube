use std::path::PathBuf;
use std::sync::{Arc, Weak};

use hyperpuzzle::Puzzle;
use parking_lot::Mutex;

use crate::gfx::GraphicsState;
use crate::gui::PuzzleWidget;
use crate::preferences::{
    AnimationPreferences, InteractionPreferences, ModifiedPreset, Preferences,
    PuzzleViewPreferencesSet,
};

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,

    pub active_puzzle_view: ActivePuzzleView,

    pub(crate) animation_prefs: ModifiedPreset<AnimationPreferences>,
    pub(crate) interaction_prefs: ModifiedPreset<InteractionPreferences>,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, _initial_file: Option<PathBuf>) -> Self {
        let prefs = Preferences::load(None);

        let animation_prefs = prefs.animation.load_last_loaded();
        let interaction_prefs = prefs.interaction.load_last_loaded();

        Self {
            gfx: Arc::new(GraphicsState::new(
                cc.wgpu_render_state.as_ref().expect("no wgpu render state"),
            )),

            prefs,

            active_puzzle_view: ActivePuzzleView::default(),

            animation_prefs,
            interaction_prefs,
        }
    }

    pub(crate) fn set_active_puzzle_view(
        &mut self,
        puzzle_view: &Arc<Mutex<Option<PuzzleWidget>>>,
    ) {
        self.active_puzzle_view = ActivePuzzleView::from(puzzle_view);
        self.notify_active_puzzle_changed();
    }
    pub(crate) fn set_active_puzzle(&mut self, new_puzzle_view: Option<PuzzleWidget>) {
        match self.active_puzzle_view.0.upgrade() {
            Some(puzzle_view) => {
                *puzzle_view.lock() = new_puzzle_view;
                self.notify_active_puzzle_changed();
            }
            None => log::warn!("No active puzzle view"),
        }
    }
    fn notify_active_puzzle_changed(&mut self) {
        self.active_puzzle_view.with(|p| {
            let view_prefs_set = PuzzleViewPreferencesSet::from_ndim(p.puzzle().ndim());
            self.prefs
                .view_presets_mut(view_prefs_set)
                .set_last_loaded(p.view.camera.view_preset.base.name());

            self.prefs
                .color_schemes
                .get_mut(&p.puzzle().colors)
                .schemes
                .set_last_loaded(p.view.colors.base.name());

            // TODO: add more presets here as relevant
            self.prefs.needs_save_eventually = true;
        });
    }

    pub(crate) fn load_puzzle(&mut self, lib: &hyperpuzzle::Library, puzzle_id: &str) {
        if self.active_puzzle_view.view().is_some() {
            if let Some(new_puzzle_view) =
                PuzzleWidget::new(lib, &self.gfx, &mut self.prefs, puzzle_id)
            {
                self.set_active_puzzle(Some(new_puzzle_view));
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ActivePuzzleView(Weak<Mutex<Option<PuzzleWidget>>>);
impl From<&Arc<Mutex<Option<PuzzleWidget>>>> for ActivePuzzleView {
    fn from(value: &Arc<Mutex<Option<PuzzleWidget>>>) -> Self {
        Self(Arc::downgrade(value))
    }
}
impl ActivePuzzleView {
    pub fn ty(&self) -> Option<Arc<Puzzle>> {
        self.with(|p| p.puzzle())
    }
    pub fn with<R>(&self, f: impl FnOnce(&mut PuzzleWidget) -> R) -> Option<R> {
        Some(f(self.0.upgrade()?.lock().as_mut()?))
    }
    pub fn with_opt<R>(&self, f: impl FnOnce(Option<&mut PuzzleWidget>) -> R) -> R {
        let mutex = self.0.upgrade();
        let mut mutex_guard = mutex.as_ref().map(|m| m.lock());
        f(mutex_guard.as_mut().and_then(|m| m.as_mut()))
    }

    /// Returns whether there is an active puzzle widget. It may not have a
    /// puzzle in it.
    pub fn view(&self) -> Option<Arc<Mutex<Option<PuzzleWidget>>>> {
        self.0.upgrade()
    }

    /// Returns whether there is an an active puzzle widget with a puzzle in it.
    pub fn has_puzzle(&self) -> bool {
        self.with(|_| ()).is_some()
    }
}

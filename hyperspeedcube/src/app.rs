use std::path::PathBuf;
use std::sync::{Arc, Weak};

use egui::mutex::RwLock;
use hyperdraw::GraphicsState;
use hyperprefs::{AnimationPreferences, ModifiedPreset, Preferences, PuzzleViewPreferencesSet};
use hyperpuzzle::{Puzzle, ScrambleParams, ScrambleType};
use hyperpuzzleview::ReplayEvent;
use parking_lot::Mutex;
use rand::Rng;

use crate::gui::PuzzleWidget;
use crate::L;

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,

    pub active_puzzle_view: ActivePuzzleView,

    pub(crate) animation_prefs: ModifiedPreset<AnimationPreferences>,

    egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, _initial_file: Option<PathBuf>) -> Self {
        let prefs = Preferences::load(None);

        let animation_prefs = prefs
            .animation
            .load_last_loaded(&L.presets.default_preset_name);

        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("no wgpu render state");
        Self {
            gfx: Arc::new(GraphicsState::new(
                Arc::clone(&wgpu_render_state.device),
                Arc::clone(&wgpu_render_state.queue),
            )),

            prefs,

            active_puzzle_view: ActivePuzzleView::default(),

            animation_prefs,

            egui_wgpu_renderer: Arc::clone(&wgpu_render_state.renderer),
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

    pub(crate) fn load_puzzle(&mut self, puzzle_id: &str) {
        if self.active_puzzle_view.view().is_some() {
            let egui_wgpu_renderer = Arc::clone(&self.egui_wgpu_renderer);
            if let Some(new_puzzle_view) =
                PuzzleWidget::new(&self.gfx, egui_wgpu_renderer, &mut self.prefs, puzzle_id)
            {
                self.set_active_puzzle(Some(new_puzzle_view));
            }
        }
    }

    fn prompt_file_save_path() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Hyperspeedcube Log Files", &["hsc"])
            .add_filter("All files", &["*"])
            .save_file()
    }
    fn prompt_file_load_path() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Hyperspeedcube Log Files", &["hsc"])
            .add_filter("All files", &["*"])
            .pick_file()
    }

    pub(crate) fn save_file(&mut self) {
        if let Some(contents) = self.serialize_puzzle_log() {
            let last_file_path = self
                .active_puzzle_view
                .with(|p| p.sim().lock().last_log_file.clone())
                .flatten();
            if let Some(path) = last_file_path.or_else(Self::prompt_file_save_path) {
                // TODO: handle error
                std::fs::write(path.clone(), contents);
                self.active_puzzle_view
                    .with(|p| p.sim().lock().last_log_file = Some(path));
            }
        }
    }
    pub(crate) fn save_file_as(&mut self) {
        if let Some(contents) = self.serialize_puzzle_log() {
            if let Some(path) = Self::prompt_file_save_path() {
                // TODO: handle error
                std::fs::write(path.clone(), contents);
                self.active_puzzle_view
                    .with(|p| p.sim().lock().last_log_file = Some(path));
            }
        }
    }
    pub(crate) fn serialize_puzzle_log(&mut self) -> Option<String> {
        let solve = self.active_puzzle_view.with(|p| p.sim().lock().serialize());
        Some(
            hyperpuzzlelog::LogFile {
                program: Some(crate::PROGRAM.clone()),
                solves: vec![solve?],
            }
            .serialize(),
        )
    }
    pub(crate) fn open_file(&mut self) {
        if let Some(path) = Self::prompt_file_load_path() {
            // TODO: handle error
            if let Ok(contents) = std::fs::read_to_string(path) {
                self.paste_from_string(&contents);
            }
        }
    }
    pub(crate) fn paste_from_string(&mut self, s: &str) {
        // TODO: handle error
        match hyperpuzzlelog::LogFile::deserialize(s) {
            Ok(log_file) => {
                // TODO: load multiple solves at once
                if let Some(first_solve) = log_file.solves.first() {
                    // TODO: new tab if none exists
                    self.active_puzzle_view.with(|p| {
                        // TODO: check puzzle version
                        // TODO: don't block
                        match crate::LIBRARY
                            .with(|lib| lib.build_puzzle(&first_solve.puzzle.id))
                            .take_result_blocking()
                        {
                            Ok(puzzle) => {
                                *p.sim().lock() = hyperpuzzleview::PuzzleSimulation::deserialize(
                                    &puzzle,
                                    first_solve,
                                );
                            }
                            Err(e) => {
                                log::error!("error constructing puzzle specified in log file: {e}");
                            }
                        }
                    });
                }
            }
            Err(e) => log::error!("error loading log file: {e}"),
        }
    }

    pub(crate) fn has_undo(&self) -> bool {
        self.active_puzzle_view
            .with(|p| p.sim().lock().has_undo())
            .unwrap_or(false)
    }
    pub(crate) fn has_redo(&self) -> bool {
        self.active_puzzle_view
            .with(|p| p.sim().lock().has_redo())
            .unwrap_or(false)
    }
    pub(crate) fn undo(&self) {
        self.active_puzzle_view
            .with(|p| p.sim().lock().event(ReplayEvent::Undo));
    }
    pub(crate) fn redo(&self) {
        self.active_puzzle_view
            .with(|p| p.sim().lock().event(ReplayEvent::Redo));
    }
    pub(crate) fn reset_puzzle(&self) {
        self.active_puzzle_view.with(|p| p.sim().lock().reset());
    }
    pub(crate) fn scramble(&self, ty: ScrambleType) {
        self.active_puzzle_view.with(|p| {
            p.sim().lock().scramble(ScrambleParams::new(ty));
        });
    }

    /// Shows a dialog asking the user to confirm discarding the puzzle state,
    /// and returns `true` if they do confirm. The dialog is only shown if there
    /// are unsaved changes that the user might want to save.
    ///
    /// Returns `false` if there is no puzzle.
    pub(crate) fn confirm_discard_changes(&mut self, description: &str) -> bool {
        self.active_puzzle_view
            .with(|p| {
                let sim = p.sim().lock();

                let mut needs_save = sim.has_unsaved_changes();

                if self.prefs.interaction.confirm_discard_only_when_scrambled
                    && !sim.has_been_fully_scrambled()
                {
                    needs_save = false;
                }

                let confirm = !needs_save
                    || rfd::MessageDialog::new()
                        .set_title(L.confirm_discard.title)
                        .set_description(description)
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .show()
                        == rfd::MessageDialogResult::Yes;
                if confirm {
                    self.prefs.log_file = None;
                    self.prefs.needs_save = true;
                }
                confirm
            })
            .unwrap_or(false)
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

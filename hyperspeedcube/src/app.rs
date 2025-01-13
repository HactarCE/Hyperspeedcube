use std::path::PathBuf;
use std::sync::{Arc, Weak};

use egui::mutex::RwLock;
use hyperdraw::GraphicsState;
use hyperprefs::{AnimationPreferences, ModifiedPreset, Preferences, PuzzleViewPreferencesSet};
use hyperpuzzle::{Puzzle, ScrambleParams, ScrambleType};
use hyperpuzzle_log::Solve;
use hyperpuzzle_view::{PuzzleSimulation, PuzzleView, ReplayEvent};
use hyperstats::StatsDb;
use parking_lot::Mutex;

use crate::gui::PuzzleWidget;
use crate::L;

pub struct App {
    pub(crate) gfx: Arc<GraphicsState>,

    pub(crate) prefs: Preferences,
    pub(crate) stats: StatsDb,

    pub active_puzzle: ActivePuzzleWidget,

    pub(crate) animation_prefs: ModifiedPreset<AnimationPreferences>,

    egui_wgpu_renderer: Arc<RwLock<eframe::egui_wgpu::Renderer>>,
}

impl App {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, _initial_file: Option<PathBuf>) -> Self {
        let prefs = Preferences::load(None);
        let stats = hyperstats::load();

        let animation_prefs = prefs
            .animation
            .load_last_loaded(hyperprefs::DEFAULT_PRESET_NAME);

        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("no wgpu render state");
        Self {
            gfx: Arc::new(GraphicsState::new(
                Arc::clone(&wgpu_render_state.device),
                Arc::clone(&wgpu_render_state.queue),
            )),

            prefs,
            stats,

            active_puzzle: ActivePuzzleWidget::default(),

            animation_prefs,

            egui_wgpu_renderer: Arc::clone(&wgpu_render_state.renderer),
        }
    }

    pub(crate) fn update_active_puzzle(&mut self, new_puzzle_widget: &Arc<Mutex<PuzzleWidget>>) {
        self.active_puzzle = ActivePuzzleWidget(Arc::downgrade(new_puzzle_widget));
        self.notify_active_puzzle_changed();
    }
    pub(super) fn notify_active_puzzle_changed(&mut self) {
        self.active_puzzle.with_view(|view| {
            let view_prefs_set = PuzzleViewPreferencesSet::from_ndim(view.puzzle().ndim());
            self.prefs
                .view_presets_mut(view_prefs_set)
                .set_last_loaded(view.camera.view_preset.base.name());

            self.prefs
                .color_schemes
                .get_mut(&view.puzzle().colors)
                .schemes
                .set_last_loaded(view.colors.base.name());

            // TODO: add more presets here as relevant
            self.prefs.needs_save_eventually = true;
        });
    }

    pub(crate) fn new_puzzle_widget(&self) -> Arc<Mutex<PuzzleWidget>> {
        Arc::new(Mutex::new(PuzzleWidget::new(
            &self.gfx,
            &self.egui_wgpu_renderer,
        )))
    }

    pub(crate) fn load_puzzle(&mut self, puzzle_id: &str) {
        if let Some(puzzle_widget) = self.active_puzzle.widget() {
            puzzle_widget.lock().load_puzzle(puzzle_id, &mut self.prefs);
        }
    }
    pub(crate) fn load_solve(&mut self, solve: Solve) {
        if let Some(puzzle_widget) = self.active_puzzle.widget() {
            puzzle_widget
                .lock()
                .load_solve(Arc::new(solve), &mut self.prefs);
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
                .active_puzzle
                .with_sim(|sim| sim.last_log_file.clone())
                .flatten();
            if let Some(path) = last_file_path.or_else(Self::prompt_file_save_path) {
                // TODO: handle error
                std::fs::write(path.clone(), contents);
                self.active_puzzle
                    .with_sim(|sim| sim.last_log_file = Some(path));
            }
        }
    }
    pub(crate) fn save_file_as(&mut self) {
        if let Some(contents) = self.serialize_puzzle_log() {
            if let Some(path) = Self::prompt_file_save_path() {
                // TODO: handle error
                std::fs::write(path.clone(), contents);
                self.active_puzzle
                    .with_sim(|sim| sim.last_log_file = Some(path));
            }
        }
    }
    pub(crate) fn serialize_puzzle_log(&mut self) -> Option<String> {
        let solve = self.active_puzzle.with_sim(|sim| sim.serialize());
        Some(
            hyperpuzzle_log::LogFile {
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
        match hyperpuzzle_log::LogFile::deserialize(s) {
            Ok((log_file, warnings)) => {
                for warning in warnings {
                    log::warn!("warning while loading log file: {warning}");
                }

                // TODO: load multiple solves at once
                if let Some(first_solve) = log_file.solves.into_iter().next() {
                    self.load_solve(first_solve);
                }
            }
            Err(e) => log::error!("error loading log file: {e}"),
        }
    }

    pub(crate) fn has_undo(&self) -> bool {
        self.active_puzzle
            .with_sim(|sim| sim.has_undo())
            .unwrap_or(false)
    }
    pub(crate) fn has_redo(&self) -> bool {
        self.active_puzzle
            .with_sim(|sim| sim.has_redo())
            .unwrap_or(false)
    }
    pub(crate) fn undo(&self) {
        self.active_puzzle
            .with_sim(|sim| sim.do_event(ReplayEvent::Undo));
    }
    pub(crate) fn redo(&self) {
        self.active_puzzle
            .with_sim(|sim| sim.do_event(ReplayEvent::Redo));
    }
    pub(crate) fn reset_puzzle(&self) {
        self.active_puzzle.with_sim(|sim| sim.reset());
    }
    pub(crate) fn scramble(&self, ty: ScrambleType) {
        self.active_puzzle.with_sim(|sim| {
            sim.scramble(ScrambleParams::new(ty));
        });
    }

    /// Shows a dialog asking the user to confirm discarding the puzzle state,
    /// and returns `true` if they do confirm. The dialog is only shown if there
    /// are unsaved changes that the user might want to save.
    ///
    /// Returns `false` if there is no puzzle.
    pub(crate) fn confirm_discard_changes(&mut self, description: &str) -> bool {
        self.active_puzzle
            .with_sim(|sim| {
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
pub struct ActivePuzzleWidget(Weak<Mutex<PuzzleWidget>>);
impl From<&Arc<Mutex<PuzzleWidget>>> for ActivePuzzleWidget {
    fn from(value: &Arc<Mutex<PuzzleWidget>>) -> Self {
        Self(Arc::downgrade(value))
    }
}
impl ActivePuzzleWidget {
    pub fn ty(&self) -> Option<Arc<Puzzle>> {
        self.with_view(|v| v.puzzle())
    }

    pub fn contains(&self, other: &Arc<Mutex<PuzzleWidget>>) -> bool {
        self.0.ptr_eq(&Arc::downgrade(other))
    }

    /// Returns the active puzzle widget, if any.
    pub fn widget(&self) -> Option<Arc<Mutex<PuzzleWidget>>> {
        self.0.upgrade()
    }
    pub fn with_widget<R>(&self, f: impl FnOnce(&mut PuzzleWidget) -> R) -> Option<R> {
        Some(f(&mut self.widget()?.lock()))
    }
    pub fn with_view<R>(&self, f: impl FnOnce(&mut PuzzleView) -> R) -> Option<R> {
        Some(f(self.widget()?.lock().view_mut()?))
    }
    pub fn with_opt_view<R>(&self, f: impl FnOnce(Option<&mut PuzzleView>) -> R) -> R {
        let mutex = self.widget();
        let mut mutex_guard = mutex.as_ref().map(|m| m.lock());
        f(mutex_guard.as_mut().and_then(|m| m.view_mut()))
    }
    pub fn with_sim<R>(&self, f: impl FnOnce(&mut PuzzleSimulation) -> R) -> Option<R> {
        self.with_view(|v| f(&mut v.sim.lock()))
    }

    /// Returns whether there is an an active puzzle widget with a puzzle in it.
    pub fn has_puzzle(&self) -> bool {
        self.with_view(|_| ()).is_some()
    }
}

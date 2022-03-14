use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use winit::event::{ElementState, ModifiersState, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::commands::{Command, PuzzleCommand, SelectCategory};
use crate::preferences::{Key, KeyCombo, Preferences};
use crate::puzzle::{
    Face, LayerMask, Puzzle, PuzzleController, PuzzleControllerTrait, PuzzleType, Selection,
    TwistDirection,
};
use crate::render::PuzzleRenderCache;

pub struct App {
    pub(crate) prefs: Preferences,

    events: EventLoopProxy<AppEvent>,

    pub(crate) puzzle: Puzzle,
    pub(crate) render_cache: PuzzleRenderCache,
    pub(crate) wants_repaint: bool,

    /// Set of pressed modifier keys.
    modifiers: ModifiersState,
    /// Selections tied to a held key.
    held_selections: HashMap<Key, Selection>,
    /// Semi-permanent selections.
    toggle_selections: Selection,

    status_msg: String,
}
impl App {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let mut this = Self {
            prefs: Preferences::load(None),

            events: event_loop.create_proxy(),

            puzzle: Puzzle::default(),
            render_cache: PuzzleRenderCache::default(),
            wants_repaint: true,

            modifiers: ModifiersState::default(),
            held_selections: HashMap::new(),
            toggle_selections: Selection::default(),

            status_msg: String::new(),
        };

        // Always save preferences after opening.
        this.prefs.needs_save = true;

        // Load last open file.
        if let Some(path) = this.prefs.log_file.take() {
            this.try_load_puzzle(path);
        }

        this
    }

    pub(crate) fn event(&self, event: impl Into<AppEvent>) {
        self.events
            .send_event(event.into())
            .expect("tried to send event but event loop doesn't exist")
    }

    pub(crate) fn handle_app_event(&mut self, event: AppEvent, control_flow: &mut ControlFlow) {
        self.clear_status();
        match event {
            AppEvent::Command(c) => match c {
                Command::Open => {
                    if self.confirm_discard_changes("open another file") {
                        if let Some(path) = file_dialog().pick_file() {
                            self.try_load_puzzle(path);
                        }
                    }
                }
                Command::Save => match self.prefs.log_file.clone() {
                    Some(path) => self.try_save_puzzle(&path),
                    None => self.try_save_puzzle_as(),
                },
                Command::SaveAs => self.try_save_puzzle_as(),
                Command::Exit => {
                    if self.confirm_discard_changes("exit") {
                        *control_flow = ControlFlow::Exit;
                    }
                }

                Command::Undo => {
                    if !self.puzzle.undo() {
                        self.set_status_err("Nothing to undo");
                    }
                }
                Command::Redo => {
                    if !self.puzzle.redo() {
                        self.set_status_err("Nothing to redo");
                    }
                }
                Command::Reset => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = Puzzle::new(self.puzzle.ty());
                        self.wants_repaint = true;
                    }
                }

                Command::NewPuzzle(puzzle_type) => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = Puzzle::new(puzzle_type);
                        self.wants_repaint = true;
                        self.prefs.log_file = None;
                        self.set_status_ok(format!("Loaded {}", puzzle_type));
                    }
                }

                Command::None => (),
            },

            AppEvent::Twist {
                face,
                direction,
                layer_mask,
            } => {
                if let Err(e) = self.puzzle.do_twist_command(face, direction, layer_mask) {
                    self.set_status_err(e);
                }
            }
            AppEvent::Recenter(face) => {
                if let Err(e) = self.puzzle.do_recenter_command(face) {
                    self.set_status_err(e);
                }
            }

            AppEvent::StatusError(msg) => self.set_status_err(msg),
        }
    }
    pub(crate) fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.event(Command::Exit),

            WindowEvent::DroppedFile(path) => {
                if self.confirm_discard_changes("open another file") {
                    self.try_load_puzzle(path.to_owned());
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = *mods;
                // Sometimes we miss key events for modifiers when the left and
                // right modifiers are both pressed at once (at least in my
                // testing on Windows 11) so clean that up here just in case.
                self.remove_held_selections(|k| {
                    // If the selection requires a modifier and that modifier is
                    // not pressed, then remove the selection.
                    k.is_shift() && !mods.shift()
                        || k.is_ctrl() && !mods.ctrl()
                        || k.is_alt() && !mods.alt()
                        || k.is_logo() && !mods.logo()
                });
            }

            WindowEvent::KeyboardInput { input, .. } => {
                let sc = key_names::sc_to_key(input.scancode as u16).map(Key::Sc);
                let vk = input.virtual_keycode.map(Key::Vk);
                let is_shift =
                    sc.map_or(false, |k| k.is_shift()) || vk.map_or(false, |k| k.is_shift());
                let is_ctrl =
                    sc.map_or(false, |k| k.is_ctrl()) || vk.map_or(false, |k| k.is_ctrl());
                let is_alt = sc.map_or(false, |k| k.is_alt()) || vk.map_or(false, |k| k.is_alt());
                let is_logo =
                    sc.map_or(false, |k| k.is_logo()) || vk.map_or(false, |k| k.is_logo());

                if input.state == ElementState::Released {
                    // Remove selections for this held key.
                    self.remove_held_selections(|k| Some(k) == sc || Some(k) == vk);
                    return;
                }

                let ignore_shift = is_shift || self.held_selections.keys().any(|k| k.is_shift());
                let ignore_ctrl = is_ctrl || self.held_selections.keys().any(|k| k.is_ctrl());
                let ignore_alt = is_alt || self.held_selections.keys().any(|k| k.is_alt());
                let ignore_logo = is_logo || self.held_selections.keys().any(|k| k.is_logo());

                // We don't care about left vs. right modifiers, so just extract
                // the bits that don't specify left vs. right.
                let mods = self.modifiers
                    & (ModifiersState::SHIFT
                        | ModifiersState::CTRL
                        | ModifiersState::ALT
                        | ModifiersState::LOGO);

                let key_combo_matches = |key_combo: KeyCombo| match key_combo.key() {
                    Some(k) => {
                        (Some(k) == sc || Some(k) == vk)
                            && (key_combo.shift() == mods.shift() || ignore_shift)
                            && (key_combo.ctrl() == mods.ctrl() || ignore_ctrl)
                            && (key_combo.alt() == mods.alt() || ignore_alt)
                            && (key_combo.logo() == mods.logo() || ignore_logo)
                    }
                    None => false,
                };

                for bind in &self.prefs.puzzle_keybinds[self.puzzle.ty()] {
                    if key_combo_matches(bind.key) {
                        let sel = self.puzzle_selection();

                        match &bind.command {
                            PuzzleCommand::Twist {
                                face,
                                direction,
                                layer_mask,
                            } => {
                                if let Some(face) =
                                    face.or_else(|| sel.exactly_one_face(self.puzzle.ty()))
                                {
                                    self.event(AppEvent::Twist {
                                        face,
                                        direction: *direction,
                                        layer_mask: sel.layer_mask_or_default(*layer_mask),
                                    });
                                } else {
                                    self.event(AppEvent::StatusError(
                                        "No face selected".to_string(),
                                    ));
                                }
                            }
                            PuzzleCommand::Recenter { face } => {
                                if let Some(face) =
                                    face.or_else(|| sel.exactly_one_face(self.puzzle.ty()))
                                {
                                    self.event(AppEvent::Recenter(face));
                                } else {
                                    self.event(AppEvent::StatusError(
                                        "No face selected".to_string(),
                                    ));
                                }
                            }

                            PuzzleCommand::HoldSelect(thing) => {
                                let sel = Selection::from(*thing);
                                let old_sel =
                                    self.held_selections.insert(bind.key.key().unwrap(), sel);
                                self.wants_repaint = old_sel != Some(sel);
                            }
                            PuzzleCommand::ToggleSelect(thing) => {
                                self.toggle_selections ^= Selection::from(*thing);
                                self.wants_repaint = true;
                            }
                            PuzzleCommand::ClearToggleSelect(category) => {
                                let default = Selection::default();
                                let tog_sel = &mut self.toggle_selections;
                                let old_tog_sel = *tog_sel;

                                use SelectCategory::*;
                                match category {
                                    Face => tog_sel.face_mask = default.face_mask,
                                    Layers => tog_sel.layer_mask = default.layer_mask,
                                    PieceType => tog_sel.piece_type_mask = default.piece_type_mask,
                                }
                                self.wants_repaint |= old_tog_sel != *tog_sel;
                            }

                            PuzzleCommand::None => return, // Do not try to match other keybinds.
                        }
                    }
                }

                for bind in &self.prefs.general_keybinds {
                    if key_combo_matches(bind.key) {
                        match &bind.command {
                            Command::None => return, // Do not try to match other keybinds.
                            other => self.event(other.clone()),
                        }
                    }
                }
            }

            _ => (),
        }
    }

    pub(crate) fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub(crate) fn frame(&mut self, delta: Duration) {
        self.wants_repaint |= self.puzzle.advance(delta);
    }

    fn confirm_discard_changes(&self, action: &str) -> bool {
        !self.puzzle.is_unsaved()
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(&format!("Discard puzzle state and {}?", action))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
    }

    fn try_load_puzzle(&mut self, path: PathBuf) {
        match PuzzleController::load_file(&path) {
            Ok(p) => {
                self.puzzle = Puzzle::Rubiks4D(p);
                self.wants_repaint = true;

                self.set_status_ok(format!("Loaded log file from {}", path.display()));

                self.prefs.log_file = Some(path);
                self.prefs.needs_save = true;
            }
            Err(e) => show_error_dialog("Unable to load log file", e),
        }
    }
    fn try_save_puzzle(&mut self, path: &Path) {
        match &mut self.puzzle {
            Puzzle::Rubiks4D(p) => match p.save_file(path) {
                Ok(()) => {
                    self.prefs.log_file = Some(path.to_path_buf());
                    self.prefs.needs_save = true;

                    self.set_status_ok(format!("Saved log file to {}", path.display()));
                }
                Err(e) => show_error_dialog("Unable to save log file", e),
            },
            _ => show_error_dialog(
                "Unable to save log file",
                format!("Log files are only supported for {}.", PuzzleType::Rubiks4D),
            ),
        }
    }
    fn try_save_puzzle_as(&mut self) {
        if let Some(path) = file_dialog().save_file() {
            self.try_save_puzzle(&path)
        }
    }

    pub(crate) fn status_msg(&self) -> &str {
        &self.status_msg
    }
    fn clear_status(&mut self) {
        self.status_msg = String::new();
    }
    fn set_status_ok(&mut self, msg: impl fmt::Display) {
        self.status_msg = msg.to_string()
    }
    fn set_status_err(&mut self, msg: impl fmt::Display) {
        self.status_msg = format!("Error: {}", msg)
    }

    pub(crate) fn puzzle_selection(&self) -> Selection {
        let mut ret = self
            .held_selections
            .values()
            .copied()
            .reduce(|a, b| a | b)
            .unwrap_or(self.toggle_selections);
        ret.face_mask |= self.toggle_selections.face_mask;
        if ret.layer_mask == 0 {
            ret.layer_mask = self.toggle_selections.layer_mask;
        }
        if self
            .held_selections
            .values()
            .all(|s| s.piece_type_mask == 0)
        {
            ret.piece_type_mask = self.toggle_selections.piece_type_mask;
        }
        ret
    }
    fn remove_held_selections(&mut self, mut remove_if: impl FnMut(Key) -> bool) {
        self.held_selections.retain(|&k, _v| !remove_if(k));
        self.wants_repaint = true;
    }
}

#[derive(Debug)]
pub(crate) enum AppEvent {
    Command(Command),

    Twist {
        face: Face,
        direction: TwistDirection,
        layer_mask: LayerMask,
    },
    Recenter(Face),

    StatusError(String),
}
impl From<Command> for AppEvent {
    fn from(c: Command) -> Self {
        Self::Command(c)
    }
}

fn file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter("Magic Cube 4D Log Files", &["log"])
        .add_filter("All files", &["*"])
}
fn show_error_dialog(title: &str, e: impl fmt::Display) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&e.to_string())
        .show();
}

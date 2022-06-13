use cgmath::Point2;
use key_names::KeyMappingCode;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use winit::event::{ElementState, ModifiersState, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

use crate::commands::{Command, PuzzleCommand, SelectCategory};
use crate::controller::PuzzleController;
use crate::preferences::{Key, Keybind, Preferences};
use crate::puzzle::{Face, LayerMask, Selection, Twist, TwistDirection, TwistDirection2D};
use crate::render::{GraphicsState, PuzzleRenderCache};

pub struct App {
    pub(crate) prefs: Preferences,

    events: EventLoopProxy<AppEvent>,

    pub(crate) puzzle: PuzzleController,
    pub(crate) render_cache: PuzzleRenderCache,
    wants_to_redraw_puzzle: bool,

    /// Mouse cursor position relative to the puzzle texture.
    pub(crate) cursor_pos: Option<Point2<f32>>,
    /// Set of pressed keys.
    pressed_keys: HashSet<Key>,
    /// Set of pressed modifier keys.
    pressed_modifiers: ModifiersState,
    /// Selections tied to a held key.
    held_selections: HashMap<Key, Selection>,
    /// Semi-permanent selections.
    pub(crate) toggle_selections: Selection,

    status_msg: String,
}
impl App {
    pub(crate) fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let mut this = Self {
            prefs: Preferences::load(None),

            events: event_loop.create_proxy(),

            puzzle: PuzzleController::default(),
            render_cache: PuzzleRenderCache::default(),
            wants_to_redraw_puzzle: true,

            cursor_pos: None,
            pressed_keys: HashSet::default(),
            pressed_modifiers: ModifiersState::default(),
            held_selections: HashMap::default(),
            toggle_selections: Selection::default(),

            status_msg: String::default(),
        };

        // Always save preferences after opening.
        this.prefs.needs_save = true;

        // Load last open file.
        if let Some(path) = this.prefs.log_file.take() {
            this.try_load_puzzle(path);
        }

        this
    }

    pub(crate) fn request_redraw_puzzle(&mut self) {
        self.wants_to_redraw_puzzle = true;
    }
    pub(crate) fn draw_puzzle(
        &mut self,
        gfx: &mut GraphicsState,
        texture_size: (u32, u32),
    ) -> Option<wgpu::TextureView> {
        let ret = crate::render::draw_puzzle(self, gfx, texture_size, self.wants_to_redraw_puzzle);
        self.wants_to_redraw_puzzle = false;
        ret
    }

    pub(crate) fn event(&self, event: impl Into<AppEvent>) {
        self.events
            .send_event(event.into())
            .expect("tried to send event but event loop doesn't exist")
    }

    pub(crate) fn handle_app_event(&mut self, event: AppEvent, control_flow: &mut ControlFlow) {
        self.clear_status();
        if let Err(e) = self.handle_app_event_internal(event, control_flow) {
            self.set_status_err(e);
        }
    }
    fn handle_app_event_internal(
        &mut self,
        event: AppEvent,
        control_flow: &mut ControlFlow,
    ) -> Result<(), String> {
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
                    self.puzzle.undo()?;
                }
                Command::Redo => {
                    self.puzzle.redo()?;
                }
                Command::Reset => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle.reset();
                    }
                }

                Command::ScrambleN(n) => {
                    if self.confirm_discard_changes("scramble") {
                        // TODO: `n` is not validated. It may be `0` (which is
                        // harmless) or `usize::MAX` (which will freeze the
                        // program).
                        self.puzzle.scramble_n(n)?;
                        self.set_status_ok(format!(
                            "Scrambled {} random {}",
                            n,
                            if n == 1 { "move" } else { "moves" }
                        ));
                    }
                }
                Command::ScrambleFull => {
                    if self.confirm_discard_changes("scramble") {
                        self.puzzle.scramble_full()?;
                        self.set_status_ok(format!("Scrambled fully",));
                    }
                }

                Command::NewPuzzle(puzzle_type) => {
                    if self.confirm_discard_changes("reset puzzle") {
                        self.puzzle = PuzzleController::new(puzzle_type);
                        self.prefs.log_file = None;
                        self.set_status_ok(format!("Loaded {}", puzzle_type));
                    }
                }

                Command::ToggleBlindfold => {
                    self.prefs.colors.blindfold ^= true;
                    self.prefs.needs_save = true;
                    self.request_redraw_puzzle();
                }

                Command::SwitchView => {
                    self.puzzle.target_view_mode = 1.0 - self.puzzle.target_view_mode.round();
                }

                Command::None => (),
            },

            AppEvent::Twist {
                face,
                direction,
                layer_mask,
            } => self.puzzle.twist(Twist::from_face_with_layers(
                face,
                direction.name(),
                layer_mask,
            )?)?,
            AppEvent::Recenter(face) => self.puzzle.twist(Twist::from_face_recenter(face)?)?,

            AppEvent::Click(egui::PointerButton::Primary) => {
                if let Some(sticker) = self.puzzle.hovered_sticker() {
                    self.puzzle.twist(Twist::from_sticker(
                        sticker,
                        TwistDirection2D::CCW,
                        self.puzzle_selection()
                            .layer_mask_or_default(LayerMask::default()),
                    )?)?;
                }
            }
            AppEvent::Click(egui::PointerButton::Secondary) => {
                if let Some(sticker) = self.puzzle.hovered_sticker() {
                    self.puzzle.twist(Twist::from_sticker(
                        sticker,
                        TwistDirection2D::CW,
                        self.puzzle_selection()
                            .layer_mask_or_default(LayerMask::default()),
                    )?)?;
                }
            }
            AppEvent::Click(egui::PointerButton::Middle) => {
                if let Some(sticker) = self.puzzle.hovered_sticker() {
                    self.puzzle
                        .twist(Twist::from_face_recenter(sticker.face())?)?;
                }
            }

            AppEvent::StatusError(msg) => return Err(msg),
        }
        Ok(())
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
                self.pressed_modifiers = *mods;
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
                let sc = key_names::sc_to_key(input.scancode as u16);
                let vk = input.virtual_keycode;

                match input.state {
                    ElementState::Pressed => {
                        if let Some(sc) = sc {
                            self.pressed_keys.insert(Key::Sc(sc));
                        }
                        if let Some(vk) = vk {
                            self.pressed_keys.insert(Key::Vk(vk));
                        }

                        // Only allow one twist command per keypress. Macros are
                        // icky.
                        let mut done_twist_command = false;

                        let puzzle_keybinds = &self.prefs.puzzle_keybinds[self.puzzle.ty()];
                        for bind in self.resolve_keypress(puzzle_keybinds, sc, vk) {
                            match &bind.command {
                                PuzzleCommand::Twist {
                                    face,
                                    direction,
                                    layer_mask,
                                } => {
                                    if !done_twist_command {
                                        done_twist_command = true;
                                        self.do_twist(*face, *direction, *layer_mask);
                                    }
                                }
                                PuzzleCommand::Recenter { face } => {
                                    if !done_twist_command {
                                        done_twist_command = true;
                                        self.do_recenter(*face);
                                    }
                                }

                                PuzzleCommand::HoldSelect(thing) => {
                                    let sel = Selection::from(*thing);
                                    self.held_selections.insert(bind.key.key().unwrap(), sel);
                                }
                                PuzzleCommand::ToggleSelect(thing) => {
                                    self.toggle_selections ^= Selection::from(*thing);
                                }
                                PuzzleCommand::ClearToggleSelect(category) => {
                                    let default = Selection::default();
                                    let tog_sel = &mut self.toggle_selections;

                                    use SelectCategory::*;
                                    match category {
                                        Face => tog_sel.face_mask = default.face_mask,
                                        Layers => tog_sel.layer_mask = default.layer_mask,
                                        PieceType => {
                                            tog_sel.piece_type_mask = default.piece_type_mask
                                        }
                                    }
                                }

                                PuzzleCommand::None => return, // Do not try to match other keybinds.
                            }
                        }

                        for bind in self.resolve_keypress(&self.prefs.general_keybinds, sc, vk) {
                            match &bind.command {
                                Command::None => return, // Do not try to match other keybinds.

                                _ => self.event(bind.command.clone()),
                            }
                        }
                    }

                    ElementState::Released => {
                        if let Some(sc) = sc {
                            self.pressed_keys.remove(&Key::Sc(sc));
                        }
                        if let Some(vk) = vk {
                            self.pressed_keys.remove(&Key::Vk(vk));
                        }

                        // Remove selections for this held key.
                        self.remove_held_selections(|k| {
                            Some(k) == sc.map(Key::Sc) || Some(k) == vk.map(Key::Vk)
                        });
                    }
                }
            }

            _ => (),
        }
    }

    pub(crate) fn resolve_keypress<'a, C>(
        &self,
        keybinds: &'a [Keybind<C>],
        sc: Option<KeyMappingCode>,
        vk: Option<VirtualKeyCode>,
    ) -> Vec<&'a Keybind<C>> {
        let sc = sc.map(Key::Sc);
        let vk = vk.map(Key::Vk);

        // Sometimes, we want to ignore certain modifier keys when resolving a
        // keypress -- in particular, if another keybind has already consumed
        // the modifier.
        //
        // For example, if `Shift` is bound to "select layer 2," then a keybind
        // bound to `A` will still match `Shift`+`A` because `Shift` is in the
        // "ignored modifiers" set.
        //
        // A modifier is also ignored when matching its own key, hence
        // `.chain(&sc).chain(&vk)`. For example, the shift modifier is ignored
        // when matching the shift key.
        let modifiers_mask = self.held_selections.keys().chain(&sc).chain(&vk).fold(
            // Consider all modifiers, but don't distinguish left vs. right.
            ModifiersState::SHIFT
                | ModifiersState::CTRL
                | ModifiersState::ALT
                | ModifiersState::LOGO,
            // Ignore held selections and the key currently being pressed.
            |mods, key| mods & !key.modifier_bit(),
        );

        keybinds
            .iter()
            .filter(move |bind| {
                let key_combo = bind.key;
                let key = key_combo.key();
                let key_matches = (sc.is_some() && sc == key) || (vk.is_some() && vk == key);
                let mods_match =
                    key_combo.mods() & modifiers_mask == self.pressed_modifiers() & modifiers_mask;
                key_matches && mods_match
            })
            .collect()
    }

    pub(crate) fn do_twist(
        &self,
        face: Option<Face>,
        direction: TwistDirection,
        layer_mask: LayerMask,
    ) {
        let sel = self.puzzle_selection();

        if let Some(face) = face.or_else(|| sel.exactly_one_face(self.puzzle.ty())) {
            self.event(AppEvent::Twist {
                face,
                direction,
                layer_mask: sel.layer_mask_or_default(layer_mask),
            });
        } else {
            self.event(AppEvent::StatusError("No face selected".to_string()));
        }
    }
    pub(crate) fn do_recenter(&self, face: Option<Face>) {
        let sel = self.puzzle_selection();

        if let Some(face) = face.or_else(|| sel.exactly_one_face(self.puzzle.ty())) {
            self.event(AppEvent::Recenter(face));
        } else {
            self.event(AppEvent::StatusError("No face selected".to_string()));
        }
    }

    pub(crate) fn pressed_keys(&self) -> &HashSet<Key> {
        &self.pressed_keys
    }
    pub(crate) fn pressed_modifiers(&self) -> ModifiersState {
        self.pressed_modifiers
    }

    pub(crate) fn frame(&mut self, _delta: Duration) {
        self.puzzle.set_selection(self.puzzle_selection());
        if self.puzzle.check_just_solved() {
            self.set_status_ok("Solved!");
        }
    }

    fn confirm_discard_changes(&self, action: &str) -> bool {
        let mut needs_save = self.puzzle.is_unsaved();

        if self.prefs.interaction.confirm_discard_only_when_scrambled
            && !self.puzzle.has_been_fully_scrambled()
        {
            needs_save = false;
        }

        !needs_save
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(&format!("Discard puzzle state and {}?", action))
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
    }

    fn try_load_puzzle(&mut self, path: PathBuf) {
        match PuzzleController::load_file(&path) {
            Ok(p) => {
                self.puzzle = p;

                self.set_status_ok(format!("Loaded log file from {}", path.display()));

                self.prefs.log_file = Some(path);
                self.prefs.needs_save = true;
            }
            Err(e) => show_error_dialog("Unable to load log file", e),
        }
    }
    fn try_save_puzzle(&mut self, path: &Path) {
        match self.puzzle.save_file(path) {
            Ok(()) => {
                self.prefs.log_file = Some(path.to_path_buf());
                self.prefs.needs_save = true;

                self.set_status_ok(format!("Saved log file to {}", path.display()));
            }
            Err(e) => show_error_dialog("Unable to save log file", e),
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

    Click(egui::PointerButton),

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

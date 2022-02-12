#![allow(clippy::nonminimal_bool)]

use glium::glutin::event::*;
use itertools::Itertools;
use std::collections::HashMap;

use crate::commands::{Command, PuzzleCommand, SelectCategory, SelectThing};
use crate::preferences::{Key, KeyCombo};
use crate::puzzle::{traits::*, Face, LayerMask, Puzzle, PuzzleController, PuzzleType, Selection};

const SHIFT: ModifiersState = ModifiersState::SHIFT;
const CTRL: ModifiersState = ModifiersState::CTRL;
const ALT: ModifiersState = ModifiersState::ALT;
const LOGO: ModifiersState = ModifiersState::LOGO;

#[must_use = "call finish()"]
pub struct InputFrame<'a> {
    state: &'a mut State,
    puzzle: &'a Puzzle,
    command_queue: &'a mut Vec<Command>,
}
impl InputFrame<'_> {
    pub fn handle_event(&mut self, ev: &Event<'_, ()>) {
        match ev {
            // Handle WindowEvents.
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        self.command_queue.push(Command::Quit);
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if self.state.has_keyboard {
                            self.handle_key(*input);
                        }
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        self.state.modifiers = *new_modifiers;
                        // Sometimes we miss key events for modifiers when the
                        // left and right modifiers are both pressed at once (at
                        // least in my testing on Windows 11) so clean that up
                        // here just in case.
                        self.state.held_selections.retain(|&k, _v| {
                            // If the selection requires a modifier and that
                            // modifier is not pressed, then remove the
                            // selection.
                            !(k.is_shift() && !self.state.modifiers.shift()
                                || k.is_ctrl() && !self.state.modifiers.ctrl()
                                || k.is_alt() && !self.state.modifiers.alt()
                                || k.is_logo() && !self.state.modifiers.logo())
                        })
                    }

                    // Ignore other `WindowEvent`s.
                    _ => (),
                }
            }

            // Ignore non-`WindowEvent`s.
            _ => (),
        }
    }

    fn handle_key(&mut self, input: KeyboardInput) {
        let sc = key_names::sc_to_key(input.scancode as u16).map(Key::Sc);
        let vk = input.virtual_keycode.map(Key::Vk);
        let is_shift = sc.map_or(false, |k| k.is_shift()) || vk.map_or(false, |k| k.is_shift());
        let is_ctrl = sc.map_or(false, |k| k.is_ctrl()) || vk.map_or(false, |k| k.is_ctrl());
        let is_alt = sc.map_or(false, |k| k.is_alt()) || vk.map_or(false, |k| k.is_alt());
        let is_logo = sc.map_or(false, |k| k.is_logo()) || vk.map_or(false, |k| k.is_logo());

        if input.state == ElementState::Released {
            // Remove selections for this held key.
            self.state
                .held_selections
                .retain(|&k, _v| Some(k) != sc && Some(k) != vk);
            return;
        }

        let puzzle_type = self.puzzle.ty();

        // We don't care about left vs. right modifiers, so just extract
        // the bits that don't specify left vs. right.
        let modifiers = self.state.modifiers & (SHIFT | CTRL | ALT | LOGO);

        let prefs = crate::get_prefs();

        let ignore_shift = is_shift || self.state.held_selections.keys().any(|k| k.is_shift());
        let ignore_ctrl = is_ctrl || self.state.held_selections.keys().any(|k| k.is_ctrl());
        let ignore_alt = is_alt || self.state.held_selections.keys().any(|k| k.is_alt());
        let ignore_logo = is_logo || self.state.held_selections.keys().any(|k| k.is_logo());

        // All other modifiers must exactly match those of the keybind.
        let mods = self.state.modifiers;

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

        for bind in &prefs.puzzle_keybinds[puzzle_type] {
            if key_combo_matches(bind.key) {
                let sel = self.state.total_selection();

                match &bind.command {
                    PuzzleCommand::Twist {
                        face,
                        direction,
                        layer_mask,
                    } => {
                        if let Some(face) = face.or_else(|| sel.exactly_one_face(puzzle_type)) {
                            self.command_queue.push(Command::Twist {
                                face,
                                direction: *direction,
                                layer_mask: sel.layer_mask_or_default(*layer_mask),
                            });
                        } else {
                            self.command_queue
                                .push(Command::ErrorMsg("No face selected".to_string()))
                        }
                    }
                    PuzzleCommand::Recenter { face } => {
                        if let Some(face) = face.or_else(|| sel.exactly_one_face(puzzle_type)) {
                            self.command_queue.push(Command::Recenter(face));
                        } else {
                            self.command_queue
                                .push(Command::ErrorMsg("No face selected".to_string()))
                        }
                    }

                    PuzzleCommand::HoldSelect(thing) => {
                        self.state
                            .held_selections
                            .insert(bind.key.key().unwrap(), Selection::from(*thing));
                    }
                    PuzzleCommand::ToggleSelect(thing) => {
                        self.state.toggle_selections ^= Selection::from(*thing);
                    }
                    PuzzleCommand::ClearToggleSelect(category) => {
                        let default = Selection::default();
                        let tog_sel = &mut self.state.toggle_selections;

                        use SelectCategory::*;
                        match category {
                            Face => tog_sel.face_mask = default.face_mask,
                            Layers => tog_sel.layer_mask = default.layer_mask,
                            PieceType => tog_sel.piece_type_mask = default.piece_type_mask,
                        }
                    }

                    PuzzleCommand::None => return, // Do not try to match other keybinds.
                }
            }
        }

        for bind in &prefs.general_keybinds {
            if key_combo_matches(bind.key) {
                match &bind.command {
                    Command::None => return, // Do not try to match other keybinds.
                    other => self.command_queue.push(other.clone()),
                }
            }
        }
    }

    pub fn finish(self) {}
}

#[derive(Debug, Default)]
pub struct State {
    /// Set of pressed modifiers.
    modifiers: ModifiersState,
    /// Whether to handle keyboard input (false if it is captured by imgui).
    has_keyboard: bool,

    held_selections: HashMap<Key, Selection>,
    toggle_selections: Selection,
}
impl State {
    pub fn frame<'a>(
        &'a mut self,
        puzzle: &'a Puzzle,
        imgui_io: &imgui::Io,
        command_queue: &'a mut Vec<Command>,
    ) -> InputFrame<'a> {
        self.has_keyboard = !imgui_io.want_capture_keyboard;
        InputFrame {
            state: self,
            puzzle,
            command_queue,
        }
    }

    pub(crate) fn total_selection(&self) -> Selection {
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
}

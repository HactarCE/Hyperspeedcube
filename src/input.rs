#![allow(clippy::nonminimal_bool)]

use glium::glutin::event::*;
use itertools::Itertools;
use std::collections::HashMap;

use crate::preferences::Key;
use crate::puzzle::{
    traits::*, Command, Face, LayerMask, Puzzle, PuzzleController, PuzzleType, SelectCategory,
    SelectThing,
};

const SHIFT: ModifiersState = ModifiersState::SHIFT;
const CTRL: ModifiersState = ModifiersState::CTRL;
const ALT: ModifiersState = ModifiersState::ALT;
const LOGO: ModifiersState = ModifiersState::LOGO;

#[must_use = "call finish()"]
pub struct FrameInProgress<'a> {
    state: &'a mut State,
    puzzle: &'a mut Puzzle,
}
impl FrameInProgress<'_> {
    pub fn handle_event(&mut self, ev: &Event<'_, ()>) {
        match ev {
            // Handle WindowEvents.
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        // self.state.keys.update(*input); // TODO: probably delete this
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
        // TODO: this is massive and ugly and I hate it.
        let is_shift = sc.map(|sc| sc.is_shift()).unwrap_or_default()
            || vk.map(|vk| vk.is_shift()).unwrap_or_default();
        let is_ctrl = sc.map(|sc| sc.is_ctrl()).unwrap_or_default()
            || vk.map(|vk| vk.is_ctrl()).unwrap_or_default();
        let is_alt = sc.map(|sc| sc.is_alt()).unwrap_or_default()
            || vk.map(|vk| vk.is_alt()).unwrap_or_default();
        let is_logo = sc.map(|sc| sc.is_logo()).unwrap_or_default()
            || vk.map(|vk| vk.is_logo()).unwrap_or_default();

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

        let mut selection = self.state.total_selection();

        for bind in &prefs.keybinds[puzzle_type] {
            let bind_key = match bind.key {
                Some(k) => k,
                None => continue,
            };
            if (Some(bind_key) == sc || Some(bind_key) == vk)
                && (bind.shift == mods.shift() || ignore_shift)
                && (bind.ctrl == mods.ctrl() || ignore_ctrl)
                && (bind.alt == mods.alt() || ignore_alt)
                && (bind.logo == mods.logo() || ignore_logo)
            {
                match &bind.command {
                    Command::Twist {
                        face,
                        layers,
                        direction,
                    } => {
                        if let Some(face) = face.or_else(|| selection.exactly_one_face(puzzle_type))
                        {
                            let layers = selection.layers_mask_or_default(*layers);
                            if let Err(e) = self.puzzle.twist_from_command(face, layers, *direction)
                            {
                                // TODO handle error
                            }
                        }
                    }
                    Command::Recenter { face } => {
                        if let Some(face) = face.or_else(|| selection.exactly_one_face(puzzle_type))
                        {
                            if let Err(e) = self.puzzle.recenter_from_command(face) {
                                // TODO handle error
                            }
                        }
                    }

                    Command::HoldSelect(thing) => {
                        self.state
                            .held_selections
                            .insert(bind_key, Selection::from(*thing));
                    }
                    Command::ToggleSelect(thing) => {
                        self.state.toggle_selections ^= Selection::from(*thing);
                    }
                    Command::ClearToggleSelect(category) => {
                        let default = Selection::default();
                        let tog_sel = &mut self.state.toggle_selections;

                        use SelectCategory::*;
                        match category {
                            Face => tog_sel.faces_mask = default.faces_mask,
                            Layers => tog_sel.layers_mask = default.layers_mask,
                            PieceType => tog_sel.piece_types_mask = default.piece_types_mask,
                        }
                    }

                    Command::None => break, // Do not try to match other keybinds.
                }

                selection = self.state.total_selection();
            }
        }

        if modifiers == CTRL {
            match input.virtual_keycode {
                // Undo.
                Some(VirtualKeyCode::Z) => self.puzzle.undo(),
                // Redo.
                Some(VirtualKeyCode::Y) => self.puzzle.redo(),
                // Reset.
                Some(VirtualKeyCode::R) => println!("TODO reset puzzle state"),
                // Copy puzzle state.
                Some(VirtualKeyCode::C) => println!("TODO copy puzzle state"),
                // Paste puzzle state.
                Some(VirtualKeyCode::V) => println!("TODO paste puzzle state"),
                // Save file.
                Some(VirtualKeyCode::S) => match self.puzzle {
                    Puzzle::Rubiks3D(_) => eprintln!("error: can't save 3D puzzle"),
                    Puzzle::Rubiks4D(cube) => {
                        if let Err(e) = cube.save_file(&prefs.log_file) {
                            eprintln!("error: {}", e);
                        }
                    }
                },
                // Full scramble.
                Some(VirtualKeyCode::F) => println!("TODO full scramble"),
                _ => (),
            }
        }

        if modifiers == SHIFT | CTRL {
            match input.virtual_keycode {
                // Redo.
                Some(VirtualKeyCode::Z) => self.puzzle.redo(),
                _ => (),
            }
        }
    }

    pub fn finish(self) {
        let mut prefs = crate::get_prefs();

        let view_prefs = &mut prefs.view[self.puzzle.ty()];

        // TODO

        // let speed = 1.0_f32.to_radians();
        // if self.state.keys[VirtualKeyCode::Up] {
        //     view_prefs.theta += speed;
        // }
        // if self.state.keys[VirtualKeyCode::Down] {
        //     view_prefs.theta -= speed;
        // }
        // if self.state.keys[VirtualKeyCode::Right] {
        //     view_prefs.phi += speed;
        // }
        // if self.state.keys[VirtualKeyCode::Left] {
        //     view_prefs.phi -= speed;
        // }

        match self.puzzle {
            Puzzle::Rubiks3D(cube) => update_puzzle_display(cube, self.state.total_selection()),
            Puzzle::Rubiks4D(cube) => update_puzzle_display(cube, self.state.total_selection()),
        }
    }
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
        puzzle: &'a mut Puzzle,
        imgui_io: &imgui::Io,
    ) -> FrameInProgress<'a> {
        self.has_keyboard = !imgui_io.want_capture_keyboard;
        FrameInProgress {
            state: self,
            puzzle,
        }
    }

    fn total_selection(&self) -> Selection {
        let mut ret = self
            .held_selections
            .values()
            .copied()
            .reduce(|a, b| a | b)
            .unwrap_or(self.toggle_selections);
        ret.faces_mask |= self.toggle_selections.faces_mask;
        if ret.layers_mask == 0 {
            ret.layers_mask = self.toggle_selections.layers_mask;
        }
        if self
            .held_selections
            .values()
            .all(|s| s.piece_types_mask == 0)
        {
            ret.piece_types_mask = self.toggle_selections.piece_types_mask;
        }
        ret
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Selection {
    faces_mask: u32,
    layers_mask: u32,
    piece_types_mask: u32,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            faces_mask: 0,
            layers_mask: 0,
            piece_types_mask: u32::MAX,
        }
    }
}
impl std::ops::BitOr for Selection {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            faces_mask: self.faces_mask | rhs.faces_mask,
            layers_mask: self.layers_mask | rhs.layers_mask,
            piece_types_mask: self.piece_types_mask | rhs.piece_types_mask,
        }
    }
}
impl std::ops::BitXorAssign for Selection {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.faces_mask ^= rhs.faces_mask;
        self.layers_mask ^= rhs.layers_mask;
        self.piece_types_mask ^= rhs.piece_types_mask;
    }
}
impl From<SelectThing> for Selection {
    fn from(thing: SelectThing) -> Self {
        let mut ret = Self {
            faces_mask: 0,
            layers_mask: 0,
            piece_types_mask: 0,
        };
        match thing {
            SelectThing::Face(face) => ret.faces_mask = 1 << face.id(),
            SelectThing::Layers(layers) => ret.layers_mask = layers.0,
            SelectThing::PieceType(piece_type) => ret.piece_types_mask = 1 << piece_type.id(),
        }
        ret
    }
}
impl Selection {
    fn exactly_one_face(&self, puzzle_type: PuzzleType) -> Option<Face> {
        if self.faces_mask.count_ones() == 1 {
            let face_id = self.faces_mask.trailing_zeros() as usize; // index of first `1` bit
            puzzle_type.faces().get(face_id).copied()
        } else {
            None
        }
    }
    fn layers_mask_or_default(self, default: LayerMask) -> LayerMask {
        if self.layers_mask != 0 {
            LayerMask(self.layers_mask)
        } else {
            default
        }
    }
}

// // TODO: document this
// #[derive(Debug, Default)]
// struct KeysPressed {
//     /// The set of scancodes for keys that are held.
//     scancodes: HashSet<u32>,
//     /// The set of virtual keycodes for keys that are held.
//     virtual_keycodes: HashSet<VirtualKeyCode>,
// }
// impl KeysPressed {
//     /// Updates internal key state based on a KeyboardInput event.
//     pub fn update(&mut self, input: KeyboardInput) {
//         match input.state {
//             ElementState::Pressed => {
//                 self.scancodes.insert(input.scancode);
//                 if let Some(virtual_keycode) = input.virtual_keycode {
//                     self.virtual_keycodes.insert(virtual_keycode);
//                 }
//             }
//             ElementState::Released => {
//                 self.scancodes.remove(&input.scancode);
//                 if let Some(virtual_keycode) = input.virtual_keycode {
//                     self.virtual_keycodes.remove(&virtual_keycode);
//                 }
//             }
//         }
//     }
// }
// impl Index<u32> for KeysPressed {
//     type Output = bool;
//     fn index(&self, scancode: u32) -> &bool {
//         if self.scancodes.contains(&scancode) {
//             &true
//         } else {
//             &false
//         }
//     }
// }
// impl Index<VirtualKeyCode> for KeysPressed {
//     type Output = bool;
//     fn index(&self, virtual_keycode: VirtualKeyCode) -> &bool {
//         if self.virtual_keycodes.contains(&virtual_keycode) {
//             &true
//         } else {
//             &false
//         }
//     }
// }

// fn handle_key_rubiks3d(
//     cube: &mut PuzzleController<Rubiks3D>,
//     keycode: VirtualKeyCode,
//     state: &mut State,
// ) {
//     use rubiks3d::*;
//     use VirtualKeyCode as Vk;

//     if state.modifiers.shift() {
//         match keycode {
//             _ => (),
//         }
//     } else {
//         match keycode {
//             Vk::U => cube.twist(twists::R),
//             Vk::E => cube.twist(twists::R.rev()),
//             Vk::L => cube.twist(twists::R.fat()),
//             Vk::M => cube.twist(twists::R.fat().rev()),
//             Vk::N => cube.twist(twists::U),
//             Vk::T => cube.twist(twists::U.rev()),
//             Vk::S => cube.twist(twists::L),
//             Vk::F => cube.twist(twists::L.rev()),
//             Vk::V => cube.twist(twists::L.fat()),
//             Vk::P => cube.twist(twists::L.fat().rev()),
//             Vk::R => cube.twist(twists::D),
//             Vk::I => cube.twist(twists::D.rev()),
//             Vk::H => cube.twist(twists::F),
//             Vk::D => cube.twist(twists::F.rev()),
//             Vk::W => cube.twist(twists::B),
//             Vk::Y => cube.twist(twists::B.rev()),
//             Vk::G | Vk::J => cube.twist(twists::X),
//             Vk::B | Vk::K => cube.twist(twists::X.rev()),
//             Vk::O => cube.twist(twists::Y),
//             Vk::A => cube.twist(twists::Y.rev()),
//             Vk::Semicolon => cube.twist(twists::Z),
//             Vk::Q => cube.twist(twists::Z.rev()),
//             _ => (),
//         }
//     }
// }

// fn handle_key_rubiks4d(
//     cube: &mut PuzzleController<Rubiks4D>,
//     keycode: VirtualKeyCode,
//     state: &mut State,
// ) {
//     use crate::puzzle::TwistDirection::*;
//     use rubiks4d::*;
//     use VirtualKeyCode as Vk;

//     const FACE_KEYS: [(Face, Vk, &str); 8] = [
//         (Face::L, Vk::W, "W"),
//         (Face::U, Vk::F, "F"),
//         (Face::B, Vk::P, "P"),
//         (Face::F, Vk::R, "R"),
//         (Face::I, Vk::S, "S"),
//         (Face::R, Vk::T, "T"),
//         (Face::D, Vk::C, "C"),
//         (Face::O, Vk::V, "V"),
//     ];

//     if let Ok((face, _, _)) = FACE_KEYS
//         .into_iter()
//         .filter(|(_, vk, _)| state.keys[*vk])
//         .exactly_one()
//     {
//         let layer0 = !state.modifiers.alt();
//         let layer1 = state.modifiers.alt() || state.modifiers.shift();
//         let layers = [layer0, layer1, false];
//         let twist = match keycode {
//             Vk::U => twists::by_3d_view(face, Axis::X, CW).layers(layers),
//             Vk::E => twists::by_3d_view(face, Axis::X, CCW).layers(layers),
//             Vk::N => twists::by_3d_view(face, Axis::Y, CW).layers(layers),
//             Vk::I => twists::by_3d_view(face, Axis::Y, CCW).layers(layers),
//             Vk::Y => twists::by_3d_view(face, Axis::Z, CW).layers(layers),
//             Vk::L => twists::by_3d_view(face, Axis::Z, CCW).layers(layers),
//             Vk::Space => match twists::recenter(face) {
//                 Some(twist) => twist,
//                 None => return,
//             },
//             _ => return,
//         };
//         cube.twist(twist);
//     } else if state.modifiers.shift() {
//         match keycode {
//             Vk::Key1 => state.perma_layer_hide_mask[0] = !state.perma_layer_hide_mask[0],
//             Vk::Key2 => state.perma_layer_hide_mask[1] = !state.perma_layer_hide_mask[1],
//             Vk::Key3 => state.perma_layer_hide_mask[2] = !state.perma_layer_hide_mask[2],
//             Vk::Key4 => state.perma_layer_hide_mask[3] = !state.perma_layer_hide_mask[3],
//             _ => (),
//         }
//     } else {
//         match keycode {
//             Vk::G | Vk::J => cube.twist(*twists::X),
//             Vk::B | Vk::K => cube.twist(twists::X.rev()),
//             Vk::O => cube.twist(*twists::Y),
//             Vk::A => cube.twist(twists::Y.rev()),
//             Vk::Semicolon => cube.twist(*twists::Z),
//             Vk::Q => cube.twist(twists::Z.rev()),
//             _ => (),
//         }
//     }
// }

fn update_puzzle_display<P: PuzzleState>(cube: &mut PuzzleController<P>, selection: Selection) {
    let selected_piece_types_mask = selection.piece_types_mask;

    let selected_faces = std::iter::successors(Some(selection.faces_mask), |mask| Some(mask >> 1))
        .take_while(|&mask| mask != 0)
        .positions(|mask| mask & 1 != 0)
        .filter_map(P::Face::from_id)
        .collect_vec();

    let selected_layers_mask = selection.layers_mask_or_default(LayerMask::default());

    cube.highlight_filter = Box::new(move |sticker| {
        let piece = sticker.piece();

        // Filter by piece type.
        if selected_piece_types_mask & (1 << piece.piece_type().id()) == 0 {
            return false;
        }

        // Filter by face and layer.
        for &face in &selected_faces {
            if let Some(layer) = piece.layer(face) {
                if selected_layers_mask.0 & (1 << layer) != 0 {
                    continue;
                }
            }
            return false;
        }

        true
    });

    // cube.labels = vec![];
    // if state.keys[Vk::Tab] {
    //     for &face in Face::ALL {
    //         cube.labels
    //             .push((Facet::Face(face), face.symbol().to_string()));
    //     }
    // }
}

// fn update_display_rubiks4d(cube: &mut PuzzleController<Rubiks4D>, selection: Selection) {
//     use rubiks4d::*;

//     cube.highlight_filter = Box::new(move |sticker| {
//         let piece = sticker.piece();

//         // Filter by piece type.
//         if selection.piece_types_mask & (1 << piece.piece_type_id()) == 0 {
//             return false;
//         }

//         let selected_face_indices =
//             std::iter::successors(Some(selection.faces_mask), |mask| Some(mask >> 1))
//                 .take_while(|&mask| mask != 0)
//                 .positions(|&mask| mask & 1 != 0);

//         let selected_layers = if selection.layers_mask == 0 {
//             1
//         } else {
//             selection.layers_mask
//         };

//         // Filter by face and layer.
//         for face_id in selected_face_indices {
//             let layer = piece.layer_from_face(rubiks4d::Face::from_id(FaceId(id as u32)));
//             if selection.layers_mask & (1 << layer) == 0 {
//                 return false;
//             }
//         }

//         true
//     });

//     // const FACE_KEYS: [(Face, Vk, &str); 8] = [
//     //     (Face::L, Vk::W, "W"),
//     //     (Face::U, Vk::F, "F"),
//     //     (Face::B, Vk::P, "P"),
//     //     (Face::F, Vk::R, "R"),
//     //     (Face::I, Vk::S, "S"),
//     //     (Face::R, Vk::T, "T"),
//     //     (Face::D, Vk::C, "C"),
//     //     (Face::O, Vk::V, "V"),
//     // ];

//     // let has_keyboard = state.has_keyboard;

//     // let highlight_faces = FACE_KEYS
//     //     .into_iter()
//     //     .filter(|(_, vk, _)| state.keys[*vk])
//     //     .map(|(f, _, _)| f)
//     //     .collect_vec();
//     // let layer0 = !state.modifiers.alt();
//     // let layer1 = state.modifiers.alt() || state.modifiers.shift();

//     // cube.labels = vec![];
//     // // if let Some(face) = highlight_face {
//     // //     cube.labels.push((
//     // //         Facet::Face(face),
//     // //         format!("{}{}", face.symbol(), if wide { "w" } else { "" }),
//     // //     ));
//     // // }
//     // if state.keys[Vk::Tab] {
//     //     for (face, _, text) in FACE_KEYS {
//     //         if face != Face::O {
//     //             cube.labels.push((Facet::Face(face), text.to_owned()));
//     //         }
//     //     }
//     // }

//     // let show_1c = state.keys[Vk::Key1];
//     // let show_2c = state.keys[Vk::Key2];
//     // let show_3c = state.keys[Vk::Key3];
//     // let show_4c = state.keys[Vk::Key4];
//     // let temp_highlight = (show_1c || show_2c || show_3c || show_4c) && !state.modifiers.shift();
//     // let highlight_piece_types = [
//     //     if temp_highlight {
//     //         show_1c
//     //     } else {
//     //         !state.perma_layer_hide_mask[0]
//     //     },
//     //     if temp_highlight {
//     //         show_2c
//     //     } else {
//     //         !state.perma_layer_hide_mask[1]
//     //     },
//     //     if temp_highlight {
//     //         show_3c
//     //     } else {
//     //         !state.perma_layer_hide_mask[2]
//     //     },
//     //     if temp_highlight {
//     //         show_4c
//     //     } else {
//     //         !state.perma_layer_hide_mask[3]
//     //     },
//     // ];

//     // cube.highlight_filter = Box::new(move |sticker| {
//     //     if !has_keyboard {
//     //         return true;
//     //     }

//     //     for face in &highlight_faces {
//     //         match sticker.piece()[face.axis()] * face.sign() {
//     //             Sign::Neg => return false,
//     //             Sign::Zero if !layer1 => return false,
//     //             Sign::Pos if !layer0 => return false,
//     //             _ => (),
//     //         }
//     //     }

//     //     highlight_piece_types[sticker.piece().sticker_count() - 1]
//     // });
// }

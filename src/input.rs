use glium::glutin::event::*;
use itertools::Itertools;
use std::collections::HashSet;
use std::ops::Index;

use crate::puzzle::{
    rubiks3d, rubiks4d, traits::*, PuzzleController, PuzzleEnum, Rubiks3D, Rubiks4D, Sign,
};

const SHIFT: ModifiersState = ModifiersState::SHIFT;
const CTRL: ModifiersState = ModifiersState::CTRL;
const ALT: ModifiersState = ModifiersState::ALT;
const LOGO: ModifiersState = ModifiersState::LOGO;

#[must_use = "call finish()"]
pub struct FrameInProgress<'a> {
    state: &'a mut State,
    puzzle: &'a mut PuzzleEnum,
}
impl FrameInProgress<'_> {
    pub fn handle_event(&mut self, ev: &Event<'_, ()>) {
        match ev {
            // Handle WindowEvents.
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.state.keys.update(*input);
                        if self.state.has_keyboard {
                            self.handle_key(*input);
                        }
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        self.state.modifiers = *new_modifiers;
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
        // We don't care about left vs. right modifiers, so just extract
        // the bits that don't specify left vs. right.
        let modifiers = self.state.modifiers & (SHIFT | CTRL | ALT | LOGO);

        if (modifiers & (CTRL | LOGO)).is_empty() {
            if let KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(keycode),
                ..
            } = input
            {
                match self.puzzle {
                    PuzzleEnum::Rubiks3D(cube) => handle_key_rubiks3d(cube, keycode, self.state),
                    PuzzleEnum::Rubiks4D(cube) => handle_key_rubiks4d(cube, keycode, self.state),
                }
            }
        } else if input.state == ElementState::Pressed {
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
                        PuzzleEnum::Rubiks3D(_) => eprintln!("error: can't save 3D puzzle"),
                        PuzzleEnum::Rubiks4D(cube) => {
                            if let Err(e) = cube.save_file(&crate::get_config().log_file) {
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
    }

    pub fn finish(self) {
        let mut config = crate::get_config();

        let speed = 1.0_f32.to_radians();

        if self.state.keys[VirtualKeyCode::Up] {
            config.view.theta += speed;
        }
        if self.state.keys[VirtualKeyCode::Down] {
            config.view.theta -= speed;
        }
        if self.state.keys[VirtualKeyCode::Right] {
            config.view.phi += speed;
        }
        if self.state.keys[VirtualKeyCode::Left] {
            config.view.phi -= speed;
        }

        match self.puzzle {
            PuzzleEnum::Rubiks3D(cube) => update_display_rubiks3d(cube, self.state),
            PuzzleEnum::Rubiks4D(cube) => update_display_rubiks4d(cube, self.state),
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    /// Set of pressed keys.
    keys: KeysPressed,
    /// Set of pressed modifiers.
    modifiers: ModifiersState,
    /// Whether to handle keyboard input (false if it is captured by imgui).
    has_keyboard: bool,

    perma_layer_hide_mask: [bool; 4],
}
impl State {
    pub fn frame<'a>(
        &'a mut self,
        puzzle: &'a mut PuzzleEnum,
        imgui_io: &imgui::Io,
    ) -> FrameInProgress<'a> {
        self.has_keyboard = !imgui_io.want_capture_keyboard;
        FrameInProgress {
            state: self,
            puzzle,
        }
    }
}

// TODO: document this
#[derive(Debug, Default)]
struct KeysPressed {
    /// The set of scancodes for keys that are held.
    scancodes: HashSet<u32>,
    /// The set of virtual keycodes for keys that are held.
    virtual_keycodes: HashSet<VirtualKeyCode>,
}
impl KeysPressed {
    /// Updates internal key state based on a KeyboardInput event.
    pub fn update(&mut self, input: KeyboardInput) {
        match input.state {
            ElementState::Pressed => {
                self.scancodes.insert(input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.insert(virtual_keycode);
                }
            }
            ElementState::Released => {
                self.scancodes.remove(&input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.remove(&virtual_keycode);
                }
            }
        }
    }
}
impl Index<u32> for KeysPressed {
    type Output = bool;
    fn index(&self, scancode: u32) -> &bool {
        if self.scancodes.contains(&scancode) {
            &true
        } else {
            &false
        }
    }
}
impl Index<VirtualKeyCode> for KeysPressed {
    type Output = bool;
    fn index(&self, virtual_keycode: VirtualKeyCode) -> &bool {
        if self.virtual_keycodes.contains(&virtual_keycode) {
            &true
        } else {
            &false
        }
    }
}

fn handle_key_rubiks3d(
    cube: &mut PuzzleController<Rubiks3D>,
    keycode: VirtualKeyCode,
    state: &mut State,
) {
    use rubiks3d::*;
    use VirtualKeyCode as Vk;

    if state.modifiers.shift() {
        match keycode {
            _ => (),
        }
    } else {
        match keycode {
            Vk::U => cube.twist(twists::R),
            Vk::E => cube.twist(twists::R.rev()),
            Vk::L => cube.twist(twists::R.fat()),
            Vk::M => cube.twist(twists::R.fat().rev()),
            Vk::N => cube.twist(twists::U),
            Vk::T => cube.twist(twists::U.rev()),
            Vk::S => cube.twist(twists::L),
            Vk::F => cube.twist(twists::L.rev()),
            Vk::V => cube.twist(twists::L.fat()),
            Vk::P => cube.twist(twists::L.fat().rev()),
            Vk::R => cube.twist(twists::D),
            Vk::I => cube.twist(twists::D.rev()),
            Vk::H => cube.twist(twists::F),
            Vk::D => cube.twist(twists::F.rev()),
            Vk::W => cube.twist(twists::B),
            Vk::Y => cube.twist(twists::B.rev()),
            Vk::G | Vk::J => cube.twist(twists::X),
            Vk::B | Vk::K => cube.twist(twists::X.rev()),
            Vk::O => cube.twist(twists::Y),
            Vk::A => cube.twist(twists::Y.rev()),
            Vk::Semicolon => cube.twist(twists::Z),
            Vk::Q => cube.twist(twists::Z.rev()),
            _ => (),
        }
    }
}

fn handle_key_rubiks4d(
    cube: &mut PuzzleController<Rubiks4D>,
    keycode: VirtualKeyCode,
    state: &mut State,
) {
    use crate::puzzle::TwistDirection::*;
    use rubiks4d::*;
    use VirtualKeyCode as Vk;

    const FACE_KEYS: [(Face, Vk, &str); 8] = [
        (Face::L, Vk::W, "W"),
        (Face::U, Vk::F, "F"),
        (Face::B, Vk::P, "P"),
        (Face::F, Vk::R, "R"),
        (Face::I, Vk::S, "S"),
        (Face::R, Vk::T, "T"),
        (Face::D, Vk::C, "C"),
        (Face::O, Vk::V, "V"),
    ];

    if let Ok((face, _, _)) = FACE_KEYS
        .into_iter()
        .filter(|(_, vk, _)| state.keys[*vk])
        .exactly_one()
    {
        let layer0 = !state.modifiers.alt();
        let layer1 = state.modifiers.alt() || state.modifiers.shift();
        let layers = [layer0, layer1, false];
        let twist = match keycode {
            Vk::U => twists::by_3d_view(face, Axis::X, CW).layers(layers),
            Vk::E => twists::by_3d_view(face, Axis::X, CCW).layers(layers),
            Vk::N => twists::by_3d_view(face, Axis::Y, CW).layers(layers),
            Vk::I => twists::by_3d_view(face, Axis::Y, CCW).layers(layers),
            Vk::Y => twists::by_3d_view(face, Axis::Z, CW).layers(layers),
            Vk::L => twists::by_3d_view(face, Axis::Z, CCW).layers(layers),
            Vk::Space => match twists::recenter(face) {
                Some(twist) => twist,
                None => return,
            },
            _ => return,
        };
        cube.twist(twist);
    } else if state.modifiers.shift() {
        match keycode {
            Vk::Key1 => state.perma_layer_hide_mask[0] = !state.perma_layer_hide_mask[0],
            Vk::Key2 => state.perma_layer_hide_mask[1] = !state.perma_layer_hide_mask[1],
            Vk::Key3 => state.perma_layer_hide_mask[2] = !state.perma_layer_hide_mask[2],
            Vk::Key4 => state.perma_layer_hide_mask[3] = !state.perma_layer_hide_mask[3],
            _ => (),
        }
    } else {
        match keycode {
            Vk::G | Vk::J => cube.twist(*twists::X),
            Vk::B | Vk::K => cube.twist(twists::X.rev()),
            Vk::O => cube.twist(*twists::Y),
            Vk::A => cube.twist(twists::Y.rev()),
            Vk::Semicolon => cube.twist(*twists::Z),
            Vk::Q => cube.twist(twists::Z.rev()),
            _ => (),
        }
    }
}

fn update_display_rubiks3d(cube: &mut PuzzleController<Rubiks3D>, state: &mut State) {
    use rubiks3d::*;
    use VirtualKeyCode as Vk;

    if !state.has_keyboard {
        return;
    }

    cube.labels = vec![];
    if state.keys[Vk::Tab] {
        for face in Face::iter() {
            cube.labels
                .push((Facet::Face(face), face.symbol().to_string()));
        }
    }
}

fn update_display_rubiks4d(cube: &mut PuzzleController<Rubiks4D>, state: &mut State) {
    use rubiks4d::*;
    use VirtualKeyCode as Vk;

    const FACE_KEYS: [(Face, Vk, &str); 8] = [
        (Face::L, Vk::W, "W"),
        (Face::U, Vk::F, "F"),
        (Face::B, Vk::P, "P"),
        (Face::F, Vk::R, "R"),
        (Face::I, Vk::S, "S"),
        (Face::R, Vk::T, "T"),
        (Face::D, Vk::C, "C"),
        (Face::O, Vk::V, "V"),
    ];

    let has_keyboard = state.has_keyboard;

    let highlight_faces = FACE_KEYS
        .into_iter()
        .filter(|(_, vk, _)| state.keys[*vk])
        .map(|(f, _, _)| f)
        .collect_vec();
    let layer0 = !state.modifiers.alt();
    let layer1 = state.modifiers.alt() || state.modifiers.shift();

    cube.labels = vec![];
    // if let Some(face) = highlight_face {
    //     cube.labels.push((
    //         Facet::Face(face),
    //         format!("{}{}", face.symbol(), if wide { "w" } else { "" }),
    //     ));
    // }
    if state.keys[Vk::Tab] {
        for (face, _, text) in FACE_KEYS {
            if face != Face::O {
                cube.labels.push((Facet::Face(face), text.to_owned()));
            }
        }
    }

    let show_1c = state.keys[Vk::Key1];
    let show_2c = state.keys[Vk::Key2];
    let show_3c = state.keys[Vk::Key3];
    let show_4c = state.keys[Vk::Key4];
    let temp_highlight = (show_1c || show_2c || show_3c || show_4c) && !state.modifiers.shift();
    let highlight_piece_types = [
        if temp_highlight {
            show_1c
        } else {
            !state.perma_layer_hide_mask[0]
        },
        if temp_highlight {
            show_2c
        } else {
            !state.perma_layer_hide_mask[1]
        },
        if temp_highlight {
            show_3c
        } else {
            !state.perma_layer_hide_mask[2]
        },
        if temp_highlight {
            show_4c
        } else {
            !state.perma_layer_hide_mask[3]
        },
    ];

    cube.highlight_filter = Box::new(move |sticker| {
        if !has_keyboard {
            return true;
        }

        for face in &highlight_faces {
            match sticker.piece()[face.axis()] * face.sign() {
                Sign::Neg => return false,
                Sign::Zero if !layer1 => return false,
                Sign::Pos if !layer0 => return false,
                _ => (),
            }
        }

        highlight_piece_types[sticker.piece().sticker_count() - 1]
    });
}

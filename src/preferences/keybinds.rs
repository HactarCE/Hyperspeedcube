use key_names::KeyMappingCode;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use winit::event::{ModifiersState, VirtualKeyCode};

use super::{is_false, DeserializePerPuzzle};
use crate::commands::{Command, PuzzleCommand, PuzzleCommandSerde};
use crate::puzzle::PuzzleType;

pub type PuzzleKeybind = Keybind<PuzzleCommand>;
pub type GeneralKeybind = Keybind<Command>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Keybind<C> {
    #[serde(flatten, deserialize_with = "deser_valid_key_combo")]
    pub key: KeyCombo,
    pub command: C,
}
fn deser_valid_key_combo<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KeyCombo, D::Error> {
    KeyCombo::deserialize(deserializer).map(KeyCombo::validate)
}

impl<'de> DeserializePerPuzzle<'de> for Vec<Keybind<PuzzleCommand>> {
    type Proxy = Vec<Keybind<PuzzleCommandSerde<'de>>>;

    fn deserialize_from(value: Vec<Keybind<PuzzleCommandSerde<'de>>>, ty: PuzzleType) -> Self {
        value
            .into_iter()
            .map(|keybind| Keybind {
                key: keybind.key,
                command: PuzzleCommand::deserialize_from(keybind.command, ty),
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct KeyCombo {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    key: Option<Key>,

    #[serde(skip_serializing_if = "is_false")]
    ctrl: bool,
    #[serde(skip_serializing_if = "is_false")]
    shift: bool,
    #[serde(skip_serializing_if = "is_false")]
    alt: bool,
    #[serde(skip_serializing_if = "is_false")]
    logo: bool,
}
impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mods = key_names::mods_prefix_string(self.shift, self.ctrl, self.alt, self.logo);
        write!(f, "{}", mods)?;

        match self.key {
            Some(Key::Sc(sc)) => write!(f, "{}", key_names::key_name(sc)),
            // TODO: virtual key code names aren't platform-aware and might not
            // match scancode names
            Some(Key::Vk(vk)) => match vk {
                VirtualKeyCode::Key1 => write!(f, "1"),
                VirtualKeyCode::Key2 => write!(f, "2"),
                VirtualKeyCode::Key3 => write!(f, "3"),
                VirtualKeyCode::Key4 => write!(f, "4"),
                VirtualKeyCode::Key5 => write!(f, "5"),
                VirtualKeyCode::Key6 => write!(f, "6"),
                VirtualKeyCode::Key7 => write!(f, "7"),
                VirtualKeyCode::Key8 => write!(f, "8"),
                VirtualKeyCode::Key9 => write!(f, "9"),
                VirtualKeyCode::Key0 => write!(f, "0"),
                VirtualKeyCode::Scroll => write!(f, "ScrollLock"),
                VirtualKeyCode::Back => write!(f, "Backspace"),
                VirtualKeyCode::Return => write!(f, "Enter"),
                VirtualKeyCode::Capital => write!(f, "CapsLock"),
                other => write!(f, "{:?}", other),
            },
            None => write!(f, "(no key set)"),
        }
    }
}
impl KeyCombo {
    pub fn new(key: Option<Key>, mods: ModifiersState) -> Self {
        Self {
            key,
            ctrl: mods.ctrl(),
            shift: mods.shift(),
            alt: mods.alt(),
            logo: mods.logo(),
        }
        .validate()
    }
    #[must_use]
    pub fn validate(self) -> Self {
        Self {
            key: self.key(),

            // If `key` is equivalent to a modifier key, exclude it from the
            // modifier booleans.
            ctrl: self.ctrl() && self.key().map_or(true, |k| !k.is_ctrl()),
            shift: self.shift() && self.key().map_or(true, |k| !k.is_shift()),
            alt: self.alt() && self.key().map_or(true, |k| !k.is_alt()),
            logo: self.logo() && self.key().map_or(true, |k| !k.is_logo()),
        }
    }
    pub fn key(self) -> Option<Key> {
        self.key
    }
    pub fn ctrl(self) -> bool {
        self.ctrl
    }
    pub fn shift(self) -> bool {
        self.shift
    }
    pub fn alt(self) -> bool {
        self.alt
    }
    pub fn logo(self) -> bool {
        self.logo
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    /// OS-independent "key mapping code" which corresponds to OS-dependent
    /// scan code (i.e., physical location of key on keyboard).
    #[serde(with = "crate::serde_impl::KeyMappingCodeSerde")]
    Sc(KeyMappingCode),
    /// OS-independent "virtual key code" (i.e., semantic meaning of key on
    /// keyboard, taking into account the current layout).
    Vk(VirtualKeyCode),
}
impl Key {
    pub fn is_shift(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ShiftLeft | Sc::ShiftRight) => true,
            Self::Vk(Vk::LShift | Vk::RShift) => true,
            _ => false,
        }
    }
    pub fn is_ctrl(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ControlLeft | Sc::ControlRight) => true,
            Self::Vk(Vk::LControl | Vk::RControl) => true,
            _ => false,
        }
    }
    pub fn is_alt(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::AltLeft | Sc::AltRight) => true,
            Self::Vk(Vk::LAlt | Vk::RAlt) => true,
            _ => false,
        }
    }
    pub fn is_logo(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::MetaLeft | Sc::MetaRight) => true,
            Self::Vk(Vk::LWin | Vk::RWin) => true,
            _ => false,
        }
    }
}

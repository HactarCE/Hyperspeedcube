use key_names::KeyMappingCode;
use serde::{Deserialize, Serialize};
use std::fmt;
use winit::event::{ModifiersState, VirtualKeyCode};

use super::{is_false, DeserializePerPuzzle};
use crate::puzzle::{Command, CommandSerde, PuzzleType};

#[derive(Debug, Default, Clone)]
pub struct Keybind {
    pub key: Option<Key>,

    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,

    pub command: Command,
}
impl Serialize for Keybind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        KeybindSerde::from(self).serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct KeybindSerde<'a> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub key: Option<Key>,

    #[serde(skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub logo: bool,

    #[serde(borrow)]
    pub command: CommandSerde<'a>,
}
impl<'a> From<&'a Keybind> for KeybindSerde<'_> {
    fn from(keybind: &'a Keybind) -> Self {
        KeybindSerde {
            key: keybind.key,

            ctrl: keybind.ctrl,
            shift: keybind.shift,
            alt: keybind.alt,
            logo: keybind.logo,

            command: (&keybind.command).into(),
        }
    }
}
impl<'de> DeserializePerPuzzle<'de> for Keybind {
    type Proxy = KeybindSerde<'de>;

    fn deserialize_from(keybind: KeybindSerde<'de>, ty: PuzzleType) -> Self {
        Self {
            key: keybind.key,

            ctrl: keybind.ctrl,
            shift: keybind.shift,
            alt: keybind.alt,
            logo: keybind.logo,

            command: Command::deserialize_from(keybind.command, ty),
        }
    }
}
impl fmt::Display for Keybind {
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
impl Keybind {
    pub fn new(key: Option<Key>, mods: ModifiersState, command: Command) -> Self {
        let mut ret = Self {
            key,

            ctrl: mods.ctrl(),
            shift: mods.shift(),
            alt: mods.alt(),
            logo: mods.logo(),

            command,
        };
        ret.validate_keybind();
        ret
    }
    pub fn validate_keybind(&mut self) {
        if let Some(key) = self.key {
            use KeyMappingCode as Sc;
            use VirtualKeyCode as Vk;

            // Remove redundant modifiers.
            match key {
                Key::Sc(Sc::ControlLeft | Sc::ControlRight) => self.ctrl = false,
                Key::Sc(Sc::ShiftLeft | Sc::ShiftRight) => self.shift = false,
                Key::Sc(Sc::AltLeft | Sc::AltRight) => self.alt = false,
                Key::Sc(Sc::MetaLeft | Sc::MetaRight) => self.logo = false,

                Key::Vk(Vk::LControl | Vk::RControl) => self.ctrl = false,
                Key::Vk(Vk::LShift | Vk::RShift) => self.shift = false,
                Key::Vk(Vk::LAlt | Vk::RAlt) => self.alt = false,
                Key::Vk(Vk::LWin | Vk::RWin) => self.logo = false,

                _ => (),
            }
        }
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

impl<'de> DeserializePerPuzzle<'de> for Vec<Keybind> {
    type Proxy = Vec<KeybindSerde<'de>>;

    fn deserialize_from(value: Vec<KeybindSerde<'de>>, ty: PuzzleType) -> Self {
        value
            .into_iter()
            .filter(|keybind| keybind.key.is_some())
            .map(|keybind| Keybind::deserialize_from(keybind, ty))
            .collect()
    }
}

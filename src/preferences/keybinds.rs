use key_names::KeyMappingCode;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use winit::event::{ModifiersState, VirtualKeyCode};

use super::is_false;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct KeybindSet<C: Default> {
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub includes: BTreeSet<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keybinds: Vec<Keybind<C>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Keybind<C> {
    #[serde(flatten, deserialize_with = "deser_valid_key_combo")]
    pub key: KeyCombo,
    pub command: C,
}
fn deser_valid_key_combo<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KeyCombo, D::Error> {
    KeyCombo::deserialize(deserializer).map(KeyCombo::validate)
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq)]
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

    pub fn mods(self) -> ModifiersState {
        let mut ret = ModifiersState::empty();
        if self.shift() {
            ret |= ModifiersState::SHIFT;
        }
        if self.ctrl() {
            ret |= ModifiersState::CTRL;
        }
        if self.alt() {
            ret |= ModifiersState::ALT;
        }
        if self.logo() {
            ret |= ModifiersState::LOGO;
        }
        ret
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
            Self::Sc(Sc::ShiftLeft | Sc::ShiftRight) |
            Self::Vk(Vk::LShift | Vk::RShift) => true,
            _ => false,
        }
    }
    pub fn is_ctrl(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ControlLeft | Sc::ControlRight) |
            Self::Vk(Vk::LControl | Vk::RControl) => true,
            _ => false,
        }
    }
    pub fn is_alt(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::AltLeft | Sc::AltRight) |
            Self::Vk(Vk::LAlt | Vk::RAlt) => true,
            _ => false,
        }
    }
    pub fn is_logo(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::MetaLeft | Sc::MetaRight) |
            Self::Vk(Vk::LWin | Vk::RWin) => true,
            _ => false,
        }
    }

    pub fn modifier_bit(self) -> ModifiersState {
        match self {
            _ if self.is_shift() => ModifiersState::SHIFT,
            _ if self.is_ctrl() => ModifiersState::CTRL,
            _ if self.is_alt() => ModifiersState::ALT,
            _ if self.is_logo() => ModifiersState::LOGO,
            _ => ModifiersState::empty(),
        }
    }
}

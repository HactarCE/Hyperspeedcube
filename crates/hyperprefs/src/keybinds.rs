use std::collections::BTreeSet;
use std::fmt;

use key_names::KeyMappingCode;
use serde::{Deserialize, Deserializer, Serialize};
use winit::keyboard::{Key as VirtualKeyCode, ModifiersState, NamedKey};

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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
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
        write!(f, "{mods}")?;

        match &self.key {
            // Some(Key::Sc(sc)) => write!(f, "{}", key_names::key_name(sc)), // TODO: update key_names
            Some(Key::Sc(sc)) => write!(f, "{sc:?}"),
            // TODO: virtual key code names aren't platform-aware and might not
            //       match scancode names
            // TODO: check that these are always reasonable (ask people on other OSes)
            Some(Key::Vk(vk)) => match vk.to_text() {
                Some(s) => write!(f, "{s}"),
                None => write!(f, "{vk:?}"),
            },
            None => write!(f, "(no key set)"),
        }
    }
}
impl KeyCombo {
    pub fn new(key: Option<Key>, mods: ModifiersState) -> Self {
        Self {
            key,
            ctrl: mods.control_key(),
            shift: mods.shift_key(),
            alt: mods.alt_key(),
            logo: mods.super_key(),
        }
        .validate()
    }
    #[must_use]
    pub fn validate(self) -> Self {
        Self {
            // If `key` is equivalent to a modifier key, exclude it from the
            // modifier booleans.
            ctrl: self.ctrl() && self.key().map_or(true, |k| !k.is_ctrl()),
            shift: self.shift() && self.key().map_or(true, |k| !k.is_shift()),
            alt: self.alt() && self.key().map_or(true, |k| !k.is_alt()),
            logo: self.logo() && self.key().map_or(true, |k| !k.is_logo()),

            key: self.key,
        }
    }
    pub fn key(&self) -> Option<&Key> {
        self.key.as_ref()
    }
    pub fn ctrl(&self) -> bool {
        self.ctrl
    }
    pub fn shift(&self) -> bool {
        self.shift
    }
    pub fn alt(&self) -> bool {
        self.alt
    }
    pub fn logo(&self) -> bool {
        self.logo
    }

    pub fn mods(&self) -> ModifiersState {
        let mut ret = ModifiersState::empty();
        if self.shift() {
            ret |= ModifiersState::SHIFT;
        }
        if self.ctrl() {
            ret |= ModifiersState::CONTROL;
        }
        if self.alt() {
            ret |= ModifiersState::ALT;
        }
        if self.logo() {
            ret |= ModifiersState::SUPER;
        }
        ret
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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
    pub fn is_shift(&self) -> bool {
        use {KeyMappingCode as Sc, VirtualKeyCode as Vk};
        match self {
            Self::Sc(Sc::ShiftLeft | Sc::ShiftRight) => true,
            Self::Vk(Vk::Named(NamedKey::Shift)) => true,
            _ => false,
        }
    }
    pub fn is_ctrl(&self) -> bool {
        use {KeyMappingCode as Sc, VirtualKeyCode as Vk};
        match self {
            Self::Sc(Sc::ControlLeft | Sc::ControlRight) => true,
            Self::Vk(Vk::Named(NamedKey::Control)) => true,
            _ => false,
        }
    }
    pub fn is_alt(&self) -> bool {
        use {KeyMappingCode as Sc, VirtualKeyCode as Vk};
        match self {
            Self::Sc(Sc::AltLeft | Sc::AltRight) => true,
            Self::Vk(Vk::Named(NamedKey::Alt)) => true,
            _ => false,
        }
    }
    pub fn is_logo(&self) -> bool {
        use {KeyMappingCode as Sc, VirtualKeyCode as Vk};
        match self {
            Self::Sc(Sc::MetaLeft | Sc::MetaRight) => true,
            Self::Vk(Vk::Named(NamedKey::Super)) => true,
            _ => false,
        }
    }

    pub fn modifier_bit(&self) -> ModifiersState {
        match self {
            _ if self.is_shift() => ModifiersState::SHIFT,
            _ if self.is_ctrl() => ModifiersState::CONTROL,
            _ if self.is_alt() => ModifiersState::ALT,
            _ if self.is_logo() => ModifiersState::SUPER,
            _ => ModifiersState::empty(),
        }
    }
}

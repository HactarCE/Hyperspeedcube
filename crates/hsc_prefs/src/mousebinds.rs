use serde::{Deserialize, Serialize};
use winit::keyboard::ModifiersState;

use super::is_false;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Mousebind<C> {
    pub button: MouseButton,

    #[serde(skip_serializing_if = "is_false")]
    pub ctrl: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub shift: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub alt: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub logo: bool,

    pub command: C,
}
impl<C> Mousebind<C> {
    pub fn mods(&self) -> ModifiersState {
        let mut ret = ModifiersState::empty();
        if self.shift {
            ret |= ModifiersState::SHIFT;
        }
        if self.ctrl {
            ret |= ModifiersState::CONTROL;
        }
        if self.alt {
            ret |= ModifiersState::ALT;
        }
        if self.logo {
            ret |= ModifiersState::SUPER;
        }
        ret
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}
impl From<MouseButton> for egui::PointerButton {
    fn from(b: MouseButton) -> Self {
        match b {
            MouseButton::Left => egui::PointerButton::Primary,
            MouseButton::Right => egui::PointerButton::Secondary,
            MouseButton::Middle => egui::PointerButton::Middle,
        }
    }
}

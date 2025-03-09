use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct InteractionPreferences {
    pub confirm_discard_only_when_scrambled: bool,

    pub drag_sensitivity: f32,
    // TODO: drag sensitivity for rotations vs. twists
    pub scale_twist_drag_by_radius: bool,
    pub realign_on_release: bool,
    pub realign_on_keypress: bool,
    pub smart_realign: bool,

    pub middle_click_delete: bool,
    pub confirm_on_delete_button: bool,
    pub confirm_on_alt_click_delete: bool,
    pub confirm_on_middle_click_delete: bool,

    pub reverse_filter_rules: bool,
}

/// Input from the user showing intent to delete something.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DeleteInput {
    /// Click on a button in the GUI.
    Button,
    /// Alt+click.
    AltClick,
    /// Middle-click.
    MiddleClick,
}

impl InteractionPreferences {
    pub fn needs_confirm_delete(&self, input: DeleteInput) -> bool {
        match input {
            DeleteInput::Button => self.confirm_on_delete_button,
            DeleteInput::AltClick => self.confirm_on_alt_click_delete,
            DeleteInput::MiddleClick => self.confirm_on_middle_click_delete,
        }
    }
}

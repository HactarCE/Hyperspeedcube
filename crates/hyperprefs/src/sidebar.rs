use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SidebarPreferences {
    pub show: bool,
    pub show_labels: bool,
}

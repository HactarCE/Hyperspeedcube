use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct Layout {
    /// Dock state, serialized as a string because it uses types that are only
    /// defined in the `hyperspeedcube` crate.
    ///
    /// Also, it should be possible to make breaking changes to the dock state
    /// schema without breaking preferences.
    pub dock_state: Option<String>,

    /// Sidebar style.
    pub sidebar_style: SidebarStyle,

    /// Sidebar utility, serializes as a srting because it uses types that are
    /// only defined in the `hyperspeedcube` crate.
    pub sidebar_utility: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SidebarStyle {
    Disabled,
    IconsOnly,
    #[default]
    IconsAndText,
}

impl SidebarStyle {
    pub fn is_shown(self) -> bool {
        self != SidebarStyle::Disabled
    }

    #[must_use]
    pub fn toggle_labels(self) -> Self {
        match self {
            SidebarStyle::Disabled => SidebarStyle::Disabled,
            SidebarStyle::IconsOnly => SidebarStyle::IconsAndText,
            SidebarStyle::IconsAndText => SidebarStyle::IconsOnly,
        }
    }
}

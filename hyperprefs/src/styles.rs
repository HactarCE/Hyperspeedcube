use hyperpuzzle::Rgb;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct StylePreferences {
    pub dark_background_color: Rgb,
    pub light_background_color: Rgb,
    pub internals_color: Rgb,
    pub blocking_outline_color: Rgb, // TODO: move to its own style, maybe?
    pub blocking_outline_size: f32,  // TODO: otherwise, add this to prefs UI

    pub default: PieceStyle,
    pub gripped: PieceStyle,
    pub ungripped: PieceStyle,
    pub hovered_piece: PieceStyle,
    pub selected_piece: PieceStyle,
    pub blind: PieceStyle,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
#[serde(default)]
pub struct PieceStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_color: Option<StyleColorMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_color: Option<StyleColorMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_lighting: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Display, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "mode")]
pub enum StyleColorMode {
    #[default]
    #[serde(rename = "sticker")]
    FromSticker,
    #[serde(rename = "fixed")]
    FixedColor {
        #[serde(default)]
        color: Rgb,
    },
    #[serde(rename = "rainbow")]
    Rainbow,
}
impl StyleColorMode {
    /// Modifies the color mode as needed to ensure that all stickers have the
    /// same color. This is useful if outlines may overlap, or if blindfolded
    /// mode is enabled.
    pub fn make_same_if(&mut self, bld: bool) {
        if bld && *self == Self::FromSticker {
            *self = Self::FixedColor { color: Rgb::BLACK };
        }
    }

    /// Returns whether the color mode is safe for the given blindsolving mode.
    pub fn is_bld_safe(self, bld: bool) -> bool {
        match self {
            StyleColorMode::FromSticker => !bld,
            StyleColorMode::FixedColor { .. } => true,
            StyleColorMode::Rainbow => true,
        }
    }

    /// Returns the fixed color if there is one.
    #[must_use]
    pub fn fixed_color(self) -> Option<Rgb> {
        match self {
            StyleColorMode::FromSticker => None,
            StyleColorMode::FixedColor { color } => Some(color),
            StyleColorMode::Rainbow => None,
        }
    }

    #[must_use]
    pub fn map_fixed_color(mut self, f: impl FnOnce(Rgb) -> Rgb) -> Self {
        if let StyleColorMode::FixedColor { color } = &mut self {
            *color = f(*color);
        }
        self
    }

    /// Returns whether the style is animated.
    pub fn is_animated(self) -> bool {
        match self {
            StyleColorMode::FromSticker | StyleColorMode::FixedColor { .. } => false,
            StyleColorMode::Rainbow => true,
        }
    }
}

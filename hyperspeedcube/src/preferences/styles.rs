use std::sync::atomic::AtomicU64;

use serde::{Deserialize, Serialize};

use super::{Rgb, WithPresets};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StyleId(u64);
impl StyleId {
    pub fn next() -> Self {
        Self(NEXT_STYLE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

lazy_static! {
    static ref NEXT_STYLE_ID: AtomicU64 = AtomicU64::new(1);
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct StylePreferences {
    pub dark_background_color: Rgb,
    pub light_background_color: Rgb,
    pub internals_color: Rgb,
    pub blocking_outline_color: Rgb, // TODO: move to its own style, maybe?
    pub blocking_outline_size: f32,  // TODO: otherwise, add this to prefs UI

    pub custom: WithPresets<PieceStyle>,

    pub default: PieceStyle,
    pub gripped: PieceStyle,
    pub ungripped: PieceStyle,
    pub hovered_piece: PieceStyle,
    pub selected_piece: PieceStyle,
    pub blind: PieceStyle,
}
impl StylePreferences {
    pub(super) fn post_init(&mut self) {}

    pub fn background_color(&self, dark_mode: bool) -> Rgb {
        match dark_mode {
            true => self.dark_background_color,
            false => self.light_background_color,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
#[serde(default)]
pub struct PieceStyle {
    /// Unique ID that lasts only for the lifetime of the program.
    #[serde(skip, default = "StyleId::next")]
    pub id: StyleId,

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
impl PartialEq for PieceStyle {
    fn eq(&self, other: &Self) -> bool {
        // ignore ID
        self.interactable == other.interactable
            && self.face_opacity == other.face_opacity
            && self.face_color == other.face_color
            && self.outline_opacity == other.outline_opacity
            && self.outline_size == other.outline_size
            && self.outline_color == other.outline_color
            && self.outline_lighting == other.outline_lighting
    }
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
        }
    }

    /// Returns the fixed color if there is one.
    #[must_use]
    pub fn fixed_color(self) -> Option<Rgb> {
        match self {
            StyleColorMode::FromSticker => None,
            StyleColorMode::FixedColor { color } => Some(color),
        }
    }

    #[must_use]
    pub fn map_fixed_color(mut self, f: impl FnOnce(Rgb) -> Rgb) -> Self {
        if let StyleColorMode::FixedColor { color } = &mut self {
            *color = f(*color);
        }
        self
    }
}

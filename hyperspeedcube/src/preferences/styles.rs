use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Rgb;

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
    pub blocking_color: Rgb,        // TODO: move to its own style, maybe?
    pub blocking_outline_size: f32, // TODO: otherwise, add this to prefs UI

    pub default: PieceStyle,

    pub gripped: PieceStyle,
    pub ungripped: PieceStyle,
    pub hovered_piece: PieceStyle,
    pub hovered_sticker: PieceStyle,
    pub selected_piece: PieceStyle,
    pub selected_sticker: PieceStyle,
    pub hidden: PieceStyle,
    pub blind: PieceStyle,

    pub custom: IndexMap<Arc<String>, PieceStyle>,
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
    pub face_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_sticker_color: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_color: Option<Rgb>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_sticker_color: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_size: Option<f32>,
}

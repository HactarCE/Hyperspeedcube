use std::collections::BTreeMap;
use std::path::PathBuf;

use hyperpuzzle::{DefaultColor, Rgb};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::FilterPieceSet;
pub use crate::{
    AnimationPreferences, ImageGeneratorPreferences, InfoPreferences, InteractionPreferences,
    PieceStyle, StylePreferences, ViewPreferences,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Preferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,

    pub info: InfoPreferences,

    pub image_generator: ImageGeneratorPreferences,

    pub animation: PresetsList<AnimationPreferences>,
    pub interaction: InteractionPreferences,
    pub styles: StylePreferences,
    pub custom_styles: PresetsList<PieceStyle>,

    pub view_3d: PresetsList<ViewPreferences>,
    pub view_4d: PresetsList<ViewPreferences>,

    pub color_palette: GlobalColorPalette,
    /// Color scheme preferences for each color system.
    pub color_schemes: BTreeMap<String, ColorSystemPreferences>,

    /// Filter preferences for each puzzle.
    pub filters: BTreeMap<String, PuzzleFilterPreferences>,

    pub show_experimental_puzzles: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct PresetsList<T: Default> {
    #[serde(skip_serializing_if = "str::is_empty")]
    pub last_loaded: String,
    /// List of user presets.
    pub presets: IndexMap<String, T>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct GlobalColorPalette {
    pub custom_colors: IndexMap<String, Rgb>,
    pub builtin_colors: IndexMap<String, Rgb>,
    pub builtin_color_sets: IndexMap<String, Vec<Rgb>>,
}

pub type ColorSystemPreferences = IndexMap<String, ColorScheme>;

pub type ColorScheme = IndexMap<String, DefaultColor>;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct PuzzleFilterPreferences {
    pub presets: IndexMap<String, FilterPreset>,
    pub sequences: IndexMap<String, IndexMap<String, FilterSeqPreset>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct FilterPreset {
    pub rules: Vec<FilterRule>,
    pub fallback_style: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct FilterRule {
    pub style: Option<String>,
    pub set: FilterPieceSet,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct FilterSeqPreset {
    pub include_previous: bool,
    pub skip: bool,
    #[serde(flatten)]
    pub inner: FilterPreset,
}

use std::collections::BTreeMap;

use hyperpuzzle::{PerColor, PerPieceType, PieceMask, Puzzle};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

mod checkboxes;
mod expr;

pub use checkboxes::*;
pub use expr::*;

use super::StylePreferences;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct FilterPreferences {
    #[serde(flatten)]
    pub puzzle_presets: BTreeMap<String, PuzzleFilterPreferences>,
    #[serde(skip)]
    empty_puzzle_presets: PuzzleFilterPreferences,
}
impl FilterPreferences {
    pub(super) fn post_init(&mut self) {
        for puzzle_presets in self.puzzle_presets.values_mut() {
            puzzle_presets.post_init();
        }
    }

    pub fn settings(&self, puzzle: &Puzzle) -> Option<&PuzzleFilterPreferences> {
        self.puzzle_presets.get(&puzzle.id)
    }
    pub fn settings_mut(&mut self, puzzle: &Puzzle) -> &mut PuzzleFilterPreferences {
        self.puzzle_presets.entry(puzzle.id.clone()).or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PuzzleFilterPreferences {
    pub presets: IndexMap<String, FilterPreset>,
    pub sequences: IndexMap<String, IndexMap<String, FilterPresetSeq>>,
}
impl PuzzleFilterPreferences {
    pub(super) fn post_init(&mut self) {}

    pub fn get(
        &self,
        sequence_name: Option<&String>,
        preset_name: Option<&String>,
    ) -> Option<&FilterPreset> {
        match sequence_name {
            Some(seq) => Some(&self.sequences.get(seq)?.get(preset_name?)?.inner),
            None => self.presets.get(preset_name?),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct FilterPresetSeq {
    pub include_previous: bool,
    pub skip: bool,
    #[serde(flatten)]
    pub inner: FilterPreset,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct FilterPreset {
    pub rules: Vec<FilterRule>,
    pub fallback_style: Option<String>,
}
impl FilterPreset {
    pub fn new(styles: &StylePreferences) -> Self {
        Self {
            rules: vec![FilterRule::new_checkboxes()],
            fallback_style: styles
                .custom
                .user_list()
                .first()
                .map(|style| style.name.clone()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct FilterRule {
    pub style: Option<String>,
    pub set: FilterPieceSet,
}
impl FilterRule {
    pub fn new_checkboxes() -> Self {
        Self {
            style: None,
            set: FilterPieceSet::Checkboxes(FilterCheckboxes::default()),
        }
    }
    pub fn new_expr() -> Self {
        Self {
            style: None,
            set: FilterPieceSet::Expr("@everything".to_owned()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum FilterPieceSet {
    Expr(String),
    Checkboxes(FilterCheckboxes),
}
impl Default for FilterPieceSet {
    fn default() -> Self {
        Self::Expr("@everything".to_owned())
    }
}
impl FilterPieceSet {
    pub fn eval(&self, puz: &Puzzle) -> PieceMask {
        match self {
            Self::Expr(expr) => expr::FilterExpr::from_str(&expr).eval(puz),
            Self::Checkboxes(checkboxes) => checkboxes.eval(puz),
        }
    }

    pub fn to_string(&self, colors: &PerColor<&str>, piece_types: &PerPieceType<&str>) -> String {
        match self {
            Self::Expr(expr) => expr.to_string(),
            Self::Checkboxes(checkboxes) => checkboxes.to_string(colors, piece_types),
        }
    }
}

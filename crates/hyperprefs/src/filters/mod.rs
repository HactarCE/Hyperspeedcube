use std::fmt;
use std::sync::Arc;
use std::{borrow::Cow, collections::HashMap};

use hyperpuzzle_core::{PieceMask, Puzzle};
use itertools::Itertools;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

mod checkboxes;
mod expr;

pub use checkboxes::*;
pub use expr::*;

use super::{PieceStyle, PresetRef, PresetTombstone, PresetsList, schema};
use crate::ext::reorderable::{DragAndDropResponse, ReorderableCollection};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FilterPresetName {
    pub seq: Option<String>,
    pub preset: String,
}
impl FilterPresetName {
    pub fn new(preset: String) -> Self {
        Self { seq: None, preset }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilterPresetRef {
    pub seq: Option<PresetRef>,
    pub preset: PresetRef,
}
impl fmt::Display for FilterPresetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(seq) = &self.seq {
            write!(f, "{} ~ ", seq.name())?;
        }
        write!(f, "{}", self.preset.name())?;
        Ok(())
    }
}
impl FilterPresetRef {
    pub fn name(&self) -> FilterPresetName {
        FilterPresetName {
            seq: self.seq.as_ref().map(|r| r.name()),
            preset: self.preset.name(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PuzzleFilterPreferences {
    pub presets: PresetsList<FilterPreset>,
    pub sequences: PresetsList<PresetsList<FilterSeqPreset>>,
}
impl schema::PrefsConvert for PuzzleFilterPreferences {
    type DeserContext = PresetsList<PieceStyle>;
    type SerdeFormat = schema::current::PuzzleFilterPreferences;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self { presets, sequences } = self;

        let presets = presets.to_serde_map();
        let sequences = sequences
            .user_presets()
            .map(|p| (p.name().clone(), p.value.to_serde_map()))
            .collect();

        schema::current::PuzzleFilterPreferences { presets, sequences }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::PuzzleFilterPreferences { presets, sequences } = value;

        self.presets.reload_from_serde_map(ctx, presets);
        self.sequences.reload_from_presets_map(
            sequences
                .into_iter()
                .filter(|(_, seq)| !seq.is_empty()) // Remove empty sequences
                .map(|(name, seq)| (name, PresetsList::from_serde_map(ctx, seq))),
        );
    }
}
impl ReorderableCollection<FilterPresetName> for PuzzleFilterPreferences {
    fn reorder(&mut self, drag: DragAndDropResponse<FilterPresetName>) {
        if drag.payload == drag.end || !self.has_preset(&drag.end) {
            return;
        }

        let preset = (|| match &drag.payload.seq {
            Some(seq_name) => self
                .sequences
                .get_mut(seq_name)?
                .value
                .remove(&drag.payload.preset),
            None => self
                .presets
                .remove(&drag.payload.preset)
                .map(FilterSeqPreset::from),
        })();
        let Some(preset_value) = preset else {
            return;
        };

        match drag.end.seq {
            Some(seq_name) => {
                let seq = &mut self
                    .sequences
                    .get_mut(&seq_name)
                    .expect("filter sequence vanished!")
                    .value;
                let new_name =
                    seq.save_preset_with_nonconflicting_name(&drag.payload.preset, preset_value);
                seq.reorder(DragAndDropResponse {
                    payload: new_name,
                    end: drag.end.preset,
                    before_or_after: drag.before_or_after,
                });
            }
            None => {
                let new_name = self
                    .presets
                    .save_preset_with_nonconflicting_name(&drag.payload.preset, preset_value.inner);
                self.presets.reorder(DragAndDropResponse {
                    payload: new_name,
                    end: drag.end.preset,
                    before_or_after: drag.before_or_after,
                });
            }
        }

        // Remove empty sequences
        if let Some(seq_name) = &drag.payload.seq {
            if let Some(seq) = self.sequences.get(seq_name) {
                if seq.value.is_empty() {
                    self.sequences.remove(seq_name);
                }
            }
        }
    }
}
impl PuzzleFilterPreferences {
    /// Returns whether a filter preset exists with the given name.
    pub fn has_preset(&self, name: &FilterPresetName) -> bool {
        match &name.seq {
            Some(seq_name) => self
                .sequences
                .get(seq_name)
                .is_some_and(|seq| seq.value.contains_key(&name.preset)),
            None => self.presets.contains_key(&name.preset),
        }
    }

    /// Removes a filter preset. Empty sequences are kept.
    pub fn remove_preset(&mut self, name: &FilterPresetName) -> Option<FilterSeqPreset> {
        match &name.seq {
            Some(seq_name) => {
                let seq = self.sequences.get_mut(seq_name)?;
                let ret = seq.value.remove(&name.preset);

                // Remove empty sequences
                if seq.value.is_empty() {
                    self.sequences.remove(seq_name);
                }

                ret
            }
            None => self.presets.remove(&name.preset).map(FilterSeqPreset::from),
        }
    }
    /// Saves a filter preset, creating a filter sequence if necessary.
    pub fn save_preset(&mut self, name: &FilterPresetName, value: FilterSeqPreset) {
        match &name.seq {
            Some(seq_name) => {
                if !self.sequences.contains_key(seq_name) {
                    self.sequences.save_preset(seq_name, PresetsList::default());
                }
                let seq = self
                    .sequences
                    .get_mut(seq_name)
                    .expect("filter sequence vanished!");

                seq.value.save_preset(&name.preset, value);
            }
            None => self.presets.save_preset(&name.preset, value.inner),
        }
    }
    /// Renames a filter preset.
    pub fn rename_preset<'a>(
        &mut self,
        old_name: &FilterPresetName,
        new_name: impl Into<Cow<'a, str>>,
    ) {
        match &old_name.seq {
            Some(seq_name) => {
                if let Some(seq) = self.sequences.get_mut(seq_name) {
                    seq.value.rename(&old_name.preset, new_name);
                }
            }
            None => self.presets.rename(&old_name.preset, new_name),
        }
    }

    /// Returns a filter preset.
    pub fn get(&self, name: &FilterPresetName) -> Option<FilterSeqPreset> {
        match &name.seq {
            Some(seq_name) => {
                let seq = &self.sequences.get(seq_name)?.value;
                Some(seq.get(&name.preset)?.value.clone())
            }
            None => Some(FilterSeqPreset::from(
                self.presets.get(&name.preset)?.value.clone(),
            )),
        }
    }
    /// Returns a reference to a filter preset with the given name, even if one
    /// does not exist.
    pub fn new_ref(&self, name: &FilterPresetName) -> FilterPresetRef {
        match &name.seq {
            Some(seq_name) => match self.sequences.get(seq_name) {
                Some(seq) => FilterPresetRef {
                    seq: Some(seq.new_ref()),
                    preset: seq.value.new_ref(&name.preset),
                },
                None => {
                    let seq_ref = PresetRef {
                        name: Arc::new(Mutex::new(seq_name.to_owned())),
                    };
                    let preset_ref = PresetRef {
                        name: Arc::new(Mutex::new(name.preset.clone())),
                    };

                    self.sequences.add_tombstone(
                        seq_name.to_owned(),
                        PresetTombstone {
                            dead_refs: vec![seq_ref.clone()],
                            value_tombstone: HashMap::from_iter([(
                                preset_ref.name(),
                                PresetTombstone {
                                    dead_refs: vec![preset_ref.clone()],
                                    value_tombstone: (),
                                },
                            )]),
                        },
                    );

                    FilterPresetRef {
                        seq: Some(seq_ref),
                        preset: preset_ref,
                    }
                }
            },
            None => FilterPresetRef {
                seq: None,
                preset: self.presets.new_ref(&name.preset),
            },
        }
    }

    /// Returns the fallbacks for a filter preset, as a single combined preset.
    ///
    /// Returns `None` if the preset is not in a sequence or if there are no
    /// fallback presets.
    pub fn combined_fallback_preset(&self, name: &FilterPresetName) -> Option<FilterPreset> {
        let seq = &self.sequences.get(name.seq.as_ref()?)?.value;
        let index = seq.get_index_of(&name.preset)?;
        let mut fallback_presets = (0..=index)
            .rev()
            .map_while(|i| seq.nth_user_preset(i))
            .map(|(_name, preset)| &preset.value)
            .take_while_inclusive(|p| p.include_previous)
            .map(|p| &p.inner)
            .skip(1);
        let mut ret = fallback_presets.next()?.clone();
        for p in fallback_presets {
            ret.rules.extend_from_slice(&p.rules);
            ret.fallback_style = p.fallback_style.clone();
        }
        Some(ret)
    }
    /// Returns the previous preset in the sequence, if there is one.
    pub fn prev_preset_in_seq(&self, name: &FilterPresetName) -> Option<FilterPresetRef> {
        let seq = &self.sequences.get(name.seq.as_ref()?)?;
        let index = seq.value.get_index_of(&name.preset)?;
        let (_, preset) = seq.value.nth_user_preset(index.checked_sub(1)?)?;
        Some(FilterPresetRef {
            seq: Some(seq.new_ref()),
            preset: preset.new_ref(),
        })
    }
    /// Returns the next preset in the sequence, if there is one.
    pub fn next_preset_in_seq(&self, name: &FilterPresetName) -> Option<FilterPresetRef> {
        let seq = &self.sequences.get(name.seq.as_ref()?)?;
        let index = seq.value.get_index_of(&name.preset)?;
        let (_, preset) = seq.value.nth_user_preset(index.checked_add(1)?)?;
        Some(FilterPresetRef {
            seq: Some(seq.new_ref()),
            preset: preset.new_ref(),
        })
    }

    /// Returns whether the filter preferences contains the defaults and so does
    /// not need to be saved.
    pub(crate) fn is_default(&self) -> bool {
        let Self { presets, sequences } = self;
        presets.is_default() && sequences.is_default()
    }
}

/// Filter preset in a sequence.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FilterSeqPreset {
    pub include_previous: bool,
    pub skip: bool,
    pub inner: FilterPreset,
}
impl schema::PrefsConvert for FilterSeqPreset {
    type DeserContext = PresetsList<PieceStyle>;
    type SerdeFormat = schema::current::FilterSeqPreset;

    fn to_serde(&self) -> Self::SerdeFormat {
        schema::current::FilterSeqPreset {
            include_previous: self.include_previous,
            skip: self.skip,
            inner: self.inner.to_serde(),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::FilterSeqPreset {
            include_previous,
            skip,
            inner,
        } = value;

        self.include_previous = include_previous;
        self.skip = skip;
        self.inner = FilterPreset::from_serde(ctx, inner);
    }
}
impl From<FilterPreset> for FilterSeqPreset {
    fn from(inner: FilterPreset) -> Self {
        Self {
            include_previous: false,
            skip: false,
            inner,
        }
    }
}
impl FilterSeqPreset {
    pub fn new_empty() -> Self {
        FilterPreset::new_empty().into()
    }
    pub fn new_with_single_rule(fallback_style: Option<PresetRef>) -> Self {
        FilterPreset::new_with_single_rule(fallback_style).into()
    }
}

/// Filter preset (standalone; not in a sequence).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FilterPreset {
    /// Filter rules, in order from highest priority to lowest priority.
    pub rules: Vec<FilterRule>,
    /// Style to apply to pieces not covered by any rule.
    pub fallback_style: Option<PresetRef>,
}
impl schema::PrefsConvert for FilterPreset {
    type DeserContext = PresetsList<PieceStyle>;
    type SerdeFormat = schema::current::FilterPreset;

    fn to_serde(&self) -> Self::SerdeFormat {
        schema::current::FilterPreset {
            rules: self.rules.iter().map(|rule| rule.to_serde()).collect(),
            fallback_style: self.fallback_style.as_ref().map(|p| p.name()),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::FilterPreset {
            rules,
            fallback_style,
        } = value;

        self.rules = rules
            .into_iter()
            .map(|rule| FilterRule::from_serde(ctx, rule))
            .collect();

        self.fallback_style = fallback_style.map(|s| ctx.new_ref(&s));
    }
}
impl FilterPreset {
    pub fn new_empty() -> Self {
        Self::default()
    }
    pub fn new_with_single_rule(fallback_style: Option<PresetRef>) -> Self {
        Self {
            rules: vec![FilterRule::new_checkboxes()],
            fallback_style,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FilterRule {
    pub style: Option<PresetRef>,
    pub set: FilterPieceSet,
}
impl schema::PrefsConvert for FilterRule {
    type DeserContext = PresetsList<PieceStyle>;
    type SerdeFormat = schema::current::FilterRule;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self { style, set } = self;

        schema::current::FilterRule {
            style: style.as_ref().map(|p| p.name()),
            set: set.to_serde(),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::FilterRule { style, set } = value;

        self.style = style.map(|s| ctx.new_ref(&s));
        self.set = FilterPieceSet::from_serde(&(), set);
    }
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
            Self::Expr(expr) => expr::FilterExpr::from_str(expr).eval(puz),
            Self::Checkboxes(checkboxes) => checkboxes.eval(puz),
        }
    }

    pub fn to_string(&self, ctx: &impl FilterCheckboxesCtx) -> String {
        match self {
            Self::Expr(expr) => expr.to_string(),
            Self::Checkboxes(checkboxes) => checkboxes.to_string(ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_preset_renaming() {
        let mut p = PuzzleFilterPreferences::default();
        let ab = p.new_ref(&FilterPresetName {
            seq: Some("a".to_owned()),
            preset: "b".to_owned(),
        });

        assert!(p.sequences.is_empty());
        assert!(p.presets.is_empty());
        p.save_preset(
            &FilterPresetName::new("a".to_owned()),
            FilterSeqPreset::default(),
        );
        p.rename_preset(&FilterPresetName::new("a".to_owned()), "b");
        p.rename_preset(&FilterPresetName::new("b".to_owned()), "c");
        assert_eq!(ab.seq.clone().unwrap(), "a");
        assert_eq!(ab.preset, "b");

        p.save_preset(&ab.name(), FilterSeqPreset::default());
        p.rename_preset(
            &FilterPresetName {
                seq: Some("a".to_owned()),
                preset: "b".to_owned(),
            },
            "y",
        );
        p.sequences.rename("a", "x");
        p.rename_preset(
            &FilterPresetName {
                seq: Some("x".to_owned()),
                preset: "y".to_owned(),
            },
            "z",
        );
        assert_eq!(
            ab.name(),
            FilterPresetName {
                seq: Some("x".to_owned()),
                preset: "z".to_owned()
            }
        );
    }
}

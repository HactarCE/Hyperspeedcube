use hyperpuzzle::{PerColor, PerPieceType, PieceMask, Puzzle};
use serde::{Deserialize, Serialize};

mod checkboxes;
mod expr;

pub use checkboxes::*;
pub use expr::*;

use crate::ext::reorderable::{DragAndDropResponse, ReorderableCollection};

use super::{schema, PieceStyle, PresetRef, PresetsList};

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
                .map(|(name, seq)| (name, PresetsList::from_serde_map(ctx, seq))),
        );
    }
}
impl PuzzleFilterPreferences {
    // pub fn get(
    //     &self,
    //     sequence_name: Option<&String>,
    //     preset_name: Option<&String>,
    // ) -> Option<&FilterPreset> {
    //     match sequence_name {
    //         Some(seq) => Some(&self.sequences.get(seq)?.value.get(preset_name?)?.inner),
    //         None => self.presets.get(preset_name?),
    //     }
    // }

    // pub fn remove_index(
    //     &mut self,
    //     seq: Option<usize>,
    //     preset: usize,
    // ) -> Option<(String, FilterPresetSeq)> {
    //     match seq {
    //         Some(i) => {
    //             let (_, seq) = self.sequences.get_index_mut(i)?;
    //             seq.shift_remove_index(preset)
    //         }
    //         None => self
    //             .presets
    //             .shift_remove_index(preset)
    //             .map(|(k, v)| (k, v.into())),
    //     }
    // }
    // fn insert_index(
    //     &mut self,
    //     seq: Option<usize>,
    //     preset: usize,
    //     key: String,
    //     value: FilterPresetSeq,
    // ) {
    //     match seq {
    //         Some(i) => {
    //             if let Some((_, seq)) = self.sequences.get_index_mut(i) {
    //                 seq.shift_insert(preset, key, value);
    //             }
    //         }
    //         None => {
    //             self.presets.shift_insert(preset, key, value.inner);
    //         }
    //     }
    // }

    // pub fn rename_preset(&mut self, seq: Option<usize>, preset: usize, new_name: String) {
    //     if let Some((_, v)) = self.remove_index(seq, preset) {
    //         self.insert_index(seq, preset, new_name, v);
    //     }
    // }
}
impl ReorderableCollection<(Option<usize>, usize)> for PuzzleFilterPreferences {
    fn reorder(&mut self, drag: DragAndDropResponse<(Option<usize>, usize)>) {
        todo!()
        // let (payload_seq, payload_preset) = drag.payload;
        // let (end_seq, mut end_preset) = drag.end;
        // if drag.before_or_after == Some(BeforeOrAfter::After) {
        //     end_preset += 1;
        // }
        // if payload_seq == end_seq && end_preset > payload_preset {
        //     end_preset -= 1;
        // }
        // if let Some((k, v)) = self.remove_index(payload_seq, payload_preset) {
        //     self.insert_index(end_seq, end_preset, k, v);
        // }
    }
}

// /// List of filter sequences.
// ///
// /// Conceptually, this is similar to `PresetList<PresetList<FilterPresetSeq>>`,
// /// but it correctly preserves references even when a whole filter sequence is
// /// deleted and recreated.
// #[derive(Debug, Default)]
// pub struct FilterSequences {
//     /// Internal [`PresetsList`]; we don't use any built-in presets.s
//     inner: PresetsList<PresetsList<FilterSeqPreset>>,
//     /// List of orphans from deleted sequences.
//     orphans: Mutex<HashMap<String, OrphanedPresetData>>,
// }
// impl schema::PrefsConvert for FilterSequences {
//     type DeserContext = PresetsList<PieceStyle>;
//     type SerdeFormat = IndexMap<String, IndexMap<String, schema::current::FilterSeqPreset>>;

//     fn to_serde(&self) -> Self::SerdeFormat {
//         self.inner
//             .user
//             .iter()
//             .map(|(k, v)| (k.clone(), v.value.to_serde_map()))
//             .collect()
//     }
//     fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
//         // Remove all presets. Orphaned references are saved.
//         self.remove_all_seqs();

//         // Add presets back. Orphaned references are restored.
//         for (k, v) in value {
//             self.save_seq(k, v);
//         }
//     }
// }
// impl FilterSequences {
//     /// Returns whether there are no filter sequences.
//     pub fn is_empty(&self) -> bool {
//         self.inner.is_empty()
//     }
//     /// Returns the number of filter sequences.
//     pub fn len(&self) -> usize {
//         self.inner.len()
//     }
//     /// Returns whether there is a filter sequence with the given name.
//     pub fn contains_seq(&self, name: &str) -> bool {
//         self.get_seq(name).is_some()
//     }

//     /// Iterates over all filter sequences.
//     pub fn seqs(
//         &self,
//     ) -> impl Iterator<Item = (&String, impl Iterator<Item = &Preset<FilterSeqPreset>>)> {
//         self.inner
//             .user_presets()
//             .map(|seq| (seq.name(), seq.value.user_presets()))
//     }

//     /// Returns a reference to a sequence with the given name, even if one does
//     /// not exist.
//     pub fn new_seq_ref(&self, name: &str) -> PresetRef {
//         self.inner.new_ref(name)
//     }

//     /// Returns a reference to preset with the given name, even if the preset or
//     /// its sequence does not exist.
//     pub fn new_preset_ref(&self, seq_name: &str, preset_name: &str) -> PresetRef {
//         if let Some(seq) = self.inner.get(seq_name) {
//             seq.value.new_ref(preset_name)
//         } else if let Some(orphan_ref) = self
//             .orphans
//             .lock()
//             .get(seq_name)
//             .and_then(|seq_orphans| seq_orphans.get(preset_name))
//             .and_then(|l| l.first())
//         {
//             orphan_ref.clone()
//         } else {
//             let new_ref = PresetRef {
//                 name: Arc::new(Mutex::new(preset_name.to_owned())),
//             };
//             let new_orphans =
//                 OrphanedPresetData::from_iter([(preset_name.to_owned(), vec![new_ref.clone()])]);
//             self.add_orphans(seq_name.to_owned(), new_orphans);
//             self.prune_orphans();
//             new_ref
//         }
//     }

//     /// Returns whether a preset has been modified.
//     ///
//     /// Returns `true` if the preset or sequence does not exist.
//     pub fn is_modified(&self, seq_name: &str, p: &ModifiedPreset<FilterSeqPreset>) -> bool {
//         match self.inner.get(seq_name) {
//             Some(seq) => seq.value.is_modified(p),
//             None => true,
//         }
//     }
//     /// Returns whether the given sequence name is a valid name for a new
//     /// sequence.
//     pub fn is_seq_name_available(&self, new_name: &str) -> bool {
//         self.inner.is_name_available(new_name)
//     }
//     /// Returns whether the given preset name is a valid name for a new preset.
//     /// Always returns `true` if the sequence does not exist.
//     pub fn is_preset_name_available(&self, seq_name: &str, new_name: &str) -> bool {
//         match self.inner.get(seq_name) {
//             Some(seq) => seq.value.is_name_available(new_name),
//             None => true,
//         }
//     }

//     /// Returns the filter sequence with the given name, or `None` if it does
//     /// not exist.
//     pub fn get_seq(&self, name: &str) -> Option<&PresetsList<FilterSeqPreset>> {
//         Some(&self.inner.get(name)?.value)
//     }

//     /// Saves a filter sequence, adding a new one if it does not already exist.
//     pub fn save_seq(&mut self, name: String, presets: Vec<(String, FilterSeqPreset)>) {
//         todo!();
//         // self.inner.save_preset(name, presets)
//         "do something here"
//     }

//     /// Removes all filter sequences.
//     pub fn remove_all_seqs(&mut self) {
//         for seq in self.inner.remove_all() {
//             self.add_orphans(seq.name().clone(), seq.value.into_orphans());
//         }
//     }
//     /// Removes a filter sequence, if it exists.
//     pub fn remove_seq(&mut self, name: &str) {
//         if let Some((_index, seq)) = self.inner.remove(name) {
//             self.add_orphans(seq.name().clone(), seq.value.into_orphans());
//         }
//     }

//     fn add_orphans(&self, seq_name: String, new_orphans: OrphanedPresetData) {
//         let mut all_orphans = self.orphans.lock();
//         let seq_orphans = all_orphans.entry(seq_name).or_default();
//         for (k, v) in new_orphans {
//             seq_orphans.entry(k).or_default().extend(v);
//         }
//     }
//     fn take_orphans(&self, seq_name: &str) -> OrphanedPresetData {
//         self.orphans.lock().remove(seq_name).unwrap_or_default()
//     }
//     fn prune_orphans(&self) {
//         self.orphans.lock().retain(|_, orphan_list| {
//             orphan_list.retain(|_, orphan_sublist| {
//                 orphan_sublist.retain(|o| o.is_used_elsewhere());
//                 !orphan_sublist.is_empty()
//             });
//             !orphan_list.is_empty()
//         });
//     }
// }

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

/// Filter preset (standalone; not in a sequence).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FilterPreset {
    pub rules: Vec<FilterRule>,
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
    pub fn new(fallback_style: Option<PresetRef>) -> Self {
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
        self.set = FilterPieceSet::from_serde(&mut (), set);
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

    pub fn to_string(&self, colors: &PerColor<&str>, piece_types: &PerPieceType<&str>) -> String {
        match self {
            Self::Expr(expr) => expr.to_string(),
            Self::Checkboxes(checkboxes) => checkboxes.to_string(colors, piece_types),
        }
    }
}

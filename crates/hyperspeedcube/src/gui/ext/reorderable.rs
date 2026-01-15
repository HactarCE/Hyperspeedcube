use hcegui::reorder::{BeforeOrAfter, ReorderDndMove};
use hyperprefs::{
    FilterPresetName, FilterSeqPreset, PresetData, PresetsList, PuzzleFilterPreferences,
};
use indexmap::IndexMap;

pub trait DndReorderExt<T> {
    /// Reorders the collection, or logs a warning if it cannot.
    fn reorder_collection(self, collection: &mut T);
}

impl<K, V> DndReorderExt<IndexMap<K, V>> for ReorderDndMove {
    fn reorder_collection(self, collection: &mut IndexMap<K, V>) {
        let (i, j) = self.list_reorder_indices();
        if i < j {
            collection.move_index(i, j - 1);
        } else if j < i {
            collection.move_index(i, j);
        }
    }
}

impl DndReorderExt<PuzzleFilterPreferences> for ReorderDndMove<FilterPresetName> {
    fn reorder_collection(self, collection: &mut PuzzleFilterPreferences) {
        let payload = self.payload;
        let (target, before_or_after) = self.target;
        if payload == target || !collection.has_preset(&target) {
            return;
        }

        let preset = (|| match &payload.seq {
            Some(seq_name) => collection
                .sequences
                .get_mut(seq_name)?
                .value
                .remove(&payload.preset),
            None => collection
                .presets
                .remove(&payload.preset)
                .map(FilterSeqPreset::from),
        })();
        let Some(preset_value) = preset else {
            return;
        };

        match target.seq {
            Some(seq_name) => {
                let seq = &mut collection
                    .sequences
                    .get_mut(&seq_name)
                    .expect("filter sequence vanished!")
                    .value;
                let new_name =
                    seq.save_preset_with_nonconflicting_name(&payload.preset, preset_value);
                ReorderDndMove {
                    payload: new_name,
                    target: (target.preset, before_or_after),
                }
                .reorder_collection(seq);
            }
            None => {
                let new_name = collection
                    .presets
                    .save_preset_with_nonconflicting_name(&payload.preset, preset_value.inner);
                ReorderDndMove {
                    payload: new_name,
                    target: (target.preset, before_or_after),
                }
                .reorder_collection(&mut collection.presets);
            }
        }

        // Remove empty sequences
        if let Some(seq_name) = &payload.seq
            && let Some(seq) = collection.sequences.get(seq_name)
            && seq.value.is_empty()
        {
            collection.sequences.remove(seq_name);
        }
    }
}

impl<T: PresetData> DndReorderExt<PresetsList<T>> for ReorderDndMove<&str> {
    fn reorder_collection(self, collection: &mut PresetsList<T>) {
        let from = self.payload;
        let (to, before_or_after) = self.target;
        match before_or_after {
            BeforeOrAfter::Before => collection.reorder_user_preset_before(from, to),
            BeforeOrAfter::After => collection.reorder_user_preset_after(from, to),
        }
    }
}

impl<T: PresetData> DndReorderExt<PresetsList<T>> for ReorderDndMove<String> {
    fn reorder_collection(self, collection: &mut PresetsList<T>) {
        ReorderDndMove {
            payload: self.payload.as_str(),
            target: (self.target.0.as_str(), self.target.1),
        }
        .reorder_collection(collection);
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_reorder_presets_list() {
        let mut list = PresetsList::<()>::default();
        list.save_preset("a", ());
        list.save_preset("b", ());
        list.save_preset("c", ());
        list.save_preset("d", ());
        list.save_preset("e", ());
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "b", "c", "d", "e"],
        );
        ReorderDndMove::new("b", ("d", BeforeOrAfter::Before)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "c", "b", "d", "e"],
        );
        ReorderDndMove::new("e", ("c", BeforeOrAfter::Before)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "e", "c", "b", "d"],
        );
        ReorderDndMove::new("a", ("a", BeforeOrAfter::Before)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "e", "c", "b", "d"],
        );
        ReorderDndMove::new("e", ("d", BeforeOrAfter::After)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "c", "b", "d", "e"],
        );
        ReorderDndMove::new("b", ("a", BeforeOrAfter::After)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "b", "c", "d", "e"],
        );
        ReorderDndMove::new("b", ("b", BeforeOrAfter::After)).reorder_collection(&mut list);
        assert_eq!(
            list.user_presets().map(|p| p.name().as_str()).collect_vec(),
            vec!["a", "b", "c", "d", "e"],
        );
    }
}

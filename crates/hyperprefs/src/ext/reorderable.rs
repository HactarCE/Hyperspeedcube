use indexmap::IndexMap;

use crate::{PresetData, PresetsList};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DragAndDropResponse<Payload, End = Payload> {
    pub payload: Payload,
    pub end: End,
    pub before_or_after: Option<BeforeOrAfter>,
}

impl DragAndDropResponse<usize> {
    /// Returns the `i` and `j` such that the element at index `i` should shift
    /// to index `j`, or `None` if it cannot be determined.
    fn list_reorder_indices(self) -> Option<(usize, usize)> {
        self.before_or_after.map(|boa| match boa {
            BeforeOrAfter::Before => (self.payload, self.end),
            BeforeOrAfter::After => (self.payload, self.end + 1),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BeforeOrAfter {
    Before,
    After,
}

/// Collection whose elements can be reordered
pub trait ReorderableCollection<I> {
    /// Reorders the collection according to `drag`, or logs a warning if it
    /// cannot.
    fn reorder(&mut self, drag: DragAndDropResponse<I, I>);
}

impl<T> ReorderableCollection<usize> for Vec<T> {
    fn reorder(&mut self, drag: DragAndDropResponse<usize>) {
        let Some((i, j)) = drag.list_reorder_indices() else {
            log::error!("missing `BeforeOrAfter` in reorder");
            return;
        };

        if i < j {
            self[i..j].rotate_left(1);
        } else if j < i {
            self[j..=i].rotate_right(1);
        }
    }
}

impl<K, V> ReorderableCollection<usize> for IndexMap<K, V> {
    fn reorder(&mut self, drag: DragAndDropResponse<usize>) {
        let Some((i, j)) = drag.list_reorder_indices() else {
            log::error!("missing `BeforeOrAfter` in reorder");
            return;
        };

        if i < j {
            self.move_index(i, j - 1);
        } else if j < i {
            self.move_index(i, j);
        }
    }
}

impl<T: PresetData> ReorderableCollection<String> for PresetsList<T> {
    /// Moves the preset `from` to `to`, shifting all the presents in between.
    ///
    /// Fails silently if either `from` or `to` does not exist.
    fn reorder(&mut self, drag: DragAndDropResponse<String, String>) {
        let Some(before_or_after) = drag.before_or_after else {
            log::error!("missing `BeforeOrAfter` in reorder");
            return;
        };

        self.reorder_user_preset(&drag.payload, &drag.end, before_or_after);
    }
}

// /// Operation that can be applied to a collection using the
// /// [`EditableCollection`] trait.
// pub enum EditOp<I> {
//     Delete(I),
//     Reorder {
//         from: I,
//         to: I,
//         before_or_after: Option<BeforeOrAfter>,
//     },
// }

// /// Trait for collections where elements can be deleted and reordered.
// pub trait EditableCollection<I> {
//     fn apply(&mut self, op: EditOp<I>);
// }

// impl<T> EditableCollection<usize> for Vec<T> {
//     fn apply(&mut self, op: EditOp<usize>) {
//         match op {
//             EditOp::Delete(i) => {
//                 self.remove(i);
//             }
//             EditOp::Reorder {
//                 from,
//                 to,
//                 before_or_after,
//             } => {
//                 let (i, j) = match before_or_after {
//                     Some(BeforeOrAfter::Before) => (from, to),
//                     Some(BeforeOrAfter::After) => (from, to + 1),
//                     None => {
//                         log::error!("missing BeforeOrAfter in reorder");
//                         return;
//                     }
//                 };

//                 if i < j {
//                     self[i..j].rotate_left(1);
//                 } else if j < i {
//                     self[j..=i].rotate_right(1);
//                 }
//             }
//         }
//     }
// }

// impl<K, V> EditableCollection<usize> for IndexMap<K, V> {
//     fn apply(&mut self, op: EditOp<usize>) {
//         match op {
//             EditOp::Delete(i) => {
//                 self.shift_remove_index(i);
//             }
//             EditOp::Reorder {
//                 from,
//                 to,
//                 before_or_after,
//             } => {
//                 let (i, j) = match before_or_after {
//                     Some(BeforeOrAfter::Before) => (from, to),
//                     Some(BeforeOrAfter::After) => (from, to + 1),
//                     None => {
//                         log::error!("missing BeforeOrAfter in reorder");
//                         return;
//                     }
//                 };

//                 if i < j {
//                     self.move_index(i, j - 1);
//                 } else if j < i {
//                     self.move_index(i, j);
//                 }
//             }
//         }
//     }
// }

// TODO: test reordering

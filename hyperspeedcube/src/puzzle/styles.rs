use std::collections::HashMap;

use bitvec::prelude::*;

use crate::preferences::{PieceStyle, StyleId, StylePreferences};

/// Style (selected, hovered, hidden, etc.) for each piece in a puzzle.
#[derive(Debug, Clone)]
pub struct PuzzleStyleStates {
    /// Number of pieces in the puzzle.
    piece_count: usize,
    /// Sets of pieces with the same decorations.
    peice_sets: HashMap<PieceStyleState, BitBox<u64>>,
}
impl PuzzleStyleStates {
    /// Constructs a new `PieceStyleStates` with all pieces in the default
    /// style.
    pub fn new(piece_count: usize) -> Self {
        let all_pieces = BitVec::repeat(true, piece_count).into_boxed_bitslice();
        Self {
            piece_count,
            peice_sets: HashMap::from_iter([(PieceStyleState::default(), all_pieces)]),
        }
    }

    /// Modifies the states of a piece set, given their current state.
    ///
    /// `modify_state` is expected to be a pure function.
    pub fn set_piece_states(
        &mut self,
        piece_set: &BitBox<u64>,
        modify_state: impl Fn(PieceStyleState) -> PieceStyleState,
    ) {
        use std::collections::hash_map::Entry;

        debug_assert_eq!(piece_set.len(), self.piece_count, "piece count mismatch");

        let inv_piece_set = !piece_set.clone();

        for (old_state, old_pieces) in std::mem::take(&mut self.peice_sets) {
            let new_state = modify_state(old_state);
            if new_state != old_state {
                let unchanged_pieces = old_pieces.clone() & &inv_piece_set;
                let changed_pieces = old_pieces.clone() & piece_set;
                self.raw_set_piece_states(unchanged_pieces, old_state);
                self.raw_set_piece_states(changed_pieces, new_state);
            } else {
                self.raw_set_piece_states(old_pieces, old_state);
            }
        }
    }

    fn raw_set_piece_states(&mut self, peice_set: BitBox<u64>, state: PieceStyleState) {
        if peice_set.any() {
            match self.peice_sets.entry(state) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    *e.get_mut() |= peice_set;
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(peice_set);
                }
            }
        }
    }

    /// Returns whether any peice in `piece_set` is hidden.
    pub fn is_any_hidden(&self, piece_set: &BitBox<u64>) -> bool {
        self.peice_sets
            .iter()
            .any(|(style_state, styled_piece_set)| {
                style_state.hidden && {
                    let mut intersection = styled_piece_set.clone();
                    intersection &= piece_set;
                    intersection.any()
                }
            })
    }

    /// Returns the set of pieces that are interactable (can be hovered with the
    /// cursor).
    pub fn interactable_pieces(&self, styles: &StylePreferences) -> BitBox<u64> {
        self.filter_pieces_by_style(|s| s.interactable(styles))
    }

    /// Returns the set of pieces for which `filter_fn` returns `true` on their
    /// style.
    pub fn filter_pieces_by_style(
        &self,
        filter_fn: impl Fn(PieceStyleState) -> bool,
    ) -> BitBox<u64> {
        self.peice_sets
            .iter()
            .filter(|(style_state, _piece_set)| filter_fn(**style_state))
            .map(|(_style_state, piece_set)| piece_set)
            .fold(bitbox![u64, Lsb0; 0; self.piece_count], |a, b| a | b)
    }

    /// Returns the style values for each set of pieces.
    pub fn values(&self, prefs: &StylePreferences) -> Vec<(PieceStyleValues, BitBox<u64>)> {
        self.peice_sets
            .iter()
            .map(|(style_state, piece_set)| (style_state.values(prefs), piece_set.clone()))
            .collect()
    }
}

/// Values for how to draw a piece, depending on its style state.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct PieceStyleValues {
    pub face_opacity: u8, // TODO: linear or gamma??
    pub face_color: [u8; 3],
    pub face_sticker_color: bool,

    pub outline_opacity: u8,
    pub outline_color: [u8; 3],
    pub outline_sticker_color: bool,

    pub outline_size: f32,
}

/// Style state for a piece.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceStyleState {
    pub base: StyleId,

    pub hidden: bool,
    pub blind: bool,
    pub gripped: bool,
    pub ungripped: bool,
    pub hovered_piece: bool,
    pub hovered_sticker: bool,
    pub selected_piece: bool,
    pub selected_sticker: bool,
}
impl PieceStyleState {
    /// Returns whether a piece with this style state is interactable (can be
    /// hovered with the cursor).
    fn interactable(self, styles: &StylePreferences) -> bool {
        let base = styles
            .custom
            .values()
            .find(|s| s.id == self.base)
            .and_then(|s| s.interactable);
        let hid = self.hidden.then_some(false);
        let ugp = self.ungripped.then_some(false);
        hid.or(ugp).or(base).unwrap_or(true)
    }

    /// Returns how to draw a piece with this style state.
    fn values(self, styles: &StylePreferences) -> PieceStyleValues {
        let def = styles.default;
        let base = styles.custom.values().find(|s| s.id == self.base).copied();
        let hid = self.hidden.then_some(styles.hidden);
        let mut bld = self.blind.then_some(styles.blind);
        let gp = self.gripped.then_some(styles.gripped);
        let ugp = self.ungripped.then_some(styles.ungripped);
        let hp = self.hovered_piece.then_some(styles.hovered_piece);
        let hs = self.hovered_sticker.then_some(styles.hovered_sticker);
        let sp = self.selected_piece.then_some(styles.selected_piece);
        let ss = self.selected_sticker.then_some(styles.selected_sticker);

        fn min(xs: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
            xs.into_iter().filter_map(|x| x).min_by(f32::total_cmp)
        }
        fn max(xs: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
            xs.into_iter().filter_map(|x| x).max_by(f32::total_cmp)
        }
        fn first_or_default<T: Default>(xs: impl IntoIterator<Item = Option<T>>) -> T {
            xs.into_iter().find_map(|x| x).unwrap_or_default()
        }

        // Ensure that blindfolded faces do not reveal information.
        if let Some(style) = &mut bld {
            style.face_sticker_color = Some(false);
            style.outline_sticker_color = Some(false);
        }

        let color_order = [bld, hs, hp, ss, sp, ugp, gp, hid, base, Some(def)];
        let opacity_order = [hs, hp, ss, sp, gp, hid];
        let size_order = [hs, hp, ss, sp, ugp, gp, hid, base, bld, Some(def)];

        fn f32_to_u8(f: f32) -> u8 {
            (f.clamp(0.0, 1.0) * 255.0) as u8
        }

        use crate::util::color_to_u8x3;

        // Apply styles in order from highest priority to lowest priority.
        PieceStyleValues {
            face_opacity: f32_to_u8(
                min([
                    ugp.and_then(|s| s.face_opacity),
                    max(opacity_order.map(|s| s?.face_opacity)),
                ])
                .or(base.and_then(|s| s.face_opacity))
                .unwrap_or(def.face_opacity.unwrap_or_default()),
            ),
            face_color: color_to_u8x3(first_or_default(color_order.map(|s| s?.face_color))),
            face_sticker_color: first_or_default(color_order.map(|s| s?.face_sticker_color)),

            outline_opacity: f32_to_u8(
                min([
                    ugp.and_then(|s| s.outline_opacity),
                    max(opacity_order.map(|s| s?.outline_opacity)),
                ])
                .or(base.and_then(|s| s.outline_opacity))
                .unwrap_or(def.outline_opacity.unwrap_or_default()),
            ),
            outline_color: color_to_u8x3(first_or_default(color_order.map(|s| s?.outline_color))),
            outline_sticker_color: first_or_default(color_order.map(|s| s?.outline_sticker_color)),

            outline_size: first_or_default(size_order.map(|s| s?.outline_size)),
        }
    }
}

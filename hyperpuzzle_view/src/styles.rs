use std::collections::HashMap;

use hyperdraw::PieceStyleValues;
use hyperprefs::{Preferences, PresetRef};
use hyperpuzzle::{Piece, PieceMask};

/// Returns a closure that updates the given style state.
#[macro_export]
macro_rules! update_styles {
    ($($field:ident = $value:expr),* $(,)?) => {
        (|mut styles| {
            $(styles.$field = $value;)*
            styles
        })
    };
}

/// Style (selected, hovered, hidden, etc.) for each piece in a puzzle.
#[derive(Debug)]
pub struct PuzzleStyleStates {
    /// Number of pieces in the puzzle.
    piece_count: usize,
    /// Sets of pieces with the same decorations.
    piece_sets: HashMap<PieceStyleState, PieceMask>,

    cached_hovered_piece: Option<Piece>,
    cached_blocking_pieces: (PieceMask, f32),
}
impl PuzzleStyleStates {
    /// Constructs a new `PieceStyleStates` with all pieces in the default
    /// style.
    pub fn new(piece_count: usize) -> Self {
        Self {
            piece_count,
            piece_sets: HashMap::from_iter([(
                PieceStyleState::default(),
                PieceMask::new_full(piece_count),
            )]),

            cached_hovered_piece: None,
            cached_blocking_pieces: (PieceMask::new_empty(piece_count), 0.0),
        }
    }

    pub fn set_base_styles(&mut self, pieces: &PieceMask, base: Option<PresetRef>) {
        self.set_piece_states(pieces, update_styles!(base = base.clone()));
    }

    /// Modifies the states of a piece set, given their current state.
    ///
    /// `modify_state` is expected to be a pure function.
    pub fn set_piece_states(
        &mut self,
        pieces: &PieceMask,
        modify_state: impl Fn(PieceStyleState) -> PieceStyleState,
    ) {
        // Early exit
        if pieces.is_empty() {
            return;
        }

        self.set_piece_states_with_opposite(pieces, modify_state, |style| style);
    }

    /// Sets the hovered piece.
    pub fn set_hovered_piece(&mut self, piece: Option<Piece>) {
        if self.cached_hovered_piece == piece {
            return;
        }
        self.cached_hovered_piece = piece;

        self.set_piece_states_with_opposite(
            &PieceMask::from_iter(self.piece_count, piece),
            update_styles!(hovered_piece = true),
            update_styles!(hovered_piece = false),
        );
    }
    pub fn set_blocking_pieces(&mut self, pieces: PieceMask, blocking_amount: f32) {
        let new_cache_value = (pieces.clone(), blocking_amount);
        if self.cached_blocking_pieces == new_cache_value {
            return;
        }
        self.cached_blocking_pieces = new_cache_value;

        // TODO: make crate-wide function for f32<->u8 conversion
        let amt = (blocking_amount * 255.0) as u8;
        self.set_piece_states_with_opposite(
            &pieces,
            update_styles!(blocking_amount = amt),
            update_styles!(blocking_amount = 0),
        );
    }

    /// Modifies the states of all pieces, given their current state, depending
    /// on whether they are in the set.
    ///
    /// `modify_state_in_set` and `modify_state_not_in_set` are expected to be
    /// pure functions.
    pub fn set_piece_states_with_opposite(
        &mut self,
        pieces: &PieceMask,
        modify_state_in_set: impl Fn(PieceStyleState) -> PieceStyleState,
        modify_state_not_in_set: impl Fn(PieceStyleState) -> PieceStyleState,
    ) {
        debug_assert_eq!(pieces.max_len(), self.piece_count, "piece count mismatch");

        let inv_pieces = !pieces.clone();

        for (old_state, old_pieces) in std::mem::take(&mut self.piece_sets) {
            let new_state_in_set = modify_state_in_set(old_state.clone());
            let new_state_not_in_set = modify_state_not_in_set(old_state);
            if new_state_in_set != new_state_not_in_set {
                let pieces_in_set = old_pieces.clone() & pieces;
                let pieces_not_in_set = old_pieces.clone() & &inv_pieces;
                self.raw_set_piece_states(pieces_in_set, new_state_in_set);
                self.raw_set_piece_states(pieces_not_in_set, new_state_not_in_set);
            } else {
                self.raw_set_piece_states(old_pieces, new_state_in_set);
            }
        }
    }

    fn raw_set_piece_states(&mut self, pieces: PieceMask, state: PieceStyleState) {
        if !pieces.is_empty() {
            match self.piece_sets.entry(state) {
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    *e.get_mut() |= pieces;
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(pieces);
                }
            }
        }
    }

    /// Returns the set of pieces that are interactable (can be hovered with the
    /// cursor).
    pub fn interactable_pieces(&self, prefs: &Preferences) -> PieceMask {
        self.filter_pieces_by_style(|s| s.interactable(prefs))
    }

    /// Returns the set of pieces for which `filter_fn` returns `true` on their
    /// style.
    pub fn filter_pieces_by_style(
        &self,
        filter_fn: impl Fn(&PieceStyleState) -> bool,
    ) -> PieceMask {
        self.piece_sets
            .iter()
            .filter(|(style_state, _piece_set)| filter_fn(style_state))
            .map(|(_style_state, piece_set)| piece_set)
            .fold(PieceMask::new_empty(self.piece_count), |a, b| a | b)
    }

    /// Returns the style values for each set of pieces.
    pub fn values(&self, prefs: &Preferences) -> Vec<(PieceStyleValues, PieceMask)> {
        self.piece_sets
            .iter()
            .map(|(style_state, piece_set)| (style_state.values(prefs), piece_set.clone()))
            .collect()
    }
}

/// Style state for a piece.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PieceStyleState {
    pub base: Option<PresetRef>,

    pub blind: bool,
    pub gripped: bool,
    pub ungripped: bool,
    pub hovered_piece: bool,
    pub hovered_sticker: bool,
    pub selected_piece: bool,
    pub selected_sticker: bool,
    pub blocking_amount: u8,
}
impl PieceStyleState {
    /// Returns whether a piece with this style state is interactable (can be
    /// hovered with the cursor).
    fn interactable(&self, prefs: &Preferences) -> bool {
        let base = prefs.base_style(&self.base).interactable;
        let ugp = self.ungripped.then_some(false);
        ugp.or(base).unwrap_or(true)
    }

    /// Returns how to draw a piece with this style state.
    fn values(&self, prefs: &Preferences) -> PieceStyleValues {
        let styles = &prefs.styles;

        let def = styles.default;
        let base = prefs.base_style(&self.base);
        let bld = self.blind.then_some(styles.blind);
        let gp = self.gripped.then_some(styles.gripped);
        let ugp = self.ungripped.then_some(styles.ungripped);
        let hp = self.hovered_piece.then_some(styles.hovered_piece);
        let hs = self.hovered_sticker.then_some(styles.hovered_piece); // may be different in the future
        let sp = self.selected_piece.then_some(styles.selected_piece);
        let ss = self.selected_sticker.then_some(styles.selected_piece); // may be different in the future

        fn min(xs: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
            xs.into_iter().flatten().min_by(f32::total_cmp)
        }
        fn max(xs: impl IntoIterator<Item = Option<f32>>) -> Option<f32> {
            xs.into_iter().flatten().max_by(f32::total_cmp)
        }
        fn first_or_default<T: Default>(xs: impl IntoIterator<Item = Option<T>>) -> T {
            xs.into_iter().find_map(|x| x).unwrap_or_default()
        }

        let color_order = [hs, hp, ss, sp, ugp, gp, bld, Some(base)];
        let opacity_order = [hs, hp, ss, sp, gp, bld, Some(base)]; // `ugp` and `def` is handled separately
        let size_order = [hs, hp, ss, sp, ugp, gp, bld, Some(base)];

        fn f32_to_u8(f: f32) -> u8 {
            (f.clamp(0.0, 1.0) * 255.0) as u8
        }

        // Apply styles in order from highest priority to lowest priority. Use
        // `.is_bld_safe()` to ensure that we do not reveal information about
        // blindfolded stickers.
        PieceStyleValues {
            face_opacity: f32_to_u8(
                min([
                    ugp.and_then(|s| s.face_opacity),
                    max(opacity_order.map(|s| s?.face_opacity)),
                ])
                .or(base.face_opacity)
                .unwrap_or(def.face_opacity.unwrap_or_default()),
            ),
            face_color: first_or_default(
                color_order.map(|s| s?.face_color.filter(|c| c.is_bld_safe(self.blind))),
            ),

            outline_opacity: f32_to_u8(
                min([
                    ugp.and_then(|s| s.outline_opacity),
                    max(opacity_order.map(|s| s?.outline_opacity)),
                ])
                .or(base.outline_opacity)
                .unwrap_or(def.outline_opacity.unwrap_or_default()),
            ),
            outline_color: first_or_default(
                color_order.map(|s| s?.outline_color.filter(|c| c.is_bld_safe(self.blind))),
            )
            .map_fixed_color(|c| {
                // TODO: blocking pieces animation is invisible when using
                // `StyleColorMode::FromSticker`
                hyperpuzzle::Rgb::mix(
                    c,
                    styles.blocking_outline_color,
                    self.blocking_amount as f32 / 255.0,
                )
            }),
            outline_lighting: first_or_default(color_order.map(|s| s?.outline_lighting)),

            outline_size: hypermath::util::lerp(
                first_or_default(size_order.map(|s| s?.outline_size)),
                styles.blocking_outline_size,
                self.blocking_amount as f32 / 255.0,
            ),
        }
    }
}

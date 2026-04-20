use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Weak};

use hypuz_notation::AxisLayersInfo;
use rand::Rng;
use scramble::{ScrambleProgress, ScrambledPuzzle};

use super::*;
use crate::{BoxDynPuzzleState, BoxDynPuzzleUiData, CatalogMetadata, Move};

/// Puzzle type info.
pub struct Puzzle {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Puzzle>,

    /// Catalog metadata.
    pub meta: Arc<CatalogMetadata>,

    /// Set of view preferences to use for the puzzle.
    pub view_prefs_set: Option<PuzzleViewPreferencesSet>,

    /// List of pieces, indexed by ID.
    pub pieces: PerPiece<PieceInfo>,
    /// List of stickers, indexed by ID.
    pub stickers: PerSticker<StickerInfo>,
    /// List of piece types, indexed by ID.
    pub piece_types: PerPieceType<PieceTypeInfo>,
    /// Hierarchy of piece types, in order.
    pub piece_type_hierarchy: PieceTypeHierarchy,
    /// Map from piece type names (including piece type _category_ names) to a
    /// set of pieces that have that type.
    pub piece_type_masks: HashMap<String, PieceMask>,

    /// Color system.
    pub colors: Arc<ColorSystem>,

    /// Returns whether or not the puzzle can be scrambled.
    pub can_scramble: bool,
    /// Number of moves for a full scramble.
    pub full_scramble_length: u32,

    /// Layer info for each axis.
    pub axis_layers: PerAxis<AxisLayersInfo>,
    /// Twist system.
    pub twists: Arc<TwistSystem>,

    /// Data for rendering and interacting with the puzzle.
    pub ui_data: BoxDynPuzzleUiData,

    /// Constructor for a solved puzzle state.
    pub new: Box<dyn Send + Sync + Fn(Arc<Self>) -> BoxDynPuzzleState>,

    /// Random move generator for scrambling. The output of this function must
    /// depend only on the state of the RNG. It must return `None` if and only
    /// if the puzzle has no twists.
    pub random_move: Box<dyn Send + Sync + Fn(&mut dyn Rng) -> Option<Move>>,
}

impl fmt::Debug for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Puzzle")
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

/// Compare by puzzle ID.
impl PartialEq for Puzzle {
    fn eq(&self, other: &Self) -> bool {
        self.meta.id == other.meta.id
    }
}
/// Compare by puzzle ID.
impl Eq for Puzzle {}

/// Compare by metadata.
impl PartialOrd for Puzzle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// Compare by metadata.
impl Ord for Puzzle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.meta.cmp(&other.meta)
    }
}

impl Puzzle {
    /// Returns an `Arc` reference to the puzzle type.
    pub fn arc(&self) -> Arc<Self> {
        self.this.upgrade().expect("`Puzzle` removed from `Arc`")
    }
    /// Constructs a new instance of the puzzle.
    pub fn new_solved_state(&self) -> BoxDynPuzzleState {
        (self.new)(self.arc())
    }
    /// Constructs a new scrambled instance of the puzzle.
    pub fn new_scrambled(&self, params: ScrambleParams) -> ScrambledPuzzle {
        self.new_scrambled_with_progress(params, None)
            .expect("scramble unexpectedly canceled")
    }
    /// Constructs a new scrambled instance of the puzzle.
    ///
    /// Takes an optional `progress` argument that tracks progress.
    pub fn new_scrambled_with_progress(
        &self,
        params: ScrambleParams,
        progress: Option<Arc<ScrambleProgress>>,
    ) -> Result<ScrambledPuzzle, ScrambleError> {
        if params.version != 2 {
            return Err(ScrambleError::UnsupportedVersion);
        }

        let ScrambleParams { ty, seed, .. } = &params;

        let mut rng = crate::util::rng_from_seed(seed);

        let scramble_length = match ty {
            ScrambleType::Full => self.full_scramble_length,
            ScrambleType::Partial(n) => *n,
        };

        if let Some(progress) = &progress {
            progress.set_total(scramble_length);
        }

        let random_twists = std::iter::from_fn(|| (self.random_move)(&mut rng)?.into());

        let mut twists_applied = vec![];
        let mut state = self.new_solved_state();
        for (i, twist) in random_twists.take(scramble_length as usize).enumerate() {
            if let Ok(new_state) = state.do_twist_dyn(&twist) {
                twists_applied.push(twist);
                state = new_state;
            }
            if let Some(progress) = &progress {
                if progress.is_cancel_requested() {
                    return Err(ScrambleError::Canceled);
                }
                progress.set_progress(i as u32);
            }
        }

        Ok(ScrambledPuzzle {
            params,
            twists: twists_applied,
            state,
        })
    }

    /// Returns the axis system.
    ///
    /// This is a shortcut for `.twists.axes()`.
    pub fn axes(&self) -> &Arc<AxisSystem> {
        &self.twists.axes
    }

    /// Returns all orbits used to construct the puzzle.
    pub fn orbits(&self) -> Vec<AnyOrbit> {
        itertools::chain(
            self.axes().orbits.iter().cloned().map(AnyOrbit::Axes),
            self.colors.orbits.iter().cloned().map(AnyOrbit::Colors),
        )
        .collect()
    }

    /// Returns which view preferences UI to display for the puzzle.
    pub fn view_prefs_set(&self) -> Option<PuzzleViewPreferencesSet> {
        self.view_prefs_set
    }

    /// Returns whether the piece has a sticker with the given color.
    pub fn piece_has_color(&self, piece: Piece, color: Color) -> bool {
        self.pieces[piece].stickers.iter().any(|&sticker| {
            let sticker_info = &self.stickers[sticker];
            sticker_info.color == color
        })
    }
}

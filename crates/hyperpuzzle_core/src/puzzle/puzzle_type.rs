use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::sync::{Arc, Weak};

use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use scramble::{ScrambleProgress, ScrambledPuzzle};
use sha2::Digest;

use super::*;
use crate::{BoxDynPuzzleState, BoxDynPuzzleUiData, PuzzleListMetadata};

/// Puzzle type info.
pub struct Puzzle {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Puzzle>,

    /// Metadata for the puzzle.
    pub meta: Arc<PuzzleListMetadata>,

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

    /// Set of twists used to scramble the puzzle, in a future-compatible order.
    pub scramble_twists: Vec<Twist>,
    /// Number of moves for a full scramble.
    pub full_scramble_length: u32,

    /// Move notation.
    pub notation: Notation,

    /// Layers for each axis.
    pub axis_layers: PerAxis<AxisLayersInfo>,
    /// For each axis, its opposite axis if there is one.
    ///
    /// This is important for Slice Turn Metric calculations.
    pub axis_opposites: PerAxis<Option<Axis>>,
    /// Twist system.
    pub twists: Arc<TwistSystem>,

    /// Data for rendering and interacting with the puzzle.
    pub ui_data: BoxDynPuzzleUiData,

    /// Constructor for a solved puzzle state.
    pub new: Box<dyn Send + Sync + Fn(Arc<Self>) -> BoxDynPuzzleState>,
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
    #[allow(clippy::unwrap_used)] // these are infallible
    pub fn new_scrambled_with_progress(
        &self,
        params: ScrambleParams,
        progress: Option<Arc<ScrambleProgress>>,
    ) -> Option<ScrambledPuzzle> {
        let ScrambleParams { ty, seed, .. } = &params;

        let mut sha256 = sha2::Sha256::new();
        sha256.write_all(&seed.len().to_le_bytes()).unwrap(); // native endianness on x86 and Apple Silicon
        sha256.write_all(seed.as_bytes()).unwrap();
        let digest = sha256.finalize();

        let mut rng =
            rand_chacha::ChaCha12Rng::from_seed(<[u8; 32]>::try_from(&digest[..32]).unwrap());

        let scramble_length = match ty {
            ScrambleType::Full => self.full_scramble_length,
            ScrambleType::Partial(n) => *n,
        };

        if let Some(progress) = &progress {
            progress.set_total(scramble_length);
        }

        let random_twists = std::iter::from_fn(|| {
            let random_twist = *self.scramble_twists.choose(&mut rng)?;

            let axis = self.twists.twists[random_twist].axis;
            let layer_count = self.axis_layers[axis].len().max(1) as crate::LayerMaskUint;
            let random_layer_mask = LayerMask(rng.random_range(1..(1 << layer_count)));

            Some(LayeredTwist {
                layers: random_layer_mask,
                transform: random_twist,
            })
        });

        let mut twists_applied = vec![];
        let mut state = self.new_solved_state();
        for (i, twist) in random_twists.take(scramble_length as usize).enumerate() {
            if let Ok(new_state) = state.do_twist_dyn(twist) {
                twists_applied.push(twist);
                state = new_state;
            }
            if let Some(progress) = &progress {
                if progress.is_cancel_requested() {
                    return None;
                }
                progress.set_progress(i as u32);
            }
        }

        Some(ScrambledPuzzle {
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

    /// Returns whether the puzzle can be scrambled.
    pub fn can_scramble(&self) -> bool {
        !self.scramble_twists.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn test_stable_deterministic_scrambles() {
        let mut rng = rand_chacha::ChaCha12Rng::from_seed((0..32).collect_array().unwrap());
        let a = rng.random::<[u64; 4]>();
        assert_eq!(
            a,
            [
                6829280927315210738,
                12268062495221155140,
                13566740668459520841,
                3898457950037656553
            ]
        );
    }
}

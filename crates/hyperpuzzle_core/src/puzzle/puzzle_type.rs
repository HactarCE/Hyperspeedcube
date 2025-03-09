use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::sync::{Arc, Weak};

use hypershape::Space;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use scramble::{ScrambleProgress, ScrambledPuzzle};
use sha2::Digest;

use super::*;
use crate::{PuzzleListMetadata, TagSet, Version};

lazy_static! {
    /// Hard-coded placeholder puzzle with no pieces, no stickers, no mesh, etc.
    pub static ref PLACEHOLDER_PUZZLE: Arc<Puzzle> = Arc::new_cyclic(|this| Puzzle {
        this: Weak::clone(this),
        meta: PuzzleListMetadata {
            id: "~placeholder".to_string(),
            version: Version::PLACEHOLDER,
            name: "🤔".to_string(),
            aliases: vec![],
            tags: TagSet::new(),
        },
        space: Space::new(3),
        mesh: Mesh::new_empty(3),
        pieces: PerPiece::new(),
        stickers: PerSticker::new(),
        piece_types: PerPieceType::new(),
        piece_type_hierarchy: PieceTypeHierarchy::new(0),
        piece_type_masks: HashMap::new(),
        colors: Arc::new(ColorSystem::new_empty()),
        scramble_twists: vec![],
        full_scramble_length: 0,
        notation: Notation {},
        axes: PerAxis::new(),
        axis_by_name: HashMap::new(),
        twists: PerTwist::new(),
        twist_by_name: HashMap::new(),
        gizmo_twists: PerGizmoFace::new(),
        dev_data: PuzzleDevData::new(),

        new: Box::new(PuzzleState::new),
    });
}

/// Puzzle type info.
pub struct Puzzle {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Puzzle>,

    /// Metadata for the puzzle.
    pub meta: PuzzleListMetadata,

    /// Space containing a polytope for each piece.
    // TODO: evaluate where this is used and how we can remove it
    pub space: Arc<Space>,
    /// Puzzle mesh for rendering.
    pub mesh: Mesh,

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

    /// List of axes, indexed by ID.
    pub axes: PerAxis<AxisInfo>,
    /// Map from axis name to axis.
    pub axis_by_name: HashMap<String, Axis>,

    /// List of twists, indexed by ID.
    pub twists: PerTwist<TwistInfo>,
    /// Map from twist name to twist.
    pub twist_by_name: HashMap<String, Twist>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Twist>,

    /// Data for puzzle developers.
    pub dev_data: PuzzleDevData,

    /// Constructor for a solved puzzle state.
    pub new: Box<dyn Send + Sync + Fn(Arc<Self>) -> PuzzleState>,
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
    pub fn new_solved_state(&self) -> PuzzleState {
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
        let ScrambleParams { ty, time, seed } = &params;

        let mut sha256 = sha2::Sha256::new();
        sha256.write_all(time.to_string().as_bytes()).unwrap();
        sha256.write_all(&seed.len().to_le_bytes()).unwrap();
        sha256.write_all(seed.as_bytes()).unwrap(); // native endianness on x86 and Apple Silicon
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

            let axis = self.twists[random_twist].axis;
            let layer_count = self.axes[axis].layers.len().max(1) as crate::LayerMaskUint;
            let random_layer_mask = LayerMask(rng.random_range(1..(1 << layer_count)));

            Some(LayeredTwist {
                layers: random_layer_mask,
                transform: random_twist,
            })
        });

        let mut twists_applied = vec![];
        let mut state = self.new_solved_state();
        for (i, twist) in random_twists.take(scramble_length as usize).enumerate() {
            if let Ok(new_state) = state.do_twist(twist) {
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

    /// Returns the number of dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.mesh.ndim
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

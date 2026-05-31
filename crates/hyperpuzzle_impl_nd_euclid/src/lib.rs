//! N-dimensional Euclidean puzzle simulation backend and Hyperpuzzlescript API
//! for Hyperspeedcube.

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use hypermath::pga;
use hyperpuzzle_core::ComponentList;
use hyperpuzzle_core::prelude::*;

mod anim;
pub mod builder;
mod components;
mod geom;
pub mod hps;
mod layers;
mod state;

mod twist_key;
mod vantage_group;

pub use anim::NdEuclidPuzzleAnimation;
pub use components::{
    NamedTwistsList, NdEuclidAxisVectors, NdEuclidTwistsList, NdEuclidViewOffset,
    PgaMotorToNearestTwist, TwistToPgaMotor,
};
pub use geom::NdEuclidPuzzleGeometry;
pub use layers::{LayerDepths, PuzzleLayerDepths};
pub use state::NdEuclidPuzzleState;
pub use twist_key::TwistKey;
pub use vantage_group::{
    NdEuclidRelativeAxis, NdEuclidRelativeTwist, NdEuclidVantageGroup, NdEuclidVantageGroupElement,
    PerReferenceVector, ReferenceVector,
};

/// Maximum period of a twist.
const MAX_TWIST_REPEAT: usize = 1000;

/// Prelude of common imports.
pub mod prelude {
    pub use crate::{
        NdEuclidPuzzleAnimation, NdEuclidPuzzleGeometry, NdEuclidPuzzleState,
        NdEuclidPuzzleStateRenderData,
    };
}

/// Puzzle render data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleStateRenderData {
    /// Transform for each piece.
    pub piece_transforms: PerPiece<pga::Motor>,
}

impl PuzzleStateRenderData for NdEuclidPuzzleStateRenderData {}

lazy_static! {
    /// Hard-coded placeholder puzzle with no pieces, no stickers, no mesh, etc.
    pub static ref PLACEHOLDER_PUZZLE: Arc<Puzzle> = {
        let mut components = ComponentList::new();
        let geom = Arc::new(NdEuclidPuzzleGeometry::placeholder());
        components.insert(Arc::clone(&geom));

        Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(CatalogMetadata {
                id: CatalogId::new("placeholder",[]).expect("bad placeholder ID"),
                version: Version::PLACEHOLDER,
                name: "🤔".to_string(),
                aliases: vec![],
                tags: TagSet::new(),
            }),
            view_prefs_set: None,
            pieces: PerPiece::new(),
            stickers: PerSticker::new(),
            piece_types: PerPieceType::new(),
            piece_type_hierarchy: PieceTypeHierarchy::new(0),
            piece_type_masks: HashMap::new(),
            colors: Arc::new(ColorSystem::new_empty()),
            can_scramble: false,
            full_scramble_length: 0,
            axis_layers: PerAxis::new(),
            twists: Arc::new(TwistSystem::new_empty()),
            new: Box::new(move |this| NdEuclidPuzzleState::new(this, Arc::clone(&geom)).into()),
            random_move: Box::new(move |_rng| None),
            components,
        })
    };
}

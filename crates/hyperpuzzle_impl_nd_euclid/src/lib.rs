//! N-dimensional Euclidean puzzle simulation backend for Hyperspeedcube.
//!
//! This crate provides the implementation for storing and simulating these
//! puzzles. See `hyperpuzzle_lua` for generating them.

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use hypermath::pga;
use hyperpuzzle_core::prelude::*;

mod anim;
pub mod builder;
mod engine;
mod geom;
mod state;
mod twist_key;
mod vantage_group;

pub use anim::NdEuclidPuzzleAnimation;
pub use engine::{NdEuclidTwistSystemEngineData, NdEuclidVantageSetEngineData};
pub use geom::NdEuclidPuzzleGeometry;
pub use state::NdEuclidPuzzleState;
pub use twist_key::TwistKey;
pub use vantage_group::{
    NdEuclidRelativeAxis, NdEuclidRelativeTwist, NdEuclidVantageGroup, NdEuclidVantageGroupElement,
    PerReferenceVector, ReferenceVector,
};

/// Prefix for ad-hoc color system and twist system IDs.
const PUZZLE_PREFIX: &str = "puzzle:";

/// Prelude of common imports.
pub mod prelude {
    pub use crate::{
        NdEuclidPuzzleAnimation, NdEuclidPuzzleGeometry, NdEuclidPuzzleState,
        NdEuclidPuzzleStateRenderData, NdEuclidPuzzleUiData,
    };
}

/// Puzzle render data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleStateRenderData {
    /// Transform for each piece.
    pub piece_transforms: PerPiece<pga::Motor>,
}
impl PuzzleStateRenderData for NdEuclidPuzzleStateRenderData {}

/// UI rendering & interaction data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleUiData(Arc<NdEuclidPuzzleGeometry>);
impl PuzzleUiData for NdEuclidPuzzleUiData {}
impl NdEuclidPuzzleUiData {
    /// Wraps an `Arc<NdEuclidPuzzleGeometry>` to form a [`BoxDynPuzzleUiData`].
    pub fn new_dyn(geom: &Arc<NdEuclidPuzzleGeometry>) -> BoxDynPuzzleUiData {
        Self(Arc::clone(geom)).into()
    }
    /// Returns the underlying [`NdEuclidPuzzleGeometry`].
    pub fn geom(&self) -> Arc<NdEuclidPuzzleGeometry> {
        Arc::clone(&self.0)
    }
}

lazy_static! {
    /// Hard-coded placeholder puzzle with no pieces, no stickers, no mesh, etc.
    pub static ref PLACEHOLDER_PUZZLE: Arc<Puzzle> = {
        let axes = Arc::new(AxisSystem::new_empty());
        let twists = Arc::new(TwistSystem::new_empty(&axes));
        let geom = Arc::new(NdEuclidPuzzleGeometry::placeholder());
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);
        Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(PuzzleListMetadata {
                id: "~placeholder".to_string(),
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
            scramble_twists: vec![],
            full_scramble_length: 0,
            notation: Notation {},
            axis_layers: PerAxis::new(),
            axis_opposites: PerAxis::new(),
            twists,
            ui_data,
            new: Box::new(move |this| NdEuclidPuzzleState::new(this, Arc::clone(&geom)).into()),
        })
    };
}

use std::sync::Arc;

use hypermath::{Hyperplane, Vector, pga};
use hypershape::{PolytopeId, Space};

mod state;

pub use state::NdEuclidPuzzleState;

use crate::{
    BoxDynPuzzleAnimation, Mesh, PerAxis, PerGizmoFace, PerPiece, PerSticker, PerTwist, PieceMask,
    PuzzleAnimation, PuzzleStateRenderData, PuzzleUiData, Twist,
};

/// UI rendering & interaction data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleGeometry {
    // TODO: just record the vertex set for each polytope because that's all we need
    pub space: Arc<Space>,

    /// Puzzle mesh for rendering.
    pub mesh: Mesh,

    /// Polytope for each piece, used to compute its grip.
    pub piece_polytopes: PerPiece<PolytopeId>,

    /// Plane for each sticker, used to compute whether the puzzle is solved.
    pub sticker_planes: PerSticker<Hyperplane>, // TODO: avoid storing a bunch of duplicates

    /// Vector for each axis.
    ///
    /// The axis vector is perpendicular to all layer boundaries on the axis and
    /// is fixed by all turns on the axis.
    pub axis_vectors: PerAxis<Vector>,

    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: PerTwist<pga::Motor>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Twist>,
}
impl PuzzleUiData for NdEuclidPuzzleGeometry {}
impl NdEuclidPuzzleGeometry {
    /// Returns an empty 3D puzzle geometry.
    pub fn placeholder() -> Self {
        Self {
            space: Space::new(3),
            mesh: Mesh::new_empty(3),
            piece_polytopes: PerPiece::new(),
            sticker_planes: PerSticker::new(),
            axis_vectors: PerAxis::new(),
            twist_transforms: PerTwist::new(),
            gizmo_twists: PerGizmoFace::new(),
        }
    }

    /// Returns the number of dimensions of the space the puzzle inhabits.
    pub fn ndim(&self) -> u8 {
        self.mesh.ndim
    }
}

/// Puzzle render data for an N-dimensional Euclidean puzzle.
pub struct NdEuclidPuzzleStateRenderData {
    /// Transform for each piece.
    pub piece_transforms: PerPiece<pga::Motor>,
}
impl PuzzleStateRenderData for NdEuclidPuzzleStateRenderData {}

/// Animation for an N-dimensional Euclidean puzzle.
#[derive(Debug, Clone)]
pub struct NdEuclidPuzzleAnimation {
    /// Set of pieces affected by the animation.
    pub pieces: PieceMask,
    /// Initial transform of the pieces (identity, unless the move was inputted
    /// using a mouse drag).
    pub initial_transform: pga::Motor,
    /// Final transform for the pieces.
    pub final_transform: pga::Motor,
}
impl PuzzleAnimation for NdEuclidPuzzleAnimation {
    fn dyn_clone(&self) -> BoxDynPuzzleAnimation
    where
        Self: Sized,
    {
        self.clone().into()
    }
}

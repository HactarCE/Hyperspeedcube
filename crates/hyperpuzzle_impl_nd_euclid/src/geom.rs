use std::sync::Arc;

use hypermath::{Float, Hyperplane, Point, Vector, VectorRef, pga};
use hyperpuzzle_core::{
    notation::{InvertError, Transform},
    prelude::*,
};

use crate::PuzzleLayerDepths;

/// Geometry for an N-dimensional Euclidean puzzle.
#[derive(Debug)]
pub struct NdEuclidPuzzleGeometry {
    /// Flattened vertex coordinates.
    pub vertex_coordinates: Vec<Float>,
    /// Vertex set for each piece, as an index into `vertex_coordinates` (after
    /// dividing by number of dimensions).
    ///
    /// This is used to compute whether a move is allowed.
    pub piece_vertex_sets: PerPiece<tinyset::Set64<usize>>,
    /// Centroid for each piece.
    ///
    /// This point is not guaranteed to actually be in the center of the piece.
    /// The algorithm that generates it may be change in future versions.
    ///
    /// This is used for recentering the camera.
    pub piece_centroids: PerPiece<Point>,

    /// Facet hyperplanes.
    pub planes: Vec<Hyperplane>,
    /// Hyperplane for each sticker, as an index into `hyperplanes`.
    ///
    /// This is used to compute whether the puzzle is solved.
    pub sticker_planes: PerSticker<usize>,

    /// Puzzle mesh for rendering.
    pub mesh: Mesh,

    /// Vector for each axis.
    ///
    /// The axis vector is perpendicular to all layer boundaries on the axis and
    /// is fixed by all turns on the axis.
    ///
    /// This vector is **not** necessarily unit.
    pub axis_vectors: Arc<PerAxis<Vector>>,
    /// Top and bottom depths for each layer on each axis.
    pub axis_layer_depths: PerAxis<PuzzleLayerDepths>,
    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: Arc<PerTwist<pga::Motor>>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<GizmoTwist>,
}

impl NdEuclidPuzzleGeometry {
    /// Returns an empty 3D puzzle geometry.
    pub fn placeholder() -> Self {
        Self {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new(),
            piece_centroids: PerPiece::new(),

            planes: vec![],
            sticker_planes: PerSticker::new(),

            mesh: Mesh::new_empty(3),
            axis_vectors: Arc::new(PerAxis::new()),
            axis_layer_depths: PerAxis::new(),
            twist_transforms: Arc::new(PerTwist::new()),
            gizmo_twists: PerGizmoFace::new(),
        }
    }

    /// Returns the number of dimensions of the space the puzzle inhabits.
    pub fn ndim(&self) -> u8 {
        self.mesh.ndim
    }

    /// Returns the hyperplane for a sticker.
    pub fn sticker_plane(&self, sticker: Sticker) -> &Hyperplane {
        &self.planes[self.sticker_planes[sticker]]
    }

    /// Returns the `i`th vertex in [`Self::vertex_coordinates`].
    fn vertex(&self, i: usize) -> impl VectorRef {
        let ndim = self.ndim() as usize;
        &self.vertex_coordinates[i * ndim..(i + 1) * ndim]
    }

    /// Returns the minimum and maximum coordinate of a piece on an axis.
    ///
    /// Returns `None` if the piece has no vertices.
    pub fn piece_min_max_on_axis(
        &self,
        piece: Piece,
        axis_vector: impl VectorRef,
    ) -> Option<(Float, Float)> {
        let normalized_axis_vector = axis_vector.normalize()?;
        let vertex_coordinates = self.piece_vertex_sets[piece].iter().map(|i| self.vertex(i));
        let vertex_distances_along_axis =
            vertex_coordinates.map(|vertex| normalized_axis_vector.dot(vertex));
        hypermath::util::min_max(vertex_distances_along_axis)
    }
}

/// Clockwise twist on a twist gizmo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GizmoTwist {
    /// Axis that the gizmo belongs to.
    ///
    /// This can be derived from `transform` but is convenient to access it
    /// directly.
    pub axis: Axis,
    /// Transform of the clockwise twist of the gizmo.
    pub transform: Transform,
    /// Multiplier for the clockwise twist of the gizmo.
    ///
    /// This is almost always `Multiplier(1)`.
    pub multiplier: Multiplier,
}

impl GizmoTwist {
    /// Constructs a move for the gizmo.
    pub fn to_move(
        &self,
        layers: impl Into<LayerPrefix>,
        direction: hypermath::Sign,
    ) -> Result<Move, InvertError> {
        Ok(Move {
            layers: layers.into(),
            transform: self.transform.clone(),
            multiplier: match direction {
                hypermath::Sign::Pos => self.multiplier,
                hypermath::Sign::Neg => self.multiplier.inv()?,
            },
        })
    }
}

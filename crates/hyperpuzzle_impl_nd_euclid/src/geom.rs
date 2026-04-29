use std::sync::Arc;

use hypermath::{Float, Hyperplane, Point, VectorRef};
use hyperpuzzle_core::Component;
use hyperpuzzle_core::prelude::*;

use crate::PuzzleLayerDepths;
use crate::components::NdEuclidAxisVectors;

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
    /// This vector is **not** necessarily a unit vector.
    pub axis_vectors: Arc<NdEuclidAxisVectors>,
    /// Top and bottom depths for each layer on each axis.
    pub axis_layer_depths: PerAxis<PuzzleLayerDepths>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Move>,
}

impl Component<Puzzle> for NdEuclidPuzzleGeometry {}

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
            axis_vectors: Arc::new(NdEuclidAxisVectors::new(3)),
            axis_layer_depths: PerAxis::new(),
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

use std::sync::Arc;

use hypermath::{Float, Hyperplane, Vector, VectorRef, pga};
use hyperpuzzle_core::prelude::*;

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

    /// Face hyperplanes.
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
    /// Transforation to apply to pieces for each twist.
    pub twist_transforms: Arc<PerTwist<pga::Motor>>,

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Twist>,
}

impl NdEuclidPuzzleGeometry {
    /// Returns an empty 3D puzzle geometry.
    pub fn placeholder() -> Self {
        Self {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new(),

            planes: vec![],
            sticker_planes: PerSticker::new(),

            mesh: Mesh::new_empty(3),
            axis_vectors: Arc::new(PerAxis::new()),
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

use std::sync::Arc;

use hypermath::Vector;
use hyperpuzzle_core::Axis;

use crate::names::NameBiMap;

/// Set of axes with a common lowercase Latin prefix.
///
/// This type is reference-counted and thus relatively cheap to clone.
#[derive(Debug)]
pub(super) struct AxisSet {
    /// Number of dimensions of the space containing the puzzle.
    pub ndim: u8,
    /// Number of axes in the set.
    pub len: usize,
    /// ID offset of the axes in the set.
    ///
    /// IDs within an set always count starting from 0, but the puzzle may have
    /// multiple sets and so puzzle-facing IDs for axes in this set must start
    /// counting from this offset.
    pub id_offset: usize,
    /// Axis names.
    pub names: Arc<NameBiMap<Axis>>,
    /// Axis orbits.
    pub orbits: Vec<AxisOrbit>,
}

impl AxisSet {
    /// Lifts the axis orbit into a higher dimension.
    ///
    /// - All axis vectors are lifted into a higher dimension.
    pub fn lift_ndim(&self, ndim_below: u8, ndim_above: u8) -> Self {
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            len: self.len,
            id_offset: self.id_offset,
            names: Arc::clone(&self.names),
            orbits: self
                .orbits
                .iter()
                .map(|axis_orbit| AxisOrbit {
                    len: axis_orbit.len,
                    vector: crate::lift_vector_by_ndim(
                        &axis_orbit.vector,
                        ndim_below,
                        self.ndim,
                        ndim_above,
                    ),
                    max_layer: axis_orbit.max_layer,
                    generator_sequences: Arc::clone(&axis_orbit.generator_sequences),
                })
                .collect(),
        }
    }

    /// Offsets all axis IDs by an additional amount.
    #[must_use]
    pub fn offset_ids_by(mut self, additional_offset: usize) -> Self {
        self.id_offset += additional_offset;
        self
    }
}

#[derive(Debug)]
pub struct AxisOrbit {
    /// Number of axes in the orbit.
    pub len: usize,
    /// Vector for the first axis.
    ///
    /// This vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 4D. For a facet-turning puzzle, each axis
    /// vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub vector: Vector,
    /// Number of layers on each axis, which is equivalent to the maximum layer
    /// number.
    pub max_layer: u16,
    /// Generator sequence for each axis in the orbit.
    pub generator_sequences: Arc<Vec<hypergroup::AbbrGenSeq>>,
}

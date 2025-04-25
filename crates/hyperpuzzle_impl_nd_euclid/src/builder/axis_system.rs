use std::collections::hash_map;
use std::sync::Arc;

use eyre::{OptionExt, Result, eyre};
use hypermath::collections::{ApproxHashMap, IndexOutOfRange};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use smallvec::{SmallVec, smallvec};

use crate::NdEuclidTwistSystemEngineData;

/// Twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisBuilder {
    /// The axis's vector, which all layers must be perpendicular to and which
    /// all twists must keep fixed.
    ///
    /// Once an axis has been constructed, its vector cannot be modified.
    vector: Vector,
}
impl AxisBuilder {
    /// Returns the axis's vector.
    pub fn vector(&self) -> &Vector {
        &self.vector
    }

    /// Returns a union-of-intersections of bounded regions for the given layer
    /// mask.
    pub fn plane_bounded_regions(
        &self,
        layers: &AxisLayersInfo,
        layer_mask: LayerMask,
    ) -> Result<Vec<SmallVec<[Hyperplane; 2]>>> {
        // TODO: optimize
        layer_mask
            .iter()
            .map(|layer| self.boundary_of_layer(layers, layer))
            .collect()
    }

    /// Returns the hyperplanes bounding a layer.
    pub fn boundary_of_layer(
        &self,
        layers: &AxisLayersInfo,
        layer: Layer,
    ) -> Result<SmallVec<[Hyperplane; 2]>> {
        let l = layers.0.get(layer)?;
        let mut ret = smallvec![];
        if l.top.is_finite() {
            ret.push(Hyperplane::new(&self.vector, l.top).ok_or_eyre("bad axis vector")?);
        }
        if l.bottom.is_finite() {
            ret.push(
                Hyperplane::new(&self.vector, l.bottom)
                    .ok_or_eyre("bad axis vector")?
                    .flip(),
            );
        }
        Ok(ret)
    }
}

/// Axis system during puzzle construction.
#[derive(Debug)]
pub struct AxisSystemBuilder {
    /// Number of dimensions of the space.
    pub ndim: u8,

    /// Axis data (not including name and ordering).
    by_id: PerAxis<AxisBuilder>,
    /// Map from vector to axis ID.
    vector_to_id: ApproxHashMap<Vector, Axis>,
    /// Axis names.
    pub names: NameSpecBiMapBuilder<Axis>,

    /// Orbits used to generate axes, tracked for puzzle dev purposes.
    pub orbits: Vec<Orbit<Axis>>,
}
impl AxisSystemBuilder {
    /// Constructs a new empty axis system builder.
    pub fn new(ndim: u8) -> Self {
        Self {
            ndim,
            by_id: PerAxis::new(),
            vector_to_id: ApproxHashMap::new(),
            names: NameSpecBiMapBuilder::new(),
            orbits: vec![],
        }
    }

    /// Returns whether there are no axes in the axis system.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Returns the number of axes in the axis system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Adds a new axis.
    pub fn add(&mut self, vector: Vector) -> Result<Axis> {
        let vector = vector
            .normalize()
            .ok_or_eyre("axis vector cannot be zero")?;

        // Check that the vector isn't already taken.
        match self.vector_to_id.entry(vector.clone()) {
            hash_map::Entry::Occupied(_) => Err(eyre!("axis vector is already taken")),
            hash_map::Entry::Vacant(e) => {
                let id = self.by_id.push(AxisBuilder { vector })?;
                e.insert(id);
                Ok(id)
            }
        }
    }

    /// Returns a reference to a axis by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Axis) -> Result<&AxisBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }
    /// Returns a mutable reference to a axis by ID, or an error if the ID is
    /// out of range.
    pub fn get_mut(&mut self, id: Axis) -> Result<&mut AxisBuilder, IndexOutOfRange> {
        self.by_id.get_mut(id)
    }

    /// Returns an axis ID from its vector.
    pub fn vector_to_id(&self, vector: impl VectorRef) -> Option<Axis> {
        Some(*self.vector_to_id.get(&vector.normalize()?)?)
    }

    /// Returns an iterator over all the axes, in the canonical ordering.
    pub fn iter(&self) -> impl Iterator<Item = (Axis, &AxisBuilder)> {
        self.by_id.iter()
    }

    /// Validates and constructs an axis system.
    pub(super) fn build(&self) -> Result<AxisSystemBuildOutput> {
        let mut names = self.names.clone();
        let autonames = hyperpuzzle_core::util::iter_uppercase_letter_names();
        names.autoname(self.len(), autonames)?;
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis names")?);

        let orbits = self.orbits.clone();

        let axis_vectors = self.by_id.map_ref(|_, axis| axis.vector.clone());

        Ok(AxisSystemBuildOutput {
            axes: AxisSystem { names, orbits },
            axis_vectors,
            axis_from_vector: self.vector_to_id.clone(),
        })
    }

    /// "Unbuilds" an axis system into an axis system builder.
    pub fn unbuild(
        axis_system: &AxisSystem,
        engine_data: &NdEuclidTwistSystemEngineData,
    ) -> Result<Self> {
        let AxisSystem { names, orbits } = axis_system;

        let vector_to_id = (*engine_data.axis_from_vector).clone();

        Ok(AxisSystemBuilder {
            ndim: engine_data.ndim,
            by_id: PerAxis::new_with_len(axis_system.len()).map(|id, ()| {
                let vector = engine_data.axis_vectors[id].clone();
                AxisBuilder { vector }
            }),
            vector_to_id,
            names: (**names).clone().into(),
            orbits: orbits.clone(),
        })
    }
}

pub(super) struct AxisSystemBuildOutput {
    pub axes: AxisSystem,
    pub axis_vectors: PerAxis<Vector>,
    pub axis_from_vector: ApproxHashMap<Vector, Axis>,
}

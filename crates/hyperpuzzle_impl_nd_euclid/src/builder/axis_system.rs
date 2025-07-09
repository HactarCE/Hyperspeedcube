use std::collections::hash_map;
use std::sync::Arc;

use eyre::{OptionExt, Result, bail};
use hypermath::collections::{ApproxHashMap, IndexOutOfRange};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;

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
    autonames: AutoNames,

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
            autonames: AutoNames::default(),
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
    pub fn add(
        &mut self,
        vector: Vector,
        name_spec: Option<String>,
        warn_fn: impl FnOnce(BadName),
    ) -> Result<Axis> {
        let vector = vector
            .normalize()
            .ok_or_eyre("axis vector cannot be zero")?;

        // Check that the vector isn't already taken.
        let id = match self.vector_to_id.entry(vector.clone()) {
            hash_map::Entry::Occupied(_) => bail!("axis vector is already taken"),
            hash_map::Entry::Vacant(e) => {
                let id = self.by_id.push(AxisBuilder { vector })?;
                e.insert(id);
                id
            }
        };

        self.names
            .set_with_fallback(id, name_spec, &mut self.autonames, warn_fn)?;

        Ok(id)
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
        let names = self.names.clone();
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis names")?);

        let orbits = self.orbits.clone();

        let axis_vectors = self.by_id.map_ref(|_, axis| axis.vector.clone());

        Ok(AxisSystemBuildOutput {
            axes: AxisSystem { names, orbits },
            axis_vectors,
            axis_from_vector: self.vector_to_id.clone(),
        })
    }

    /// "Unbuilds" an axis system.
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
            autonames: AutoNames::default(),
            orbits: orbits.clone(),
        })
    }
}

pub(super) struct AxisSystemBuildOutput {
    pub axes: AxisSystem,
    pub axis_vectors: PerAxis<Vector>,
    pub axis_from_vector: ApproxHashMap<Vector, Axis>,
}

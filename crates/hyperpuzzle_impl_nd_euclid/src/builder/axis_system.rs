use std::sync::Arc;

use eyre::{OptionExt, Result};
use hypermath::prelude::*;
use hyperpuzzle_core::{ComponentList, prelude::*};

use crate::components::NdEuclidAxisVectors;

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
pub struct AdHocAxisSystemBuilder {
    /// Axis vectors.
    pub vectors: NdEuclidAxisVectors,
    /// Axis names.
    pub names: NameSpecBiMapBuilder<Axis>,
    autonames: AutoNames,
    /// Orbits used to generate axes, tracked for puzzle dev purposes.
    pub orbits: Vec<Orbit<Axis>>,
}
impl AdHocAxisSystemBuilder {
    /// Constructs a new empty axis system builder.
    pub fn new(ndim: u8) -> Self {
        Self {
            vectors: NdEuclidAxisVectors::new(ndim),
            names: NameSpecBiMapBuilder::new(),
            autonames: AutoNames::default(),
            orbits: vec![],
        }
    }

    /// Returns whether there are no axes in the axis system.
    fn is_empty(&self) -> bool {
        self.vectors.vectors_by_id.is_empty()
    }
    /// Returns the number of axes in the axis system.
    pub fn len(&self) -> usize {
        self.vectors.vectors_by_id.len()
    }

    pub fn ndim(&self) -> u8 {
        self.vectors.ndim
    }

    /// Adds a new axis.
    pub fn add(
        &mut self,
        vector: Vector,
        name_spec: Option<String>,
        warn_fn: impl FnOnce(BadName),
    ) -> Result<Axis> {
        let id = self.vectors.add_axis(vector)?;
        self.names
            .set_with_fallback(id, name_spec, &mut self.autonames, warn_fn)?;
        Ok(id)
    }

    /// Validates and constructs an axis system.
    pub(super) fn build(&self) -> Result<AxisSystem> {
        let names = self.names.clone();
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis names")?);

        let orbits = self.orbits.clone();

        let mut components = ComponentList::new();
        components.insert(Arc::new(self.vectors.clone()));

        Ok(AxisSystem {
            names,
            orbits,
            components,
        })
    }
}

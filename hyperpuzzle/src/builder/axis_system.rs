use std::collections::hash_map;

use eyre::{bail, eyre, OptionExt, Result};
use hypermath::collections::{ApproxHashMap, IndexOutOfRange};
use hypermath::prelude::*;

use super::{CustomOrdering, NamingScheme};
use crate::{Axis, LayerInfo, PerAxis, PerLayer};

/// Layer of a twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisLayerBuilder {
    /// Hyperplane bounding the bottom of the layer.
    pub bottom: Hyperplane,
    /// Hyperplane bounding the top of the layer, which is inferred to be the
    /// bottom of the next layer out (or unbounded, this is the outermost
    /// layer).
    pub top: Option<Hyperplane>,
}

/// Twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisBuilder {
    /// The axis's vector, which all layers must be perpendicular to and which
    /// all twists must keep fixed.
    ///
    /// Once an axis has been constructed, its vector cannot be modified.
    vector: Vector,
    /// Layer data for each layer on the axis.
    pub layers: PerLayer<AxisLayerBuilder>,
}
impl AxisBuilder {
    /// Returns the axis's vector.
    pub fn vector(&self) -> &Vector {
        &self.vector
    }

    fn ensure_monotonic_layers(&self) -> Result<()> {
        let mut layer_planes = vec![];
        for layer in self.layers.iter_values() {
            layer_planes.extend(layer.top.clone());
            layer_planes.push(layer.bottom.flip());
        }
        for (a, b) in layer_planes.iter().zip(layer_planes.iter().skip(1)) {
            // We expect `a` is above `b`.
            if a == b {
                continue;
            }
            if !approx_eq(&a.normal().dot(b.normal()).abs(), &1.0) {
                bail!("axis layers are not parallel")
            }
            let is_b_below_a = a.location_of_point(b.pole()) == PointWhichSide::Inside;
            let is_a_above_b = b.location_of_point(a.pole()) == PointWhichSide::Outside;
            if !is_b_below_a || !is_a_above_b {
                bail!("axis layers are not monotonic");
            }
        }
        Ok(())
    }

    pub(super) fn build_layers(&self) -> Result<PerLayer<LayerInfo>> {
        // Check that the layer planes are monotonic.
        self.ensure_monotonic_layers()?;

        // Bound the top of each layer at the bottom of the previous one.
        let mut last_bottom = None;
        Ok(self.layers.map_ref(|_, layer| LayerInfo {
            bottom: layer.bottom.clone(),
            top: layer.top.clone().or(std::mem::replace(
                &mut last_bottom,
                Some(layer.bottom.flip()),
            )),
        }))
    }
}

/// Axis system during puzzle construction.
#[derive(Debug)]
pub struct AxisSystemBuilder {
    /// Axis data (not including name and ordering).
    by_id: PerAxis<AxisBuilder>,
    /// Map from vector to axis ID.
    vector_to_id: ApproxHashMap<Vector, Axis>,
    /// User-specified axis names.
    pub names: NamingScheme<Axis>,
    /// User-specified ordering of axiss.
    pub ordering: CustomOrdering<Axis>,
}
impl AxisSystemBuilder {
    /// Constructs a new empty axis system builder.
    pub fn new() -> Self {
        Self {
            by_id: PerAxis::new(),
            vector_to_id: ApproxHashMap::new(),
            names: NamingScheme::new(),
            ordering: CustomOrdering::default(),
        }
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
                let layers = PerLayer::new();
                let id = self.by_id.push(AxisBuilder { vector, layers })?;
                self.ordering.add(id)?;
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
        self.ordering
            .ids_in_order()
            .iter()
            .map(|&id| (id, &self.by_id[id]))
    }
}

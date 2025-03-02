use std::collections::hash_map;

use eyre::{OptionExt, Result, bail, eyre};
use hypermath::collections::{ApproxHashMap, IndexOutOfRange};
use hypermath::prelude::*;
use hyperpuzzle_core::{Axis, DevOrbit, Layer, LayerInfo, LayerMask, PerAxis, PerLayer};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use super::{CustomOrdering, NamingScheme};

/// Layer of a twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisLayerBuilder {
    /// Position along the axis vector from the origin that bounds the bottom of
    /// the layer. **This may be infinite.**
    pub bottom: Float,
    /// Position along the axis vector from the origin that bounds the top of
    /// the layer. **This may be infinite.**
    pub top: Float,
}

/// Twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisBuilder {
    /// The axis's vector, which all layers must be perpendicular to and which
    /// all twists must keep fixed.
    ///
    /// Once an axis has been constructed, its vector cannot be modified.
    vector: Vector,
    /// Layer data for each layer on the axis, in order from outermost to
    /// innermost.
    pub layers: PerLayer<AxisLayerBuilder>,
}
impl AxisBuilder {
    /// Returns the axis's vector.
    pub fn vector(&self) -> &Vector {
        &self.vector
    }

    /// Returns the hyperplanes bounding a layer.
    pub fn boundary_of_layer(&self, layer: Layer) -> Result<SmallVec<[Hyperplane; 2]>> {
        let l = self.layers.get(layer)?;
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

    /// Returns a union-of-intersections of bounded regions for the given layer
    /// mask.
    pub fn plane_bounded_regions(
        &self,
        layer_mask: LayerMask,
    ) -> Result<Vec<SmallVec<[Hyperplane; 2]>>> {
        // TODO: optimize
        layer_mask
            .iter()
            .map(|layer| self.boundary_of_layer(layer))
            .collect()
    }

    fn ensure_monotonic_layers(&self) -> Result<()> {
        let mut last_depth = Float::INFINITY;
        for layer_info in self.layers.iter_values() {
            let AxisLayerBuilder { bottom, top } = layer_info;
            if !(approx_gt_eq(&last_depth, top) && approx_gt(top, bottom)) {
                let depths = self
                    .layers
                    .iter_values()
                    .map(|l| (l.top, l.bottom))
                    .collect_vec();
                bail!("axis layers {depths:?} are not sorted from outermost to innermost");
            }
            last_depth = *bottom;
        }
        Ok(())
    }

    pub(super) fn build_layers(&self) -> Result<PerLayer<LayerInfo>> {
        // Check that the layer planes are monotonic.
        self.ensure_monotonic_layers()?;

        Ok(self
            .layers
            .map_ref(|_, &AxisLayerBuilder { bottom, top }| LayerInfo { bottom, top }))
    }
}

/// Axis system during puzzle construction.
#[derive(Debug, Default)]
pub struct AxisSystemBuilder {
    /// Axis data (not including name and ordering).
    by_id: PerAxis<AxisBuilder>,
    /// Map from vector to axis ID.
    vector_to_id: ApproxHashMap<Vector, Axis>,
    /// User-specified axis names.
    pub names: NamingScheme<Axis>,
    /// User-specified ordering of axiss.
    pub ordering: CustomOrdering<Axis>,

    /// Orbits used to generate axis, tracked for puzzle dev purposes.
    pub axis_orbits: Vec<DevOrbit<Axis>>,
}
impl AxisSystemBuilder {
    /// Constructs a new empty axis system builder.
    pub fn new() -> Self {
        Self::default()
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

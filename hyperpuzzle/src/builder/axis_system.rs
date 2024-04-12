use eyre::{eyre, Result};
use std::collections::hash_map;
use std::sync::{Arc, Weak};

use hypermath::collections::generic_vec::IndexOutOfRange;
use hypermath::collections::ApproxHashMap;
use hypermath::prelude::*;
use hypershape::prelude::*;
use parking_lot::Mutex;

use super::{CustomOrdering, NamingScheme};
use crate::{Axis, PerAxis};

/// Layer of a twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisLayerBuilder {
    pub boundary: ManifoldSet,
}
impl AxisLayerBuilder {
    /// Returns a deep copy of the axis layer.
    fn clone(&self, space_map: &mut SpaceMap<'_>) -> Self {
        Self {
            boundary: space_map.map_set(&self.boundary),
        }
    }
}

/// Twist axis during puzzle construction.
#[derive(Debug, Clone)]
pub struct AxisBuilder {
    vector: Vector,
    pub layers: Vec<AxisLayerBuilder>,
}
impl AxisBuilder {
    /// Returns the axis's vector.
    pub fn vector(&self) -> &Vector {
        &self.vector
    }

    /// Returns a deep copy of axis.
    fn clone(&self, space_map: &mut SpaceMap<'_>) -> Self {
        Self {
            vector: self.vector.clone(),
            layers: self
                .layers
                .iter()
                .map(|layer| layer.clone(space_map))
                .collect(),
        }
    }
}

/// Axis system during puzzle construction.
#[derive(Debug)]
pub struct AxisSystemBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Optional ID for the whole axis system.
    pub id: Option<String>,

    /// Space where the axis system exists.
    pub space: Arc<Mutex<Space>>,

    /// Symmetry group of the axis system.
    pub symmetry: Option<SchlafliSymbol>,

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
    pub fn new(id: Option<String>, space: Arc<Mutex<Space>>) -> Arc<Mutex<Self>> {
        Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id,

                space,

                symmetry: None,

                by_id: PerAxis::new(),
                vector_to_id: ApproxHashMap::new(),
                names: NamingScheme::new(),
                ordering: CustomOrdering::default(),
            })
        })
    }

    /// Returns an `Arc` reference to the axis system builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`AxisSystemBuilder` removed from `Arc`")
    }

    /// Returns the number of axes in the axis system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Creates a deep copy in a new space.
    ///
    /// Returns an error if the new and old spaces are not compatible.
    pub fn clone(&self, space: &Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        let source = self.space.lock();
        let mut destination = space.lock();
        let mut map = SpaceMap::new(&source, &mut destination)?;

        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id: self.id.clone(),

                space: Arc::clone(&space),

                symmetry: self.symmetry.clone(),

                by_id: self.by_id.map_ref(|_id, axis| axis.clone(&mut map)),
                vector_to_id: self.vector_to_id.clone(),
                names: self.names.clone(),
                ordering: self.ordering.clone(),
            })
        }))
    }

    /// Adds a new axis.
    pub fn add(&mut self, vector: Vector) -> Result<Axis> {
        // Check that the vector isn't already taken.
        match self.vector_to_id.entry(vector.clone()) {
            hash_map::Entry::Occupied(_) => Err(eyre!("axis vector is already taken")),
            hash_map::Entry::Vacant(e) => {
                let id = self.by_id.push(AxisBuilder {
                    vector,
                    layers: vec![],
                })?;
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

    /// Returns a map from vector to axis ID.
    pub fn vector_to_id(&self) -> &ApproxHashMap<Vector, Axis> {
        &self.vector_to_id
    }

    /// Returns an iterator over all the axes, in the canonical ordering.
    pub fn iter(&self) -> impl Iterator<Item = (Axis, &AxisBuilder)> {
        self.ordering
            .ids_in_order()
            .iter()
            .map(|&id| (id, &self.by_id[id]))
    }
}

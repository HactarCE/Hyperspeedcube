use eyre::{bail, Result};
use std::sync::{Arc, Weak};

use hypermath::collections::{
    approx_hashmap::{ApproxHashMap, ApproxHashMapKey},
    generic_vec::IndexOutOfRange,
};
use hypermath::Isometry;
use hypershape::Space;
use parking_lot::Mutex;

use crate::builder::NamingScheme;
use crate::puzzle::{Axis, PerTwist, Twist};

use super::{AxisSystemBuilder, CustomOrdering};

/// Twist during puzzle construction.
#[derive(Debug, Clone)]
pub struct TwistBuilder {
    pub axis: Axis,
    pub transform: Isometry,
}
impl ApproxHashMapKey for TwistBuilder {
    type Hash = (Axis, <Isometry as ApproxHashMapKey>::Hash);

    fn approx_hash(
        &self,
        float_hash_fn: impl FnMut(
            hypermath::prelude::Float,
        ) -> hypermath::collections::approx_hashmap::FloatHash,
    ) -> Self::Hash {
        let Self { axis, transform } = self;
        (*axis, transform.approx_hash(float_hash_fn))
    }
}

/// Twist system being constructed.
#[derive(Debug)]
pub struct TwistSystemBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Optional ID for the whole twist system.
    pub id: Option<String>,

    /// Axis system.
    pub axes: Arc<Mutex<AxisSystemBuilder>>,

    /// Twist data (not including name).
    by_id: PerTwist<TwistBuilder>,
    /// Map from twist data to twist ID for each axis.
    data_to_id: ApproxHashMap<TwistBuilder, Twist>,
    /// User-specified twist names.
    pub names: NamingScheme<Twist>,
}
impl TwistSystemBuilder {
    /// Constructs a empty twist system with a given axis system.
    pub fn new(id: Option<String>, axes: Arc<Mutex<AxisSystemBuilder>>) -> Arc<Mutex<Self>> {
        Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id,

                axes,

                by_id: PerTwist::new(),
                data_to_id: ApproxHashMap::new(),
                names: NamingScheme::new(),
            })
        })
    }

    /// Returns an `Arc` reference to the twist system builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`TwistSystemBuilder` removed from `Arc`")
    }

    /// Returns the number of twists in the twist system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Creates a deep copy in a new space.
    ///
    /// Returns an error if the new and old spaces are not compatible.
    pub fn clone(&self, space: &Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        let axes = self.axes.lock();
        let new_axes = axes.clone(space)?;

        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id: self.id.clone(),

                axes: new_axes,

                by_id: self.by_id.clone(),
                data_to_id: self.data_to_id.clone(),
                names: self.names.clone(),
            })
        }))
    }

    /// Returns the twist axes in canonical alphabetical order.
    pub fn alphabetized(&self) -> Vec<Twist> {
        let mut ordering =
            CustomOrdering::with_len(self.len()).expect("error constructing default ordering");
        ordering.sort_by_name(self.names.ids_to_names());
        ordering.ids_in_order().to_vec()
    }

    /// Adds a new twist.
    pub fn add(&mut self, data: TwistBuilder) -> Result<Twist> {
        // Check that there is not already an identical twist.
        if self.data_to_id.get(&data).is_some() {
            bail!("identical twist already exists")
        }

        let id = self.by_id.push(data.clone())?;
        self.data_to_id.insert(data, id);

        Ok(id)
    }

    /// Returns a reference to a twist by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Twist) -> Result<&TwistBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }

    /// Returns a map from manifold set to color ID.
    pub fn data_to_id(&self) -> &ApproxHashMap<TwistBuilder, Twist> {
        &self.data_to_id
    }
}

use std::collections::HashMap;

use eyre::{eyre, OptionExt, Result, WrapErr};
use hypermath::collections::approx_hashmap::FloatHash;
use hypermath::collections::{ApproxHashMap, ApproxHashMapKey, IndexOutOfRange};
use hypermath::prelude::*;
use hypershape::Space;
use pga::{Blade, Motor};

use super::{AxisSystemBuilder, CustomOrdering};
use crate::builder::NamingScheme;
use crate::puzzle::{Axis, PerTwist, Twist};
use crate::{AxisInfo, PerAxis, TwistGizmoPolyhedron, TwistInfo};

/// Twist during puzzle construction.
#[derive(Debug, Clone)]
pub struct TwistBuilder {
    /// Axis that is twisted.
    pub axis: Axis,
    /// Transform to apply to pieces.
    pub transform: Motor,
    /// Value in the quarter-turn metric (or its contextual equivalent).
    pub qtm: usize,
    /// Distance of the pole for the corresponding facet in the 4D facet gizmo.
    pub gizmo_pole_distance: Option<f32>,
}
impl TwistBuilder {
    /// Canonicalizes the twist.
    #[must_use]
    pub fn canonicalize(self) -> Result<Self> {
        let transform = self
            .transform
            .canonicalize_up_to_180()
            .ok_or(BadTwist::BadTransform)?;
        Ok(Self { transform, ..self })
    }
    /// Returns the key used to hash or look up the twist.
    pub fn key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform)
    }
    /// Returns the key used to look up the reverse twist.
    pub fn rev_key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform.reverse())
    }
}

/// Unique key for a twist.
#[derive(Debug, Clone)]
pub struct TwistKey {
    /// Axis that is twisted.
    axis: Axis,
    /// Transform to apply to pieces.
    transform: Motor,
}
impl ApproxHashMapKey for TwistKey {
    type Hash = (Axis, <Motor as ApproxHashMapKey>::Hash);

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        (self.axis, self.transform.approx_hash(float_hash_fn))
    }
}
impl TwistKey {
    /// Constructs a twist key from an axis and a transform, which does not need
    /// to be canonicalized.
    pub fn new(axis: Axis, transform: &Motor) -> Result<Self, BadTwist> {
        let transform = transform
            .canonicalize_up_to_180()
            .ok_or(BadTwist::BadTransform)?;
        Ok(Self { axis, transform })
    }
}

/// Twist system being constructed.
#[derive(Debug)]
pub struct TwistSystemBuilder {
    /// Axis system being constructed.
    pub axes: AxisSystemBuilder,

    /// Twist data (not including name).
    by_id: PerTwist<TwistBuilder>,
    /// Map from twist data to twist ID for each axis.
    ///
    /// Does not include inverses.
    data_to_id: ApproxHashMap<TwistKey, Twist>,
    /// User-specified twist names.
    pub names: NamingScheme<Twist>,
}
impl TwistSystemBuilder {
    /// Constructs a empty twist system with a given axis system.
    pub fn new() -> Self {
        Self {
            axes: AxisSystemBuilder::new(),

            by_id: PerTwist::new(),
            data_to_id: ApproxHashMap::new(),
            names: NamingScheme::new(),
        }
    }

    /// Returns the number of twists in the twist system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Returns the twist axes in canonical alphabetical order.
    pub fn alphabetized(&self) -> Vec<Twist> {
        let mut ordering =
            CustomOrdering::with_len(self.len()).expect("error constructing default ordering");
        ordering.sort_by_name(self.names.ids_to_names());
        ordering.ids_in_order().to_vec()
    }

    /// Adds a new twist.
    pub fn add(&mut self, data: TwistBuilder) -> Result<Result<Twist, BadTwist>> {
        let data = data.canonicalize()?;
        let key = data.key()?;

        // Reject the identity twist.
        if data.transform.is_ident() {
            return Ok(Err(BadTwist::Identity));
        }

        // Check that there is not already an identical twist.
        if let Some(&id) = self.data_to_id.get(&key) {
            let name = self.names.get(id).unwrap_or_default();
            return Ok(Err(BadTwist::DuplicateTwist { id, name }));
        }

        let id = self.by_id.push(data)?;
        self.data_to_id.insert(key, id);

        Ok(Ok(id))
    }
    /// Adds a new twist and assigns it a name.
    ///
    /// If the twist is invalid, `warn_fn` is called with info about what went
    /// wrong and no twist is created.
    pub fn add_named(
        &mut self,
        data: TwistBuilder,
        name: String,
        mut warn_fn: impl FnMut(String),
    ) -> Result<Option<Twist>> {
        let id = match self.add(data)? {
            Ok(ok) => ok,
            Err(err) => {
                warn_fn(err.to_string());
                return Ok(None);
            }
        };
        self.names.set(id, Some(name), |e| warn_fn(e.to_string()));
        Ok(Some(id))
    }

    /// Returns a reference to a twist by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Twist) -> Result<&TwistBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }

    /// Returns a twist ID from its axis and transform.
    pub fn data_to_id(&self, key: &TwistKey) -> Option<Twist> {
        None.or_else(|| self.data_to_id.get(&key))
            .or_else(|| self.data_to_id.get(&key))
            .copied()
    }

    /// Returns the inverse of a twist, or an error if the ID is out of range.
    pub fn inverse(&self, id: Twist) -> Result<Option<Twist>> {
        Ok(self.data_to_id(&self.get(id)?.rev_key()?))
    }

    /// Finalizes the axis system and twist system, and validates them to check
    /// for errors in the definition.
    pub fn build(
        &self,
        space: &Space,
        mut warn_fn: impl FnMut(eyre::Report),
    ) -> Result<(
        PerAxis<AxisInfo>,
        PerTwist<TwistInfo>,
        Vec<TwistGizmoPolyhedron>,
    )> {
        // Assemble list of axes.
        let mut axes = PerAxis::new();
        let mut axis_map = HashMap::new();
        for (old_id, name) in super::iter_autonamed(
            &self.axes.names,
            &self.axes.ordering,
            crate::util::iter_uppercase_letter_names(),
        ) {
            let old_axis = self.axes.get(old_id)?;
            let vector = old_axis.vector().clone();
            let layers = old_axis
                .build_layers()
                .wrap_err_with(|| format!("building axis {name:?}"))?;
            let new_id = axes.push(AxisInfo {
                name,
                vector,
                layers,
            })?;

            axis_map.insert(old_id, new_id);
        }

        // Assemble list of twists.
        let mut twists = PerTwist::new();
        let mut twist_id_map = HashMap::new();
        let mut gizmo_poles_per_axis: PerAxis<Vec<(Vector, Twist)>> = axes.map_ref(|_, _| vec![]);
        for old_id in self.alphabetized() {
            let old_twist = self.get(old_id)?;
            let axis = *axis_map.get(&old_twist.axis).ok_or_eyre("bad axis ID")?;
            let new_id = twists.push(TwistInfo {
                name: match self.names.get(old_id) {
                    Some(s) => s.clone(),
                    None => (old_id.0 + 1).to_string(), // 1-indexed
                },
                qtm: old_twist.qtm,
                axis,
                transform: old_twist.transform.clone(),
                opposite: None,    // will be assigned later
                reverse: Twist(0), // will be assigned later
            })?;
            twist_id_map.insert(old_id, new_id);

            if let Some(pole_distance) = old_twist.gizmo_pole_distance {
                // We already know one vector that's fixed by the twist.
                let axis_vector = Blade::from_vector(4, &axes[axis].vector);
                // Compute the other one.
                let origin = Blade::origin(4);
                let face_normal = (|| {
                    Blade::wedge(
                        &old_twist.transform.grade_project(2),
                        &Blade::wedge(&axis_vector, &origin)?,
                    )?
                    .antidual()
                    .to_vector()?
                    .normalize()
                })()
                .ok_or_eyre("error computing normal vector for twist gizmo")?;

                gizmo_poles_per_axis[axis].push((face_normal * pole_distance as _, new_id));
            };

            // TODO: check that transform keeps layer manifolds fixed
        }
        // TODO: assign opposite twists.

        // Generate twist gizmo polyhedra.
        let mut gizmos = vec![];
        for (axis, gizmo_poles) in gizmo_poles_per_axis {
            if !gizmo_poles.is_empty() {
                gizmos.push(TwistGizmoPolyhedron::new(
                    space,
                    &axes[axis].vector,
                    gizmo_poles,
                    &axes[axis].name,
                    |twist| &twists[twist].name,
                    &mut warn_fn,
                )?);
            }
        }

        // Assign reverse twists.
        let mut twists_without_reverse = vec![];
        for (id, twist) in &mut twists {
            match self
                .data_to_id(&TwistKey::new(twist.axis, &twist.transform.reverse())?)
                .and_then(|old_id| twist_id_map.get(&old_id))
            {
                Some(&reverse_twist) => twist.reverse = reverse_twist,
                None => twists_without_reverse.push(id),
            }
        }
        if let Some(&id) = twists_without_reverse.first() {
            let name = &twists.get(id)?.name;
            warn_fn(eyre!(
                "some twists (such as {name:?}) have no reverse twist; \
                 one was autogenerated for it, but you should include \
                 one in the puzzle definition"
            ));
        }
        for id in twists_without_reverse {
            let new_twist_id = twists.next_idx()?;
            let twist = twists.get_mut(id)?;
            twist.reverse = new_twist_id;
            let new_twist_info = TwistInfo {
                name: format!("<reverse of {:?}>", twist.name),
                qtm: twist.qtm,
                axis: twist.axis,
                transform: twist.transform.reverse(),
                opposite: None,
                reverse: id,
            };
            twists.push(new_twist_info)?;
        }

        Ok((axes, twists, gizmos))
    }
}

/// Error indicating a bad twist.
#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum BadTwist {
    #[error("twist transform cannot be identity")]
    Identity,
    #[error("identical twist already exists with ID {id} and name {name:?}")]
    DuplicateTwist { id: Twist, name: String },
    #[error("bad twist transform")]
    BadTransform,
}

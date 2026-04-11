use std::sync::Arc;

use eyre::{OptionExt, Result, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::DEFAULT_VANTAGE_GROUP_NAME;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_core::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use smallvec::SmallVec;

use super::{AxisSystemBuildOutput, AxisSystemBuilder, VantageGroupBuilder, VantageSetBuilder};
use crate::{NdEuclidTwistSystemEngineData, NdEuclidVantageGroup, TwistKey};

/// Twist during puzzle construction.
#[derive(Debug, Clone)]
pub struct TwistBuilder {
    /// Axis that is twisted.
    pub axis: Axis,
    /// Transform to apply to pieces.
    pub transform: pga::Motor,
    /// Distance of the pole for the corresponding facet in the 4D facet gizmo.
    pub gizmo_pole_distance: Option<f32>,
    /// Maximum possible multiplier for use in scrambles.
    pub scramble_max_multiplier: Option<Multiplier>,
}
impl TwistBuilder {
    /// Canonicalizes the twist.
    pub fn canonicalize(self) -> Result<Self> {
        let transform = self
            .transform
            .canonicalize_up_to_180()
            .ok_or(BadTwist::BadTransform)?;
        Ok(Self { transform, ..self })
    }
    /// Returns the key used to hash or look up the twist.
    pub fn key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform).ok_or(BadTwist::BadTransform)
    }
    /// Returns the key used to look up the reverse twist.
    pub fn rev_key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform.reverse()).ok_or(BadTwist::BadTransform)
    }
}

/// Twist system being constructed.
#[derive(Debug)]
pub struct TwistSystemBuilder {
    /// Twist system ID.
    pub id: CatalogId,
    /// Name of the twist system.
    pub name: Option<String>,

    /// Axis system being constructed.
    pub axes: AxisSystemBuilder,

    /// Twist data.
    by_id: PerTwist<TwistBuilder>,
    /// Twist names.
    pub names: NameSpecBiMapBuilder<Twist>,
    /// Map from twist data to twist ID for each axis.
    ///
    /// Does not include inverses.
    data_to_id: ApproxHashMap<TwistKey, Twist>,
    autonames: AutoNames,

    /// Vantage groups.
    pub vantage_groups: IndexMap<String, VantageGroupBuilder>,
    /// Vantage sets.
    pub vantage_sets: Vec<VantageSetBuilder>,
    /// Global twist directions.
    pub directions: IndexMap<String, PerAxis<Option<SmallVec<[Twist; 4]>>>>,

    /// Whether the twist system has been modified.
    pub is_modified: bool,
    /// Whether the twist system is shared (as opposed to ad-hoc defined for a
    /// single puzzle).
    pub is_shared: bool,

    /// Exports from the Hyperpuzzlescript `build` function.
    pub hps_exports: Arc<hyperpuzzlescript::Map>,
}
impl TwistSystemBuilder {
    /// Constructs a new shared twist system.
    pub fn new_shared(id: CatalogId, name: Option<String>, ndim: u8) -> Self {
        Self::new(id, name, ndim, true)
    }

    /// Constructs a new empty ad-hoc color system.
    pub fn new_ad_hoc(puzzle_id: &CatalogId, ndim: u8) -> Self {
        Self::new(crate::ad_hoc_id(puzzle_id.clone()), None, ndim, false)
    }

    /// Constructs a empty twist system with a given axis system.
    fn new(id: CatalogId, name: Option<String>, ndim: u8, is_shared: bool) -> Self {
        Self {
            id,
            name,
            axes: AxisSystemBuilder::new(ndim),
            by_id: PerTwist::new(),
            names: NameSpecBiMapBuilder::new(),
            data_to_id: ApproxHashMap::new(APPROX),
            autonames: AutoNames::default(),
            vantage_groups: IndexMap::new(),
            vantage_sets: vec![],
            directions: IndexMap::new(),
            is_modified: false,
            is_shared,
            hps_exports: Arc::new(hyperpuzzlescript::Map::new()),
        }
    }

    /// Returns whether there are no twists in the twist system.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Returns the number of twists in the twist system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Adds a new twist.
    ///
    /// If the twist is invalid, `warn_fn` is called with info about what went
    /// wrong and no twist is created.
    pub fn add(
        &mut self,
        data: TwistBuilder,
        name_spec: Option<String>,
        mut warn_fn: impl FnMut(String),
    ) -> Result<Option<Twist>> {
        self.add_internal(data, name_spec, |e| warn_fn(e.to_string()))
            .map(|inner_result| match inner_result {
                Ok(id) => Some(id),
                Err(e) => {
                    warn_fn(e.to_string());
                    None
                }
            })
    }

    fn add_internal(
        &mut self,
        data: TwistBuilder,
        name_spec: Option<String>,
        warn_fn: impl FnOnce(BadName),
    ) -> Result<Result<Twist, BadTwist>> {
        self.is_modified = true;

        let data = data.canonicalize()?;
        let key = data.key()?;

        // Reject the identity twist.
        if data.transform.is_ident() {
            return Ok(Err(BadTwist::Identity));
        }

        // Check that there is not already an identical twist.
        if let Some(&id) = self.data_to_id.get(key.clone()) {
            let name = match self.names.get(id) {
                Some(existing_twist_name) => existing_twist_name.preferred.clone(),
                None => "?".to_owned(),
            };
            return Ok(Err(BadTwist::DuplicateTwist { id, name }));
        }

        let id = self.by_id.push(data)?;
        self.data_to_id.insert(key, id);

        self.names
            .set_with_fallback(id, name_spec, &mut self.autonames, warn_fn)?;

        Ok(Ok(id))
    }

    /// Returns a reference to a twist by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Twist) -> Result<&TwistBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }

    /// Returns a twist ID from its axis and transform.
    pub fn key_to_id(&self, key: TwistKey) -> Option<Twist> {
        self.data_to_id.get(key).copied()
    }

    /// Returns the inverse of a twist, or an error if the ID is out of range.
    pub fn inverse(&self, id: Twist) -> Result<Option<Twist>> {
        Ok(self.key_to_id(self.get(id)?.rev_key()?))
    }

    /// Validates and constructs a twist system.
    pub fn build(
        &self,
        build_ctx: Option<&BuildCtx>,
        puzzle_id: Option<&CatalogId>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<TwistSystem> {
        if let Some(build_ctx) = build_ctx {
            build_ctx.progress.lock().task = BuildTask::BuildingTwists;
        }

        let mut id = self.id.clone();
        if self.is_shared {
            if self.is_modified {
                warn_fn(eyre!("shared twist system cannot be modified"));
                if let Some(puzzle_id) = puzzle_id {
                    id = crate::ad_hoc_id(puzzle_id.clone());
                };
            }
            if self.name.is_none() {
                warn_fn(eyre!("twist system has no name"));
            }
        } else if !self.is_empty() {
            // TODO: canonicalize empty twist system
            warn_fn(eyre!("using ad-hoc twist system"));
        }
        let name = self.name.clone().unwrap_or_else(|| self.id.to_string());

        // Build axis system.
        let AxisSystemBuildOutput {
            axes,
            axis_vectors,
            axis_from_vector,
        } = self.axes.build()?;

        // Autoname twists.
        let names = Arc::new(
            self.names
                .clone()
                .build(self.len())
                .ok_or_eyre("missing twist names")?,
        );

        // Assemble list of twists.
        let mut twists: PerTwist<TwistInfo> = PerTwist::new();
        let mut twist_transforms: PerTwist<pga::Motor> = PerTwist::new();
        let mut gizmo_pole_distances: PerTwist<Option<f32>> = PerTwist::new();
        for (id, twist) in &self.by_id {
            let axis = twist.axis;
            let axis_vector = &axis_vectors[axis];

            if APPROX.ne(&twist.transform.transform(axis_vector), axis_vector) {
                warn_fn(eyre!("twist {:?} does not fix axis vector", &names[id]));
            }

            twists.push(TwistInfo {
                axis,
                scramble_max_multiplier: twist.scramble_max_multiplier,
            })?;
            twist_transforms.push(twist.transform.clone())?;
            gizmo_pole_distances.push(twist.gizmo_pole_distance)?;
        }

        let twist_from_transform = self.data_to_id.clone();

        let engine_data = NdEuclidTwistSystemEngineData {
            ndim: self.axes.ndim,

            axis_vectors: Arc::new(axis_vectors),
            axis_from_vector: Arc::new(axis_from_vector),

            twist_transforms: Arc::new(twist_transforms),
            twist_from_transform: Arc::new(twist_from_transform),

            gizmo_pole_distances: Arc::new(gizmo_pole_distances),

            hps_exports: Arc::clone(&self.hps_exports),
        };

        let twist_axes = Arc::new(twists.map_ref(|_, twist_info| twist_info.axis));

        let default_vantage_group_name = DEFAULT_VANTAGE_GROUP_NAME.to_string();
        let default_vantage_group = VantageGroupBuilder::default();
        let vantage_groups: IndexMap<String, NdEuclidVantageGroup> = self
            .vantage_groups
            .iter()
            .chain(
                self.vantage_groups
                    .is_empty()
                    .then_some((&default_vantage_group_name, &default_vantage_group)),
            )
            .map(|(id, vantage_group_builder)| {
                let vantage_group = vantage_group_builder.build(
                    Arc::clone(&axes.names),
                    Arc::clone(&names),
                    Arc::clone(&twist_axes),
                    engine_data.clone(), // relatively cheap; just a lot of `Arc::clone()`s
                )?;
                eyre::Ok((id.clone(), vantage_group))
            })
            .try_collect()?;

        let vantage_sets = self
            .vantage_sets
            .iter()
            .map(|vantage_set| vantage_set.build(&vantage_groups))
            .try_collect()?;

        let vantage_groups: IndexMap<String, BoxDynVantageGroup> = vantage_groups
            .into_iter()
            .map(|(k, v)| (k, BoxDynVantageGroup::new(v)))
            .collect();

        Ok(TwistSystem {
            id,
            name,

            axes: Arc::new(axes),

            names,
            twists,
            directions: self.directions.clone(),

            vantage_groups,
            vantage_sets,

            engine_data: engine_data.into(),
        })
    }

    /// "Unbuilds" a twist system.
    ///
    /// If the resulting twist system builder is modified, then it will emit a
    /// warning and change its ID.
    pub fn unbuild(twist_system: &TwistSystem) -> Result<Self> {
        let TwistSystem {
            id,
            name,
            axes,
            names,
            twists,
            directions,
            vantage_groups,
            vantage_sets,
            engine_data,
        } = twist_system;

        let engine_data = engine_data
            .downcast_ref::<NdEuclidTwistSystemEngineData>()
            .ok_or_eyre("expected NdEuclid twist system")?;

        let data_to_id = (*engine_data.twist_from_transform).clone();

        let nd_euclid_vantage_groups: IndexMap<String, NdEuclidVantageGroup> = vantage_groups
            .iter()
            .map(|(k, v)| {
                let vantage_group = v
                    .downcast_ref::<NdEuclidVantageGroup>()
                    .ok_or_eyre("expected NdEuclid vantage group")?;
                eyre::Ok((k.clone(), vantage_group.clone()))
            })
            .try_collect()?;

        let vantage_sets = vantage_sets
            .iter()
            .map(|vantage_set| VantageSetBuilder::unbuild(vantage_set, &nd_euclid_vantage_groups))
            .try_collect()?;

        let vantage_groups = vantage_groups
            .iter()
            .map(|(k, v)| eyre::Ok((k.clone(), VantageGroupBuilder::unbuild(v)?)))
            .try_collect()?;

        Ok(TwistSystemBuilder {
            id: id.clone(),
            name: Some(name.clone()),

            axes: AxisSystemBuilder::unbuild(axes, engine_data)?,

            by_id: twists.map_ref(|id, twist| TwistBuilder {
                axis: twist.axis,
                transform: engine_data.twist_transforms[id].clone(),
                gizmo_pole_distance: engine_data.gizmo_pole_distances[id],
                scramble_max_multiplier: twist.scramble_max_multiplier,
            }),
            names: (**names).clone().into(),
            data_to_id,
            autonames: AutoNames::default(),

            vantage_groups,
            vantage_sets,
            directions: directions.clone(),

            is_modified: false,
            is_shared: true,

            hps_exports: Arc::clone(&engine_data.hps_exports),
        })
    }
}

/// Error indicating a bad twist.
#[derive(thiserror::Error, Debug, Clone)]
pub enum BadTwist {
    #[error("twist transform cannot be identity")]
    Identity,
    #[error("identical twist already exists with ID {id} and name {name:?}")]
    DuplicateTwist { id: Twist, name: String },
    #[error("bad twist transform")]
    BadTransform,
}

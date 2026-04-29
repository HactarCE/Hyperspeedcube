use std::sync::Arc;

use eyre::{OptionExt, Result, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::{ExpectedAdHoc, MaybeAdHoc};
use hyperpuzzle_core::{ComponentList, DEFAULT_VANTAGE_GROUP_NAME};
use hyperpuzzlescript::builtins::catalog::HpsExports;
use indexmap::IndexMap;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};
use smallvec::SmallVec;

use super::{AdHocAxisSystemBuilder, VantageGroupBuilder, VantageSetBuilder};
use crate::components::{
    NdEuclidAxisVectors, NdEuclidTwistsList, PgaMotorToNearestTwist, TwistToPgaMotor,
};
use crate::{NamedTwistsList, NdEuclidVantageGroup, TwistKey};

/// Twist system, either fixed (taken from the catalog) or ad-hoc (currently
/// being constructed).
#[derive(Debug, Clone)]
pub struct TwistSystemBuilder(pub MaybeAdHoc<TwistSystem, Arc<Mutex<AdHocTwistSystemBuilder>>>);
impl TwistSystemBuilder {
    /// Locks the ad-hoc builder if it is one, or returns an error otherwise.
    pub fn lock_ad_hoc(&self) -> Result<MutexGuard<'_, AdHocTwistSystemBuilder>, ExpectedAdHoc> {
        Ok(self.0.as_ad_hoc()?.lock())
    }
}

/// Twist system being constructed.
#[derive(Debug)]
pub struct AdHocTwistSystemBuilder {
    /// Twist system ID.
    pub id: CatalogId,
    /// Name of the twist system.
    pub name: Option<String>,

    /// Axis system being constructed.
    pub axes: AdHocAxisSystemBuilder,

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

    /// Values exported by the Hyperpuzzlescript construction code.
    pub hps_exports: HpsExports,
}
impl AdHocTwistSystemBuilder {
    /// Constructs a new twist system builder.
    pub fn new(id: CatalogId, name: Option<String>, ndim: u8) -> Self {
        Self {
            id,
            name,
            axes: AdHocAxisSystemBuilder::new(ndim),
            by_id: PerTwist::new(),
            names: NameSpecBiMapBuilder::new(),
            data_to_id: ApproxHashMap::new(APPROX),
            autonames: AutoNames::default(),
            vantage_groups: IndexMap::new(),
            vantage_sets: vec![],
            directions: IndexMap::new(),
            hps_exports: HpsExports::new(),
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

    /// Returns the number of dimensions of the underlying space the puzzle is
    /// built in.
    pub fn ndim(&self) -> u8 {
        self.axes.ndim()
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
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<TwistSystem>> {
        if let Some(build_ctx) = build_ctx {
            build_ctx.set_building::<TwistSystem>();
        }

        let name = self.name.clone().unwrap_or_else(|| self.id.to_string());
        let meta = Arc::new(CatalogMetadata::simple(self.id.clone(), name));

        // Build axis system.
        let axes = self.axes.build()?;
        let axis_vectors = axes.components.get::<NdEuclidAxisVectors>()?;

        // Autoname twists.
        let twist_names = Arc::new(
            self.names
                .clone()
                .build(self.len())
                .ok_or_eyre("missing twist names")?,
        );

        // Assemble list of twists.
        let twists_list = Arc::new(NdEuclidTwistsList {
            ndim: self.ndim(),
            twist_axes: self.by_id.map_ref(|_, info| info.axis),
            twist_transforms: self.by_id.map_ref(|_, info| info.transform.clone()),
            twist_from_transform: self.data_to_id.clone(),
            gizmo_pole_distances: Some(self.by_id.map_ref(|_, info| info.gizmo_pole_distance)),
            scramble_max_multipliers: self.by_id.map_ref(|_, info| info.scramble_max_multiplier),
        });

        // Check that each transform stabilizes its axis.
        for twist_id in twists_list.iter() {
            let axis = twists_list.twist_axes[twist_id];
            let axis_vector = &axis_vectors.vectors_by_id[axis];
            let transform = &twists_list.twist_transforms[twist_id];
            if APPROX.ne(&transform.transform(axis_vector), axis_vector) {
                warn_fn(eyre!(
                    "twist {:?} does not fix axis vector",
                    &twist_names[twist_id]
                ));
            }
        }

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
                    Arc::clone(&twist_names),
                    Arc::clone(axis_vectors),
                    Arc::clone(&twists_list),
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

        let mut components = ComponentList::new();
        components.insert(Arc::clone(&twists_list));
        components.insert(Arc::new(NamedTwistsList {
            names: Arc::clone(&twist_names),
        }));
        components.insert({
            let twist_names = Arc::clone(&twist_names);
            let twists_list = Arc::clone(&twists_list);
            TwistToPgaMotor::new(move |transform| {
                let twist_id = twist_names.id_from_name(&transform.family)?;
                Some(twists_list.twist_transforms[twist_id].clone())
            })
        });
        components.insert(PgaMotorToNearestTwist::from_transforms_and_multipliers(
            axes.len(),
            twists_list.iter().map(|twist| {
                let axis = twists_list.twist_axes[twist];
                let motor = twists_list.twist_transforms[twist].clone();
                let mv = Move::new((), &twist_names[twist], None, Multiplier(1));
                (axis, motor, mv)
            }),
        ));
        components.insert(Arc::new(self.hps_exports.clone()));

        Ok(Arc::new(TwistSystem {
            meta,

            axes: Arc::new(axes),
            axis_from_family: Box::new(move |family_str| {
                let twist_id = twist_names.id_from_name(family_str)?;
                Some(twists_list.twist_axes[twist_id])
            }),

            directions: self.directions.clone(),

            vantage_groups,
            vantage_sets,

            components,
        }))
    }
}

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

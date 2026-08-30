use std::f64::consts::TAU;
use std::sync::Arc;

use eyre::{Context, OptionExt, Result, bail, eyre};
use hypergroup::{
    ConjugateCoset, GroupAction, GroupElementId, IsometryGroup, SubgroupAction,
    SubgroupConstraintSolver,
};
use hypermath::{Float, Matrix, Point, Vector, VectorRef};
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::{
    Axis, AxisSystem, CatalogId, CatalogObject, CatalogWord, ComponentList, IndexOverflow, Names,
    PerAxis, TwistSystem, TypedIndex, TypedIndexIter,
};
use hyperpuzzle_impl_nd_euclid::NdEuclidAxisVectors;
use hypuz_notation::family::SequentialLowercaseName;
use hypuz_util::FloatMinMaxByIteratorExt;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::{FactorNamedPointBasedNames, NamedPointOrbit, ProductNamedPointBasedNames};
use crate::{
    AxisOrbitJumbleData, FactorTwistSystemSpec, JumbleTransform, NamedPoint, NamedPointSet,
    PerNamedPoint, StabilizerFamily, SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent,
    UniqueMinimalClockwiseGenerator,
};

#[derive(Debug, Clone)]
pub struct TwistSystemFactor {
    pub id: CatalogId,
    pub name: String,
    pub names: FactorNamedPointBasedNames<Axis>,
    pub axis_orbits: Vec<AxisOrbit>,
    pub named_point_orbits: Vec<NamedPointOrbit>,
}

impl TwistSystemFactor {
    #[must_use]
    fn offset_ids_by(
        &self,
        axis_id_offset: usize,
        named_point_id_offset: usize,
    ) -> Result<Self, IndexOverflow> {
        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            names: self.names.clone(),
            axis_orbits: self
                .axis_orbits
                .iter()
                .map(|orbit| orbit.offset_ids_by(axis_id_offset, named_point_id_offset))
                .try_collect()?,
            named_point_orbits: self
                .named_point_orbits
                .iter()
                .map(|orbit| orbit.offset_ids_by(named_point_id_offset))
                .try_collect()?,
        })
    }

    fn add_auto_stabilizer_twists(&mut self, total_ndim: u8, new_sets: &[(NamedPointSet, Float)]) {
        for orbit in &mut self.axis_orbits {
            if total_ndim <= 4 {
                orbit.stabilizer_twists.extend(new_sets.iter().cloned().map(
                    |(setwise_stabilized_set, gizmo_pole_distance)| StabilizerTwistBuilder {
                        setwise_stabilized_set,
                        gizmo_pole_distance,
                        auto_generated: true,
                    },
                ));
            } else {
                orbit.stabilizer_twists.clear();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AxisOrbit {
    /// Number of axes in the orbit.
    pub len: usize,
    /// ID offset of the axes in the orbit.
    ///
    /// IDs within an orbit always count starting from 0, but the puzzle may
    /// have multiple sets and so puzzle-facing IDs for axes must start counting
    /// from this offset.
    pub id_offset: usize,
    /// Orbits of nontrivial stabilizer twist families, along with their gizmo
    /// pole distances.
    ///
    /// Here, "nontrivial" means that the named point set is nonempty. E.g., the
    /// named point set is typically empty for twists on rotational 3D puzzles,
    /// but these do not need to be tracked. But in rotational 4D puzzles, they
    /// do need to be tracked.
    ///
    /// Because they are not used in higher dimensions, this list is made empty
    /// in 5D+.
    pub stabilizer_twists: Vec<StabilizerTwistBuilder>,
    /// Jumble moves, not including the (optional) doctrinaire move.
    pub jumble_moves: Vec<JumbleTransform>,
    /// Jumble stops, not yet expanded by symmetry and not including the
    /// doctrinaire stop.
    pub jumble_stops: Vec<Float>,
}

impl AxisOrbit {
    pub fn first(&self) -> Axis {
        Axis(self.id_offset as _) // already checked at construction
    }

    pub fn axes(&self) -> TypedIndexIter<Axis> {
        Axis::iter_range(self.id_offset..self.id_offset + self.len) // already checked at constuction
    }

    fn offset_ids_by(
        &self,
        axis_id_offset: usize,
        named_point_id_offset: usize,
    ) -> Result<Self, IndexOverflow> {
        let new_id_offset = axis_id_offset + self.id_offset;
        Axis::try_iter_range(new_id_offset..new_id_offset + self.len)?; // check for overflow
        Ok(Self {
            len: self.len,
            id_offset: new_id_offset,
            stabilizer_twists: self
                .stabilizer_twists
                .iter()
                .map(|stab_twist_builder| stab_twist_builder.offset_ids_by(named_point_id_offset))
                .try_collect()?,
            jumble_moves: self.jumble_moves.clone(),
            jumble_stops: self.jumble_stops.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct StabilizerTwistBuilder {
    /// Set of points that are cycled (and thus setwise-stabilized) by the
    /// twist.
    pub setwise_stabilized_set: NamedPointSet,
    /// Gizmo pole distance for the twist in 4D. This is ignored for 3D puzzles,
    /// which use the axis vector to determine the gizmo pole distance.
    pub gizmo_pole_distance: Float,
    /// Whether the stabilizer twist was automatically generated via the puzzle
    /// product construction, in which case it is ok if it does not correspond
    /// to a valid twist.
    pub auto_generated: bool,
}

impl StabilizerTwistBuilder {
    fn offset_ids_by(&self, named_point_id_offset: usize) -> Result<Self, IndexOverflow> {
        Ok(Self {
            setwise_stabilized_set: self
                .setwise_stabilized_set
                .offset_ids_by(named_point_id_offset)?,
            gizmo_pole_distance: self.gizmo_pole_distance,
            auto_generated: self.auto_generated,
        })
    }
}

/// Twist system of a puzzle under construction.
#[derive(Debug, Clone)]
pub struct TwistSystemProduct {
    /// ID computed from `factor_ids`.
    pub id: CatalogId,

    /// Grip group.
    pub group: IsometryGroup,
    /// Possibly-incomplete list of normal vectors for mirror planes bounding
    /// the fundamental region of the grip group.
    ///
    /// This is used for optimization purposes only. It is always acceptable for
    /// this list to be empty.
    pub fundamental_region_mirrors: Vec<Vector>,

    /// Action of the grip group on named points.
    pub named_point_action: GroupAction<NamedPoint>,
    /// Vector for each named point.
    pub named_point_vectors: PerNamedPoint<Vector>,
    /// Normalized vector for each named point.
    pub named_point_unit_vectors: PerNamedPoint<Vector>,

    /// Action of the grip group on axes.
    pub axis_action: GroupAction<Axis>,
    /// Vector for each axis.
    ///
    /// The vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 3D and 4D. For a facet-turning puzzle, each
    /// axis vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub axis_vectors: PerAxis<Vector>,

    pub factors: Vec<TwistSystemFactor>,

    /// Nonempty sets of named points, each with a gizmo pole distance. Each
    /// orbit has only one representative in this list.
    ///
    /// Generally, every named point orbit should have a set in this list
    /// containing one named point from its orbit.
    ///
    /// These named point sets are used to construct twist gizmos and stabilizer
    /// twists for 4D puzzles. Because they are not needed in higher dimensions,
    /// this list is made empty in 4D+.
    pub named_point_set_orbits: Vec<(NamedPointSet, Float)>,
}

impl CatalogObject for TwistSystemProduct {
    fn catalog_type_name() -> &'static str {
        "symmetric twist system product"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

impl TwistSystemProduct {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Constructs the empty axis system, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        Self {
            id: crate::product_id([].into_iter()),
            group: IsometryGroup::trivial(),
            fundamental_region_mirrors: vec![],
            named_point_action: GroupAction::trivial(),
            named_point_vectors: PerNamedPoint::new(),
            named_point_unit_vectors: PerNamedPoint::new(),
            axis_action: GroupAction::trivial(),
            axis_vectors: PerAxis::new(),
            factors: vec![],
            named_point_set_orbits: vec![],
        }
    }

    pub fn new_empty(ndim: u8) -> Self {
        Self {
            id: CatalogId::new(
                "empty".parse::<CatalogWord>().expect("bad ID"),
                [(ndim as i64).into()],
                None,
            ),
            ..Self::direct_product_identity()
        }
    }

    /// Constructs a product twist system builder with a single factor.
    pub fn new_factor(
        spec: &FactorTwistSystemSpec,
        _warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let group = spec.symmetry.clone();

        let (named_point_vectors, named_point_unit_vectors, named_point_orbits, mut names) =
            FactorNamedPointBasedNames::<Axis>::from_spec(&group, &spec.named_point_orbits)?;

        let named_point_points = named_point_vectors.map_ref(|_, v| Point(v.clone()));
        let named_point_action = group.action_on_points(&named_point_points)?;

        let mut axis_orbits = vec![];
        let mut axis_deorbiters = PerAxis::new();
        let mut axis_which_orbit = PerAxis::new();
        let mut axis_vectors = PerAxis::new();
        let mut axis_id_offset = 0;
        for (orbit_index, orbit) in spec.axis_orbits.iter().enumerate() {
            let orbit_members =
                orbit.expand_and_name(&group, &named_point_unit_vectors, &named_point_action)?;
            axis_orbits.push(AxisOrbit {
                len: orbit_members.len(),
                id_offset: axis_id_offset,
                stabilizer_twists: vec![], // will be populated later, after naming
                jumble_moves: vec![],      // will be populated later, after naming
                jumble_stops: vec![],      // will be populated later, after naming
            });
            axis_id_offset += orbit_members.len();
            for (undeorbiter, axis_vector, axis_name) in orbit_members {
                axis_deorbiters.push(group.inverse(undeorbiter))?;
                axis_which_orbit.push(orbit_index)?;
                axis_vectors.push(axis_vector)?;
                names.add_member(&orbit.prefix, axis_name)?;
            }
        }

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        let axis_action = group.action_on_points(&axis_points)?;

        // Populate stabilizer twists
        for orbit in &spec.stabilizer_twist_orbits {
            let axis = names.member_from_name(&orbit.axis_name)?;
            let stabilized_points = NamedPointSet::new(
                orbit
                    .named_points
                    .iter()
                    .map(|s| names.named_point_from_name(s))
                    .try_collect()?,
            )?;
            let axis_orbit_index = axis_which_orbit[axis];
            let deorbiter = axis_deorbiters[axis];
            axis_orbits[axis_orbit_index]
                .stabilizer_twists
                .push(StabilizerTwistBuilder {
                    setwise_stabilized_set: stabilized_points
                        .transform_by_group_element(&named_point_action, deorbiter),
                    gizmo_pole_distance: orbit.gizmo_pole_distance,
                    auto_generated: false,
                });
            if orbit.gizmo_pole_distance <= 0.0 {
                bail!("stabilizer twist gizmo_pole_distance cannot be negative")
            }
        }

        let mut named_point_set_orbits: Vec<(NamedPointSet, Float)> = vec![];
        // Add a singleton named point set for each named point.
        for orbit in &named_point_orbits {
            named_point_set_orbits.push((
                NamedPointSet::new(smallvec![orbit.first()?])?,
                named_point_vectors[orbit.first()?].mag(),
            ));
        }
        // Add non-singleton named point sets.
        for orbit in &spec.named_point_set_orbits {
            let points = orbit
                .named_points
                .iter()
                .map(|s| names.named_point_from_name(s))
                .try_collect()?;
            named_point_set_orbits.push((NamedPointSet::new(points)?, orbit.gizmo_pole_distance));
            if orbit.gizmo_pole_distance <= 0.0 {
                bail!("named point set gizmo_pole_distance cannot be negative")
            }
        }

        // Assemble jumble moves.
        for jumble_move_spec in &spec.jumble_moves {
            let axis = names.member_from_name(&jumble_move_spec.axis)?;
            let orbit_index = axis_which_orbit[axis];
            let angle = match &jumble_move_spec.angle {
                crate::JumbleAngleSpec::FromTo(start, end) => {
                    let fixed_vector = &axis_vectors[axis];
                    let start_vector = &axis_vectors[names.member_from_name(&start)?];
                    let end_vector = &axis_vectors[names.member_from_name(&end)?];
                    let reject_and_normalize = |v: &Vector| {
                        v.rejected_from(fixed_vector)
                            .and_then(|u| u.normalize())
                            .unwrap_or_else(|| v.clone())
                    };
                    Vector::dot(
                        &reject_and_normalize(start_vector),
                        &reject_and_normalize(end_vector),
                    )
                    .acos()
                }
                crate::JumbleAngleSpec::Angle(a) => *a,
            };
            axis_orbits[orbit_index]
                .jumble_moves
                .push(JumbleTransform::new_unit_jumbling(
                    jumble_move_spec.suffix,
                    angle,
                ));
        }

        // Assemble jumble stops.
        for jumble_stop_spec in &spec.jumble_stops {
            let axis = names.member_from_name(&jumble_stop_spec.axis)?;
            let orbit_index = axis_which_orbit[axis];
            let angle = axis_orbits[orbit_index]
                .jumble_moves
                .iter()
                .find(|mv| mv.suffix == Some(jumble_stop_spec.suffix))
                .ok_or_else(|| {
                    eyre!("no jumble suffix \"{}\"", jumble_stop_spec.suffix)
                        .wrap_err(format!("calculating jumble stop {jumble_stop_spec:?}"))
                })?
                .angle
                * jumble_stop_spec.multiplier.0 as Float;
            axis_orbits[orbit_index].jumble_stops.push(angle);
        }

        let id = crate::product_id([&spec.id].into_iter());
        let factor = TwistSystemFactor {
            id: spec.id.clone(),
            name: spec.name.clone(),
            names,
            axis_orbits,
            named_point_orbits,
        };

        Ok(Self {
            id,

            group,
            fundamental_region_mirrors: match &spec.coxeter_matrix {
                Some(coxeter) => coxeter.mirrors()?.cols().map(|v| v.to_vector()).collect(),
                None => vec![],
            },

            named_point_action,
            named_point_vectors,
            named_point_unit_vectors,

            axis_action,
            axis_vectors,

            factors: vec![factor],

            named_point_set_orbits,
        })
    }

    /// Returns the direct product of two axis systems.
    ///
    /// See [`super::ProductPuzzleBuilder::direct_product()`].
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;
        let ndim = a.ndim() + b.ndim();

        if (a.len() + b.len()).saturating_sub(1) > Axis::MAX_INDEX {
            return Err(IndexOverflow::new::<Axis>().into());
        }

        let group = IsometryGroup::product([&a.group, &b.group])?;
        let named_point_action =
            GroupAction::product([&a.named_point_action, &b.named_point_action])?;
        let axis_action = GroupAction::product([&a.axis_action, &b.axis_action])?;

        let fundamental_region_mirrors = std::iter::chain(
            a.fundamental_region_mirrors
                .iter()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.fundamental_region_mirrors
                .iter()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let named_point_vectors = std::iter::chain(
            a.named_point_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.named_point_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();
        let named_point_unit_vectors = std::iter::chain(
            a.named_point_unit_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.named_point_unit_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();
        let axis_vectors = std::iter::chain(
            a.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let lift_b_point_set =
            |points: &NamedPointSet| points.offset_ids_by(a.named_points_count());

        let a_new_named_point_set_orbits: Vec<(NamedPointSet, f64)> =
            a.named_point_set_orbits.clone();
        let b_new_named_point_set_orbits: Vec<(NamedPointSet, f64)> = b
            .named_point_set_orbits
            .iter()
            .map(|(set, distance)| eyre::Ok((lift_b_point_set(set)?, *distance)))
            .try_collect()?;

        let mut a_new_factors: Vec<TwistSystemFactor> = a.factors.clone();
        let mut b_new_factors: Vec<TwistSystemFactor> = b
            .factors
            .iter()
            .map(|factor| factor.offset_ids_by(a.axis_vectors.len(), a.named_point_vectors.len()))
            .try_collect()?;
        for factor in &mut a_new_factors {
            factor.add_auto_stabilizer_twists(ndim, &b_new_named_point_set_orbits);
        }
        for factor in &mut b_new_factors {
            factor.add_auto_stabilizer_twists(ndim, &a_new_named_point_set_orbits);
        }

        let factors: Vec<TwistSystemFactor> =
            std::iter::chain(a_new_factors, b_new_factors).collect();

        let named_point_set_orbits = if ndim <= 4 {
            std::iter::chain(a_new_named_point_set_orbits, b_new_named_point_set_orbits).collect()
        } else {
            vec![]
        };

        Ok(Self {
            id: crate::product_id(factors.iter().map(|f| &f.id)),

            group,
            fundamental_region_mirrors,

            named_point_action,
            named_point_vectors,
            named_point_unit_vectors,

            axis_action,
            axis_vectors,

            factors,

            named_point_set_orbits,
        })
    }

    /// Returns the number of axes on the puzzle.
    pub fn len(&self) -> usize {
        self.axis_vectors.len()
    }
    /// Returns the number of named points on the puzzle.
    pub fn named_points_count(&self) -> usize {
        self.named_point_vectors.len()
    }

    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<TwistSystem>> {
        build_ctx.push_task("naming named points");
        let names = ProductNamedPointBasedNames::product(
            self.factors.iter().map(|f| f.names.clone()).collect(),
        );
        build_ctx.pop_task();

        let mut components = ComponentList::new();
        let axis_vectors = Arc::new(NdEuclidAxisVectors::from_vectors(
            self.ndim(),
            self.axis_vectors.clone(),
        ));
        components.insert(Arc::clone(&axis_vectors));

        build_ctx.push_task("constructing axis system");
        let axes = Arc::new(AxisSystem {
            names: Arc::new(names.build_member_names()?),
            orbits: vec![], // technically exists, but not necessary
            components,
        });
        build_ctx.pop_task();

        build_ctx.push_task("building named point names");
        let named_point_names = Arc::new(names.build_named_point_names()?);
        build_ctx.pop_task();

        let mut components = ComponentList::new();
        build_ctx.push_task("constructing symmetric twist system component");
        build_ctx.push_task("building axis undeorbiters");
        let axis_undeorbiters = Arc::new(self.build_axis_undeorbiters()?);
        build_ctx.pop_task();
        build_ctx.push_task("building axis orbits");
        let axis_orbits = Arc::new(self.build_axis_orbits(
            build_ctx,
            &axes.names,
            &named_point_names,
            warn_fn,
        )?);
        build_ctx.pop_task();
        components.insert(Arc::new(SymmetricTwistSystemComponent {
            axes: Arc::clone(&axes),
            group: self.group.clone(),
            fundamental_region_mirrors: self.fundamental_region_mirrors.clone(),
            axis_action: self.axis_action.clone(),

            axis_undeorbiters,
            axis_orbits,
            axis_vectors,

            named_point_action: self.named_point_action.clone(),
            named_point_names,
            named_point_vectors: Arc::new(self.named_point_vectors.clone()),
        }));
        build_ctx.pop_task();

        // TODO: verify that named points are sufficient to describe
        //       symmetry group elements

        Ok(Arc::new(TwistSystem {
            id: self.id.clone(),
            name: self.name(),
            components,
            axes: Arc::clone(&axes),
            axis_from_family: Box::new(move |family_str| {
                // TODO: correct number of underscores
                let axis_name = match family_str.split_once('_') {
                    Some((first, _)) => first,
                    None => family_str,
                };
                axes.names.lookup(axis_name)
            }),
            ..TwistSystem::new_empty()
        }))
    }

    /// Returns an iterator over all axis orbits, each paired with the ID of the
    /// first axis in that orbit.
    pub fn axis_orbits(&self) -> impl Iterator<Item = &AxisOrbit> {
        self.factors.iter().flat_map(|factor| &factor.axis_orbits)
    }

    pub fn axis_from_name(&self, axis_name: &str) -> Option<Axis> {
        let (SequentialLowercaseName(factor_index), rest) = if self.factors.len() == 1 {
            (SequentialLowercaseName(0), axis_name)
        } else {
            hypuz_notation::family::strip_sequential_lowercase_prefix(axis_name)?
        };

        self.factors
            .get(factor_index as usize)?
            .names
            .member_from_name(rest)
            .ok()
    }

    pub fn orbit_containing_axis(&self, axis: Axis) -> Option<usize> {
        // This could be faster, but it doesn't need to be
        self.axis_orbits()
            .position(|orbit| orbit.axes().contains(&axis))
    }

    fn build_axis_undeorbiters(&self) -> Result<PerAxis<(GroupElementId, usize)>> {
        let mut ret = PerAxis::new_with_len(self.len());

        for (orbit_index, orbit) in self.axis_orbits().enumerate() {
            ret[orbit.first()] = (GroupElementId::IDENTITY, orbit_index);
            hypergroup::orbit(
                (orbit.first(), GroupElementId::IDENTITY),
                self.group.generators(),
                |&(ax, undeorbiter), &g| {
                    let new_ax = self.axis_action.act(g, ax);
                    (new_ax != orbit.first() && ret[new_ax].0 == GroupElementId::IDENTITY).then(
                        || {
                            let new_undeorbiter = self.group.compose(g, undeorbiter);
                            ret[new_ax] = (new_undeorbiter, orbit_index);
                            (new_ax, new_undeorbiter)
                        },
                    )
                },
            );

            // Sanity check that we didn't miss any
            #[cfg(debug_assertions)]
            {
                let axes_in_orbit = Axis::iter_range(
                    orbit.first().to_index()..orbit.first().to_index() + orbit.len,
                );
                for ax in axes_in_orbit.skip(1) {
                    assert_ne!(ret[ax].0, GroupElementId::IDENTITY);
                    assert_eq!(ret[ax].1, orbit_index);
                }
            }
        }

        Ok(ret)
    }

    fn build_axis_orbits(
        &self,
        build_ctx: &BuildCtx,
        axis_names: &Names<Axis>,
        named_point_names: &Names<NamedPoint>,
        _warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Vec<SymmetricTwistSystemAxisOrbit>> {
        let mut ret = vec![];
        for orbit in self.axis_orbits() {
            let first_axis_vector = &self.axis_vectors[orbit.first()];

            build_ctx.push_task(format!("building orbit of {}", axis_names[orbit.first()]));

            build_ctx.push_task("constructing subgroup constraint solver");
            let subgroup_action =
                SubgroupAction::from_subgroup_predicate(&self.named_point_action, |e| {
                    self.axis_action.act(e, orbit.first()) == orbit.first()
                })?;
            let subgroup_has_reflection = subgroup_action
                .subgroup_generators()
                .into_iter()
                .any(|e| self.group.is_reflection(e));
            let mut subgroup_solver = SubgroupConstraintSolver::new(subgroup_action);
            build_ctx.pop_task();

            let stabilizer_twist_families = match self.group.ndim() {
                3 => &[StabilizerTwistBuilder {
                    setwise_stabilized_set: NamedPointSet::EMPTY,
                    gizmo_pole_distance: 0.0, // doesn't matter for 3D
                    auto_generated: false,
                }],
                4 => &*orbit.stabilizer_twists,
                _ => &[],
            };

            build_ctx.push_task("computing stabilizer twists");
            let mut stabilizer_twists: Vec<(NamedPointSet, UniqueMinimalClockwiseGenerator, f64)> =
                vec![];
            for stabilizer_twist_family in stabilizer_twist_families {
                let StabilizerTwistBuilder {
                    setwise_stabilized_set: secondary,
                    gizmo_pole_distance,
                    auto_generated,
                } = stabilizer_twist_family;

                let twist_name = StabilizerFamily {
                    primary: orbit.first(),
                    secondary: secondary.clone(),
                }
                .name(axis_names, named_point_names);

                build_ctx.push_task(format!("computing orbit of {twist_name}"));

                build_ctx.push_task("computing stabilized coset");
                let coset = subgroup_solver
                    .solve(&cycle_constraints(secondary.iter()))
                    .ok_or_else(|| {
                        eyre!(
                            "stabilizer twist {:?} imposes unsatisfiable constraints \
                             (hint: if there are more than 3 stabilized points, \
                             ensure they are in cyclic order)",
                            twist_name,
                        )
                    })?;
                build_ctx.pop_task();

                build_ctx.push_task("computing unit twist transform");
                let unit_twist_transform_result = if secondary.is_empty() {
                    unit_twist_transform(&self.group, &coset, &[first_axis_vector.clone()])
                } else {
                    let secondary_vector = secondary.vector(&self.named_point_vectors);
                    let stabilized_vectors = [first_axis_vector.clone(), secondary_vector];
                    unit_twist_transform(&self.group, &coset, &stabilized_vectors)
                }
                .wrap_err_with(|| {
                    format!(
                        "error calculating unit twist transform \
                         for stabilizer twist {twist_name:?}",
                    )
                });
                // Allow errors if auto-generated
                let opt_unit_twist_transform = Some(unit_twist_transform_result)
                    .filter(|result| result.is_ok() || !*auto_generated)
                    .transpose()?;
                build_ctx.pop_task();

                build_ctx.pop_task();

                if let Some(unit_twist_transform) = opt_unit_twist_transform {
                    stabilizer_twists.push((
                        secondary.clone(),
                        unit_twist_transform,
                        *gizmo_pole_distance,
                    ));
                }
            }
            build_ctx.pop_task();

            build_ctx.push_task("computing jumble data");
            let mut jumble_data = None;
            if !orbit.jumble_moves.is_empty() {
                let mut transforms = orbit.jumble_moves.clone();

                let doctrinaire_order = stabilizer_twists
                    .iter()
                    .find(|(point_set, _, _)| point_set.is_empty())
                    .map(|(_, clockwise_generator, _)| clockwise_generator.order.get() as usize);
                if let Some(n) = doctrinaire_order {
                    transforms.insert(0, JumbleTransform::new_unit_doctrinaire(n));
                }

                let mut stops = orbit.jumble_stops.clone();

                // Add mirror stops.
                if subgroup_has_reflection {
                    stops.extend(stops.clone().into_iter().map(|s| -s));
                }

                // Add doctrinaire stop.
                stops.push(0.0);

                // Expand by doctrinaire symmetry. This is only correct because
                // the doctrinaire twist is taken from the puzzle symmetry; if
                // the doctrinaire twist were a subset of puzzle symmetry, then
                // we would need to expand only by the puzzle symmetry.
                let n = doctrinaire_order.unwrap_or(1);
                let stops = itertools::iproduct!(0..n, &stops)
                    .map(|(i, a)| a + TAU * i as Float / n as Float)
                    .collect_vec();

                if stops.is_empty() {
                    build_ctx.warn_fn()(eyre!(
                        "axis orbit of {} has jumble moves but no jumble stops",
                        axis_names[orbit.first()],
                    ));
                }

                jumble_data = Some(AxisOrbitJumbleData::new(transforms, stops)?);
            }
            build_ctx.pop_task();

            ret.push(SymmetricTwistSystemAxisOrbit {
                first: orbit.first(),
                len: orbit.len,
                subgroup_solver: Mutex::new(subgroup_solver),
                stabilizer_twists,
                jumble_data,
            });

            build_ctx.pop_task();
        }

        Ok(ret)
    }

    pub fn name(&self) -> String {
        crate::product_name(self.factors.iter().map(|f| &f.name))
    }
}

/// Returns the unique minimal clockwise generator from a coset, or `None` if
/// there is not one.
///
/// `stabilized_vectors` must be a list of vectors of length `ndim-2`, and is
/// used to define "clockwise."
fn unit_twist_transform(
    group: &IsometryGroup,
    stabilizer_coset: &ConjugateCoset,
    stabilized_vectors: &[Vector],
) -> Result<UniqueMinimalClockwiseGenerator> {
    if stabilized_vectors.len() + 2 != group.ndim() as usize {
        bail!("`stabilized_vectors` must have length ndim-2");
    }
    let nontrivial_rotations = stabilizer_coset
        .elements()
        .into_iter()
        .filter(|&e| e != GroupElementId::IDENTITY)
        .filter(|&e| !group.is_reflection(e))
        .collect_vec();
    let (mut min_group_element, min_rotation) = nontrivial_rotations
        .iter()
        .filter_map(|&e| Some((e, group.motor(e).normalize()?)))
        .max_by_float_key(|(_e, m)| m.scalar().abs())
        .ok_or_eyre("empty coset")?;
    let arbitrary_perpendicular_vector =
        Vector::arbitrary_perpendicular_to(group.ndim(), stabilized_vectors)
            .ok_or_eyre("stabilized vectors cannot span all of space")?;
    let orientation = Matrix::from_cols(
        std::iter::chain(
            stabilized_vectors,
            [
                &arbitrary_perpendicular_vector,
                &min_rotation.transform(&arbitrary_perpendicular_vector),
            ],
        )
        .collect_vec(), // Chain does not impl ExactSizeIterator
    )
    .determinant();
    if orientation > 0.0 {
        min_group_element = group.inverse(min_group_element);
    }

    Ok(UniqueMinimalClockwiseGenerator::new(
        group.abstract_group(),
        min_group_element,
    ))
}

/// Constructs a constraint set for a cycle of points.
fn cycle_constraints(
    points: impl Iterator<Item = NamedPoint> + Clone + ExactSizeIterator,
) -> hypergroup::ConstraintSet<NamedPoint> {
    hypergroup::ConstraintSet::from_iter(
        points
            .circular_tuple_windows()
            .map(|(from, to)| hypergroup::Constraint { from, to }),
    )
}

#[cfg(test)]
mod tests {
    use hypergroup::GeneratorId;
    use hypermath::APPROX;

    use super::*;

    #[test]
    fn test_unit_twist_transform() -> Result<()> {
        let h3 = hypergroup::CoxeterMatrix::H3();
        let group =
            hypergroup::CoxeterMatrix::direct_product(&h3, &hypergroup::CoxeterMatrix::A(1)?)?
                .isometry_group()?;

        let named_point_vectors: PerNamedPoint<Vector> = group
            .orbit_geometric(
                h3.mirror_basis()?.col(2).to_vector(),
                hypergroup::ORBIT_LIMIT,
            )?
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        let points: PerNamedPoint<Point> = named_point_vectors.map_ref(|_, v| Point(v.clone()));

        let named_point_action = group.action_on_points(&points)?;

        let w = Vector::unit(3);

        let subgroup_action = SubgroupAction::from_subgroup_predicate(&named_point_action, |e| {
            APPROX.eq(&group.motor(e).transform(&w), &w) // stabilize W axis
        })?;
        let mut subgroup_solver = SubgroupConstraintSolver::new(subgroup_action);

        let g1 = group.generators()[GeneratorId(1)];
        let g2 = group.generators()[GeneratorId(2)];

        let f = NamedPoint(0);
        let u = named_point_action.act(g2, f);
        let r = named_point_action.act(g1, u);

        let mut check_unit_twist_transform = |stab: &[NamedPoint], period: usize| -> Result<()> {
            println!("Testing setwise stabilizer {:?} with period {period}", stab);

            let secondary_vector: Vector = stab.iter().map(|&p| &named_point_vectors[p]).sum();
            let stabilized_vectors = [w.clone(), secondary_vector];

            // unit_twist_transform() should always produce a clockwise
            // rotation, regardless of the input cycle direction.
            let forward_coset = subgroup_solver
                .solve(&cycle_constraints(stab.iter().copied()))
                .expect("unsat");
            let reverse_coset = subgroup_solver
                .solve(&cycle_constraints(stab.iter().rev().copied()))
                .expect("unsat");
            let unit1 = unit_twist_transform(&group, &forward_coset, &stabilized_vectors)?;
            let unit2 = unit_twist_transform(&group, &reverse_coset, &stabilized_vectors)?;
            assert_eq!(group.abstract_group().period(unit1.element), period);
            assert_eq!(group.abstract_group().period(unit2.element), period);
            assert_eq!(unit1, unit2);
            Ok(())
        };

        check_unit_twist_transform(&[u], 5)?; // face twist
        check_unit_twist_transform(&[u, r], 2)?; // edge twist
        check_unit_twist_transform(&[u, r, f], 3)?; // vertex twist

        Ok(())
    }
}

//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::sync::Arc;

use eyre::{OptionExt, Result};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::{catalog::VersionedCatalogWord, group::GroupElementId};
use hyperpuzzle_impl_nd_euclid::{
    NdEuclidAxisVectors, NdEuclidPuzzleAnimation, NdEuclidPuzzleStateRenderData,
};

mod builder;
mod cut_distances;
mod geometry;
pub mod hps;
mod named_point;
mod spec;
mod stabilizer_family;
mod twist_system;

use builder::{ColorSystemDisjointUnion, PuzzleProduct, TwistSystemProduct};
pub use cut_distances::CutDistances;
use hypuz_util::FloatMinMaxIteratorExt;
use itertools::Itertools;
pub use named_point::{NamedPoint, NamedPointSet, PerNamedPoint};
pub use spec::*;
pub use stabilizer_family::StabilizerFamily;
pub use twist_system::{
    AxisOrbitJumbleData, JumbleStop, JumbleStopInfo, JumbleTransform, PerJumbleStop,
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent, UniqueMinimalClockwiseGenerator,
};

const ROT_ID: &str = "rot";
const REFLE_ID: &str = "refle";

pub fn product_base_id() -> VersionedCatalogWord {
    "product@1".parse().expect("bad catalog ID")
}

pub fn disjoint_union_base_id() -> VersionedCatalogWord {
    "sum@1".parse().expect("bad catalog ID")
}

const ROT_NAME_PREFIX: &str = "Rot ";
const REFLE_NAME_PREFIX: &str = "Refle ";

const PRODUCT_GENERATOR_VERSION: Version = Version {
    major: 1,
    minor: 0,
    patch: 0,
};

fn product_id<'a>(mut factor_ids: impl ExactSizeIterator<Item = &'a CatalogId>) -> CatalogId {
    if factor_ids.len() == 1 {
        return factor_ids.next().expect("bad iterator len").clone();
    }
    CatalogId::new(
        product_base_id(),
        [CatalogIdValue::List(
            factor_ids.map(|id| id.clone().into()).collect(),
        )],
        None,
    )
}

fn disjoint_union_id<'a>(
    mut summand_ids: impl ExactSizeIterator<Item = &'a CatalogId>,
) -> CatalogId {
    if summand_ids.len() == 1 {
        return summand_ids.next().expect("bad iterator len").clone();
    }
    CatalogId::new(
        disjoint_union_base_id(),
        [CatalogIdValue::List(
            summand_ids.map(|id| id.clone().into()).collect(),
        )],
        None,
    )
}

fn product_name<'a>(factor_names: impl IntoIterator<Item = &'a String>) -> String {
    factor_names.into_iter().join(" × ")
}

fn sum_name<'a>(summand_names: impl IntoIterator<Item = &'a String>) -> String {
    summand_names.into_iter().join(" + ")
}

fn make_puzzle_tag_set(ndim: Option<i64>) -> Result<TagSet> {
    let mut tags = TagSet::new();
    tags.insert_named("doctrinaire", true.into())?;
    tags.insert_named("pseudodoctrinaire", true.into())?;
    if let Some(ndim) = ndim {
        tags.insert_named("ndim", ndim.into())?;
    }
    Ok(tags)
}

pub fn add_catalog_entries(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    // TODO: redirect when nested; e.g., `product([product(a,b),c])` ->
    // `product([a,b,c])`
    add_puzzles_to_catalog(catalog)?;
    add_twist_systems_to_catalog(catalog)?;
    add_color_systems_to_catalog(catalog)?;
    Ok(())
}

fn add_puzzles_to_catalog(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    let mut product_tags = TagSet::new();
    product_tags.insert_named("generator", true.into())?;
    product_tags.insert_named("doctrinaire", true.into())?;
    product_tags.insert_named("pseudodoctrinaire", true.into())?;
    product_tags.insert_named("ndim/generic", true.into())?;

    let params = vec![GeneratorParam {
        name: "Factors".to_string(),
        ty: GeneratorParamType::List(Box::new(GeneratorParamType::Puzzle {
            menu: "symmetric".to_string(),
        })),
        default: CatalogIdValue::List(vec![
            "ngon_ft_shallow(5,3)".parse().unwrap(),
            "line(3)".parse().unwrap(),
        ]),
    }];

    // let rot: CatalogWord = ROT_ID.parse().expect("bad subset ID");
    // let refle: CatalogWord = REFLE_ID.parse().expect("bad subset ID");
    // let subset_param = GeneratorSubsetParam {
    //     options: vec![
    //         GeneratorSubsetParamValue {
    //             id: rot.clone(),
    //             name_prefix: ROT_NAME_PREFIX.to_string(),
    //         },
    //         GeneratorSubsetParamValue {
    //             id: refle.clone(),
    //             name_prefix: REFLE_NAME_PREFIX.to_string(),
    //         },
    //     ],
    //     default: Some(rot),
    //     maximal: Some(refle),
    // };

    catalog.add::<PuzzleListEntry>(Arc::new(Generator {
        id: product_base_id(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: true },
        generate: Box::new(move |build_ctx| {
            if build_ctx.id().args().is_empty() {
                Ok(Arc::new(PuzzleListEntry {
                    id: build_ctx.id().clone(),
                    version: None,
                    name: "Product".to_string(),
                    aliases: vec![],
                    tags: product_tags.clone(),
                }))
            } else {
                let factors =
                    build_ctx.build_list_blocking::<PuzzleListEntry>(&build_ctx.id().args()[0])?;

                Ok(Arc::new(PuzzleListEntry {
                    id: build_ctx.id().clone(),
                    version: None,
                    name: factors.iter().map(|f| &f.name).join(" × "),
                    aliases: vec![],
                    tags: make_puzzle_tag_set(
                        factors.iter().map(|factor| factor.tags.ndim()).sum(),
                    )?,
                }))
            }
        }),
    }))?;

    catalog.add::<PuzzleProduct>(Arc::new(Generator {
        id: product_base_id(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(|build_ctx| {
            build_ctx
                .build_list_blocking::<PuzzleProduct>(&build_ctx.id().args()[0])?
                .into_iter()
                .try_fold(PuzzleProduct::direct_product_identity(), |a, b| {
                    a.direct_product(&b)
                })
                .map(Arc::new)
        }),
    }))?;

    catalog.add::<Puzzle>(Arc::new(Generator {
        id: product_base_id(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(build_product_puzzle_impl),
    }))?;

    catalog.add_to_puzzle_list(&CatalogId::new(product_base_id(), vec![], None));

    Ok(())
}

fn build_product_puzzle_impl(build_ctx: BuildCtx) -> Result<Arc<Puzzle>> {
    let meta = build_ctx.build_blocking::<PuzzleListEntry>(build_ctx.id())?;
    let puzzle_product = build_ctx.build_blocking::<PuzzleProduct>(build_ctx.id())?;
    let colors = match puzzle_product.colors_id() {
        Some(colors_id) => build_ctx.build_blocking::<ColorSystem>(&colors_id)?,
        None => puzzle_product.build_ad_hoc_color_system()?,
    };
    let twists = match puzzle_product.twists_id() {
        Some(twists_id) => build_ctx.build_blocking::<TwistSystem>(&twists_id)?,
        None => TwistSystemProduct::new_empty(puzzle_product.ndim())
            .build(&build_ctx, &mut build_ctx.warn_fn())?,
    };
    puzzle_product.build(&build_ctx, meta, colors, twists, &mut build_ctx.warn_fn())
}

fn add_twist_systems_to_catalog(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    let params = vec![GeneratorParam {
        name: "Factors".to_string(),
        ty: GeneratorParamType::List(Box::new(GeneratorParamType::Id {
            ty: TwistSystemProduct::catalog_type_name(),
        })),
        default: CatalogIdValue::List(vec![]),
    }];

    catalog.add::<TwistSystemProduct>(Arc::new(Generator {
        id: product_base_id(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(|build_ctx| {
            build_ctx
                .build_list_blocking::<TwistSystemProduct>(&build_ctx.id().args()[0])?
                .into_iter()
                .try_fold(TwistSystemProduct::direct_product_identity(), |a, b| {
                    a.direct_product(&b)
                })
                .map(Arc::new)
        }),
    }))?;

    catalog.add::<TwistSystem>(Arc::new(Generator {
        id: product_base_id(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(|build_ctx| {
            build_ctx
                .build_blocking::<TwistSystemProduct>(build_ctx.id())?
                .build(&build_ctx, &mut build_ctx.warn_fn())
        }),
    }))?;

    catalog.add::<TwistSystemProduct>(Arc::new(Generator {
        id: "empty@1".parse().expect("bad catalog ID"),
        params: vec![GeneratorParam {
            name: "Dimensions".to_string(),
            ty: GeneratorParamType::Int { min: 1, max: 8 },
            default: "1".parse().expect("bad param default"),
        }],
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(|build_ctx| {
            Ok(Arc::new(TwistSystemProduct::new_factor(
                &FactorTwistSystemSpec {
                    id: build_ctx.id().clone(),
                    name: "Empty".to_string(),
                    ndim: build_ctx.id().args()[0].to_int()?.try_into()?,
                    coxeter_matrix: None,
                    axis_orbits: vec![],
                    named_point_orbits: vec![],
                    named_point_set_orbits: vec![],
                    stabilizer_twist_orbits: vec![],
                    jumble_moves: vec![],
                    jumble_stops: vec![],
                },
                &mut build_ctx.warn_fn(),
            )?))
        }),
    }))?;

    Ok(())
}

fn add_color_systems_to_catalog(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    let params = vec![GeneratorParam {
        name: "Terms".to_string(),
        ty: GeneratorParamType::List(Box::new(GeneratorParamType::Id {
            ty: ColorSystem::catalog_type_name(),
        })),
        default: CatalogIdValue::List(vec![]),
    }];

    catalog.add::<ColorSystem>(Arc::new(Generator {
        id: disjoint_union_base_id(),
        params,
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(|build_ctx| {
            build_ctx
                .build_list_blocking::<ColorSystem>(&build_ctx.id().args()[0])?
                .into_iter()
                .try_fold(
                    ColorSystemDisjointUnion::disjoint_union_identity(),
                    |a, b| a.disjoint_union(&ColorSystemDisjointUnion::from_factor_color_system(b)),
                )?
                .build()
        }),
    }))?;

    Ok(())
}

#[derive(Debug, Clone)]
struct Attitude {
    jumble_offset: Option<Motor>,
    element: GroupElementId,
}

impl Default for Attitude {
    fn default() -> Self {
        Self::from(GroupElementId::IDENTITY)
    }
}

impl From<GroupElementId> for Attitude {
    fn from(value: GroupElementId) -> Self {
        Self {
            jumble_offset: None,
            element: value,
        }
    }
}

impl Attitude {
    fn compose(&self, other: &Self, group: &hypergroup::IsometryGroup) -> Self {
        let j1 = self.jumble_offset.as_ref();
        let j2 = other.jumble_offset.as_ref();

        // j1 e1 j2 e2
        // = (j1 (e1 j2 e1')) (e1 e2)
        let mut jumble_offset = hypermath::util::merge_options(
            j1.cloned(),
            j2.map(|j| group.motor(self.element).transform(j)),
            |a, b| a * b,
        );
        let mut element = group.compose(self.element, other.element);
        if let Some(j) = &jumble_offset
            && let Some(unjumbled) = group.element_from_motor(j)
        {
            // simplify!
            jumble_offset = None;
            element = group.compose(unjumbled, element);
        }
        Self {
            jumble_offset,
            element,
        }
    }

    fn motor(&self, group: &hypergroup::IsometryGroup) -> Motor {
        match &self.jumble_offset {
            Some(j) => j * group.motor(self.element),
            None => group.motor(self.element),
        }
    }

    fn inverse(&self, group: &hypergroup::IsometryGroup) -> Self {
        // (j1 e1)'
        // = e1' j1'
        // = (e1' j1' e1) e1'
        Self {
            jumble_offset: self
                .jumble_offset
                .as_ref()
                .map(|j| group.motor(self.element).reverse().transform(&j.reverse())),
            element: group.inverse(self.element),
        }
    }
}

/// Instance of a product puzzle with a particular state.
#[derive(Debug, Clone)]
pub struct ProductPuzzleState {
    ty: Arc<Puzzle>,
    twists: Arc<SymmetricTwistSystemComponent>,
    piece_grip_signatures: Arc<PerPiece<PerAxis<Option<LayerRange>>>>,
    piece_points: Arc<PerPiece<Vec<Point>>>,
    axis_layer_ranges: Arc<PerAxis<PerLayer<[Float; 2]>>>,
    axis_vectors: Arc<NdEuclidAxisVectors>,
    /// Current jumble stop for each layer on each axis, or `None` if this
    /// puzzle has no jumbling.
    axis_jumble_states: Option<PerAxis<PerLayer<JumbleStop>>>,
    piece_attitudes: PerPiece<Attitude>,
}

impl PuzzleState for ProductPuzzleState {
    fn ty(&self) -> &Arc<Puzzle> {
        &self.ty
    }

    fn clone_dyn(&self) -> BoxDynPuzzleState {
        self.clone().into()
    }

    fn do_twist(&self, twist: &Move) -> Result<Self, TwistError>
    where
        Self: Sized,
    {
        let (axis, element, jumble_offset) = self
            .twists
            .resolve_twist(twist)
            .map_err(|_| TwistError::Unknown)?;
        let attitude_delta = Attitude {
            element,
            jumble_offset: jumble_offset.clone().map(|(m, _)| m),
        };

        let layer_mask = twist.layers.to_layer_mask(self.ty.axis_layers[axis]);
        let pieces_affected = self.compute_grip(axis, &layer_mask);
        let blocking_pieces = pieces_affected
            .iter_filter(|_piece, &which_side| which_side == WhichSide::Split)
            .collect_vec();
        if !blocking_pieces.is_empty() {
            return Err(TwistError::Blocked(blocking_pieces));
        }

        let mut ret = self.clone();

        let (_, axis_orbit) = self.twists.axis_undeorbiters[axis];
        if let Some(axis_jumble_states) = &mut ret.axis_jumble_states
            && let Some((_, angle_delta)) = jumble_offset
            && let Some(jumble_data) = &self.twists.axis_orbits[axis_orbit].jumble_data
        {
            for layer in &layer_mask {
                let old_stop = axis_jumble_states[axis][layer];
                let old_angle = jumble_data.stops[old_stop].angle;
                let new_angle = old_angle + angle_delta;
                let new_stop = jumble_data
                    .factor_exact(new_angle)
                    .ok_or(TwistError::MissesJumbleStop(layer))?;
                // TODO: check per-layer validity

                // if !jumble_data.stops[new_stop].layer_mask.contains(layer) {
                //     return Err(TwistError::MissesJumbleStop(layer));
                // }
                axis_jumble_states[axis][layer] = new_stop;
            }
        }

        for (piece, which_side) in pieces_affected {
            if which_side == WhichSide::Inside {
                ret.piece_attitudes[piece] =
                    attitude_delta.compose(&ret.piece_attitudes[piece], &self.twists.group);
            }
        }
        Ok(ret)
    }

    fn do_twist_dyn(&self, twist: &Move) -> Result<BoxDynPuzzleState, TwistError> {
        self.do_twist(twist).map(BoxDynPuzzleState::new)
    }

    fn is_solved(&self) -> bool {
        true // TODO
    }

    fn compute_grip(&self, axis: Axis, layers: &LayerMask) -> PerPiece<WhichSide> {
        self.piece_attitudes.map_ref(
            |piece, _| match self.piece_layer_range_on_axis(piece, axis) {
                (piece_layers, piece_is_outside_layers) => WhichSide::from_points(
                    piece_layers
                        .into_iter()
                        .map(|l| {
                            if layers.contains(l) {
                                PointWhichSide::Inside
                            } else {
                                PointWhichSide::Outside
                            }
                        })
                        .chain(piece_is_outside_layers.then_some(PointWhichSide::Outside)),
                ),
            },
        )
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        let (layer_mask, outside_layers) = self.piece_layer_range_on_axis(piece, axis);
        (!outside_layers).then_some(layer_mask)
    }

    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        self.min_layer_mask(axis, piece) // no blocked layers
    }

    fn render_data(&self) -> BoxDynPuzzleStateRenderData {
        NdEuclidPuzzleStateRenderData {
            piece_transforms: self
                .piece_attitudes
                .map_ref(|_, e| e.motor(&self.twists.group)),
        }
        .into()
    }

    fn animated_render_data(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData {
        let anim = anim
            .downcast_ref::<NdEuclidPuzzleAnimation>()
            .expect("expected NdEuclidPuzzleAnimation");
        let m = if t == 0.0 {
            anim.initial_transform.clone()
        } else if t == 1.0 {
            anim.final_transform.clone()
        } else {
            pga::Motor::slerp_infallible(&anim.initial_transform, &anim.final_transform, t as _)
        };

        NdEuclidPuzzleStateRenderData {
            piece_transforms: self.partial_twist_piece_transforms(&anim.pieces, &m),
        }
        .into()
    }
}

impl ProductPuzzleState {
    /// Returns the attitude of each piece.
    fn piece_transforms(&self) -> PerPiece<pga::Motor> {
        self.piece_attitudes
            .map_ref(|_, e| e.motor(&self.twists.group))
    }

    /// Returns piece transforms for a partial twist.
    fn partial_twist_piece_transforms(
        &self,
        grip: &PieceMask,
        transform: &pga::Motor,
    ) -> PerPiece<pga::Motor> {
        let mut piece_transforms = self.piece_transforms();
        for piece in grip.iter() {
            piece_transforms[piece] = transform * &piece_transforms[piece];
        }
        piece_transforms
    }

    /// Returns the set of layers on the axis that contain any piece geometry,
    /// and a boolean indicating whether the piece contains any geometry outside
    /// the axis layers.
    fn piece_layer_range_on_axis(&self, piece: Piece, axis: Axis) -> (LayerMask, bool) {
        let attitude = &self.piece_attitudes[piece];
        let inverse_attitude = attitude.motor(&self.twists.group).reverse();
        let Some(transformed_axis_vector) = inverse_attitude
            .transform(&self.axis_vectors.vectors_by_id[axis])
            .normalize()
        else {
            return Default::default(); // bad axis vector
        };
        let Some((min, mut max)) = self.piece_points[piece]
            .iter()
            .map(|p| transformed_axis_vector.dot(p.as_vector()))
            .minmax_float()
            .into_option()
        else {
            return Default::default(); // no geometry
        };

        layers_containing_range(&self.axis_layer_ranges[axis], min, max)
    }
}

fn named_point_autonames() -> impl Iterator<Item = String> {
    (0..)
        .map(hypuz_notation::family::SequentialUppercaseName)
        .map(|name| format!("ZZ{name}"))
}

fn lift_vector_by_ndim<V: FromIterator<Float>>(
    v: impl VectorRef,
    ndim_below: u8,
    v_ndim: u8,
    ndim_above: u8,
) -> V {
    let below = std::iter::repeat_n(0.0, ndim_below as usize);
    let above = std::iter::repeat_n(0.0, ndim_above as usize);
    itertools::chain!(below.clone(), v.iter_ndim(v_ndim), above.clone()).collect()
}

fn lift_hyperplane_by_ndim(
    h: &Hyperplane,
    ndim_below: u8,
    h_ndim: u8,
    ndim_above: u8,
) -> Result<Hyperplane> {
    let normal: Vector = lift_vector_by_ndim(h.normal(), ndim_below, h_ndim, ndim_above);
    Hyperplane::new(normal, h.distance()).ok_or_eyre("error lifting hyperplane")
}

fn shuffle_group_generators(
    group: &hypergroup::IsometryGroup,
    mut rng: impl rand::Rng,
) -> Result<hypergroup::IsometryGroup> {
    use rand::RngExt;

    const SHUFFLE_ITERATIONS: usize = 100;

    if group.generators().len() < 2 {
        return Ok(group.clone());
    }

    // TODO: use motors from original group for better numerical stability
    // TODO: add more generators, especially for polygons
    let mut generators = group.generator_motors().to_vec();
    for _ in 0..SHUFFLE_ITERATIONS {
        let i = rng.random_range(0..generators.len());
        let mut j = rng.random_range(0..generators.len() - 1);
        if j >= i {
            j += 1;
        }
        generators[i] = (&generators[i] * &generators[j])
            .canonicalize()
            .ok_or_eyre("error canonicalizing motor")?;
    }
    Ok(hypergroup::IsometryGroup::from_generators(
        group.abstract_group().label(),
        hypergroup::PerGenerator::from(generators),
    )?)
}

fn chain_cloned<'a, T: 'a + Clone, B: FromIterator<T>>(
    a: impl IntoIterator<Item = &'a T>,
    b: impl IntoIterator<Item = &'a T>,
) -> B {
    std::iter::chain(a, b).cloned().collect()
}

/// Returns the minimal set of layers that fully contains the range `min..=max`,
/// and a boolean indicating whether any portion of the range is not contained
/// in any layer.
fn layers_containing_range(
    axis_layers: &PerLayer<[Float; 2]>,
    range_min: Float,
    mut range_max: Float,
) -> (LayerMask, bool) {
    let mut mask = LayerMask::new();
    let mut outside_any_layer = false;

    // Iterate from outermost (greatest) to innermost (least)
    for (layer, &[layer_max, layer_min]) in axis_layers {
        // Cover `layer_max..=range_max`
        outside_any_layer |= APPROX.lt(layer_max, range_max);

        // Exit if this layer is completely below the range
        if APPROX.lt_eq(layer_max, range_min) {
            return (mask, outside_any_layer);
        }

        // Cover `layer_min..=layer_max`
        if APPROX.lt(layer_min, range_max) {
            mask.insert(layer);
            // We've covered `layer_min..=range_max`, so the remaining range is `range_min..=layer_min`
            range_max = layer_min;
        }

        // Exit if we have covered all of the range
        if APPROX.lt_eq(range_max, range_min) {
            return (mask, outside_any_layer);
        }
    }

    (mask, true)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const TEST_LAYER_COUNT: usize = 10;

    proptest! {
        #[test]
        fn proptest_layers_containing_range(
            mut range in [0..TEST_LAYER_COUNT, 0..TEST_LAYER_COUNT],
            layers_bitmask in 0..(1_u64 << TEST_LAYER_COUNT),
        ) {
            prop_assume!(range[0] != range[1]);
            range.sort();
            test_layers_containing_range(range, layers_bitmask);
        }
    }

    fn test_layers_containing_range([range_min, range_max]: [usize; 2], layers_bitmask: u64) {
        let mut layers = PerLayer::new();
        let mut expected_layer_mask = LayerMask::new();
        let mut expected_bool = false;
        for i in (0..TEST_LAYER_COUNT).rev() {
            if layers_bitmask & (1 << i) != 0 {
                let layer = layers.push([(i + 1) as Float, i as Float]).unwrap();
                if (range_min..range_max).contains(&i) {
                    expected_layer_mask.insert(layer);
                }
            } else if (range_min..range_max).contains(&i) {
                expected_bool = true;
            }
        }
        let actual_result =
            layers_containing_range(&layers, range_min as Float, range_max as Float);
        assert_eq!((expected_layer_mask, expected_bool), actual_result);
    }

    #[test]
    fn test_repro() {
        test_layers_containing_range([0, 1], 5);
    }
}

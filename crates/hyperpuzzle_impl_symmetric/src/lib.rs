//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::sync::Arc;

use eyre::{OptionExt, Result};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::{catalog::VersionedCatalogWord, group::GroupElementId};
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleAnimation, NdEuclidPuzzleStateRenderData};

mod builder;
mod cut_distances;
mod geometry;
pub mod hps;
mod named_point;
mod spec;
mod stabilizer_family;
mod twist_system;

use builder::PuzzleProduct;
pub use cut_distances::CutDistances;
use itertools::Itertools;
pub use named_point::{NamedPoint, NamedPointSet, PerNamedPoint};
pub use spec::*;
pub use stabilizer_family::StabilizerFamily;
pub use twist_system::{
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent, UniqueMinimalClockwiseGenerator,
};

use crate::builder::{ColorSystemDisjointUnion, TwistSystemProduct};

const PRODUCT_ID: &str = "product@1";
const DISJOINT_UNION_ID: &str = "sum@1";
const ROT_ID: &str = "rot";
const REFLE_ID: &str = "refle";

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
        PRODUCT_ID.parse::<CatalogWord>().expect("bad catalog ID"),
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
        DISJOINT_UNION_ID
            .parse::<CatalogWord>()
            .expect("bad catalog ID"),
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

    let id: VersionedCatalogWord = PRODUCT_ID.parse().expect("bad catalog ID");
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
        id: id.clone(),
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
        id: id.clone(),
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
        id: id.clone(),
        params: params.clone(),
        subset_param: None,
        validation: GeneratorParamValidation { allow_empty: false },
        generate: Box::new(build_product_puzzle_impl),
    }))?;

    catalog.add_to_puzzle_list(&CatalogId::new(id.clone(), vec![], None));

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
    let id: VersionedCatalogWord = PRODUCT_ID.parse().expect("bad catalog ID");
    let params = vec![GeneratorParam {
        name: "Factors".to_string(),
        ty: GeneratorParamType::List(Box::new(GeneratorParamType::Id {
            ty: TwistSystemProduct::catalog_type_name(),
        })),
        default: CatalogIdValue::List(vec![]),
    }];

    catalog.add::<TwistSystemProduct>(Arc::new(Generator {
        id: id.clone(),
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
        id: id.clone(),
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
                },
                &mut build_ctx.warn_fn(),
            )?))
        }),
    }))?;

    Ok(())
}

fn add_color_systems_to_catalog(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    let id: VersionedCatalogWord = DISJOINT_UNION_ID.parse().expect("bad catalog ID");
    let params = vec![GeneratorParam {
        name: "Terms".to_string(),
        ty: GeneratorParamType::List(Box::new(GeneratorParamType::Id {
            ty: ColorSystem::catalog_type_name(),
        })),
        default: CatalogIdValue::List(vec![]),
    }];

    catalog.add::<ColorSystem>(Arc::new(Generator {
        id,
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

/// Instance of a product puzzle with a particular state.
#[derive(Debug, Clone)]
pub struct ProductPuzzleState {
    ty: Arc<Puzzle>,
    twists: Arc<SymmetricTwistSystemComponent>,
    piece_grip_signatures: Arc<PerPiece<PerAxis<Option<LayerRange>>>>,
    piece_attitudes: PerPiece<GroupElementId>, // TODO: consider storing inverse
}

impl PuzzleState for ProductPuzzleState {
    fn ty(&self) -> &std::sync::Arc<Puzzle> {
        &self.ty
    }

    fn clone_dyn(&self) -> BoxDynPuzzleState {
        self.clone().into()
    }

    fn do_twist(&self, twist: &Move) -> std::result::Result<Self, Vec<Piece>>
    where
        Self: Sized,
    {
        let (axis, transform) = self.twists.resolve_twist(twist).map_err(|_| vec![])?;
        let layer_mask = twist.layers.to_layer_mask(self.ty.axis_layers[axis]);
        let mut ret = self.clone();
        for (piece, which_side) in self.compute_grip(axis, &layer_mask) {
            if which_side == WhichSide::Inside {
                ret.piece_attitudes[piece] = self
                    .twists
                    .group
                    .compose(transform, ret.piece_attitudes[piece]);
            }
        }
        Ok(ret)
    }

    fn do_twist_dyn(&self, twist: &Move) -> std::result::Result<BoxDynPuzzleState, Vec<Piece>> {
        self.do_twist(twist).map(BoxDynPuzzleState::new)
    }

    fn is_solved(&self) -> bool {
        true // TODO
    }

    fn compute_grip(&self, axis: Axis, layers: &LayerMask) -> PerPiece<WhichSide> {
        self.piece_attitudes.map_ref(|piece, _| {
            match self.piece_layer_range_on_axis(piece, axis) {
                Some(range) => WhichSide::from_points(range.into_iter().map(|l| {
                    if layers.contains(l) {
                        PointWhichSide::Inside
                    } else {
                        PointWhichSide::Outside
                    }
                })),
                None => WhichSide::Split, // axis is entirely blocked
            }
        })
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        Some(LayerMask::from_range(
            self.piece_layer_range_on_axis(piece, axis)?,
        ))
    }

    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        self.min_layer_mask(axis, piece) // no blocked layers
    }

    fn render_data(&self) -> BoxDynPuzzleStateRenderData {
        NdEuclidPuzzleStateRenderData {
            piece_transforms: self
                .piece_attitudes
                .map_ref(|_, &e| self.twists.group.motor(e)),
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
            .map_ref(|_, &e| self.twists.group.motor(e))
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

    fn piece_layer_range_on_axis(&self, piece: Piece, axis: Axis) -> Option<LayerRange> {
        let attitude = self.piece_attitudes[piece];
        let inverse_attitude = self.twists.group.inverse(attitude);
        self.piece_grip_signatures[piece][self.twists.axis_action.act(inverse_attitude, axis)]
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

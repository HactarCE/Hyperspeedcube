//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.
#![allow(unused)]

use std::sync::Arc;

use eyre::{OptionExt, Result};
use hypergroup::{AbbrGenSeq, GeneratorId};
use hypermath::prelude::*;
use hyperpuzzle_core::group::{CoxeterMatrix, GroupElementId};
use hyperpuzzle_core::{TAGS, prelude::*};
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleAnimation, NdEuclidPuzzleStateRenderData};

mod builder;
mod geometry;
pub mod hps;
mod named_point;
mod names;
mod spec;
mod stabilizer_family;
mod twist_system;

use builder::ProductPuzzleBuilder;
use itertools::Itertools;
pub use named_point::{NamedPoint, NamedPointSet, PerNamedPoint};
pub use spec::{AxisOrbitSpec, FactorPuzzleSpec, NamedPointOrbitSpec, ProductPuzzleSpec};
pub use stabilizer_family::StabilizerFamily;
pub use twist_system::{
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemEngineData, UniqueMinimalClockwiseGenerator,
};

const PRODUCT_ID: &str = "product";

fn product_id(factor_ids: &[CatalogId]) -> CatalogId {
    CatalogId::new(
        crate::PRODUCT_ID,
        factor_ids.iter().map(|id| id.clone().into()),
    )
    .expect("product ID is invalid")
}

pub fn add_puzzles_to_catalog(catalog: &hyperpuzzle_core::CatalogBuilder) -> Result<()> {
    let mut product_tags = TagSet::new();
    product_tags.insert_named("type/generator", true.into())?;
    product_tags.insert_named("algebraic/doctrinaire", true.into())?;

    // catalog.add_puzzle_generator(Arc::new(PuzzleGenerator {
    //     meta: Arc::new(CatalogMetadata {
    //         id: CatalogId {
    //             base: "product".into(),
    //             args: vec![],
    //         },
    //         version: Version {
    //             major: 1,
    //             minor: 0,
    //             patch: 0,
    //         },
    //         name: "Puzzle Product".into(),
    //         aliases: vec![],
    //         tags: product_tags.clone(),
    //     }),
    //     params: vec![GeneratorParam],
    //     generate_meta: todo!(),
    //     generate: todo!(),
    // }));

    // catalog.add_puzzle_generator(Arc::new(PuzzleGenerator {
    //     meta: Arc::new(CatalogMetadata {
    //         id: CatalogId {
    //             base: "refleproduct".into(),
    //             args: vec![],
    //         },
    //         version: Version {
    //             major: 1,
    //             minor: 0,
    //             patch: 0,
    //         },
    //         name: "Reflection Puzzle Product",
    //         aliases: (),
    //         tags: (),
    //     }),
    //     params: todo!(),
    //     generate_meta: todo!(),
    //     generate: todo!(),
    // }));

    catalog.add_puzzle_generator(Arc::new(PuzzleGenerator::new_lazy_constant(
        Arc::new(CatalogMetadata {
            id: CatalogId::new("symmetric_puzzle_test", vec![]).unwrap(),
            version: Version {
                major: 0,
                minor: 0,
                patch: 1,
            },
            name: "Symmetric Puzzle Test".to_string(),
            aliases: vec![],
            tags: TagSet::new(),
        }),
        Box::new(|build_ctx| {
            // IIFE to mimic try_block
            (|| -> Result<_> {
                let mut warn_fn = |e| eprintln!("{e:#}");
                let t = std::time::Instant::now();
                let ret = ProductPuzzleBuilder::new(
                    &ProductPuzzleSpec {
                        factors: vec![
                            // ft_cube(4)?,
                            megaminx()?,
                            // shallow_polygon(20)?,
                            // shallow_polygon(7)?,
                            shallow_line()?,
                            // shallow_line()?,
                            // shallow_line()?,
                            // shallow_polygon(4)?,
                            // shallow_ft_simplex(3)?,
                            // ft_120_cell_shallow()?,
                            // ft_120_cell_evil()?,
                            // ft_120_cell(vec![])?,
                        ],
                    },
                    &mut warn_fn,
                )?
                .build(Some(&build_ctx), &mut warn_fn); // TODO: better warn function
                ret
            })()
            .map(Redirectable::Direct)
        }),
    )))?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ProductPuzzleState {
    ty: Arc<Puzzle>,
    twists: Arc<SymmetricTwistSystemEngineData>,
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

fn autonames() -> impl Iterator<Item = String> {
    (0..)
        .map(hypuz_notation::family::SequentialUppercaseName)
        .map(|prefix| prefix.to_string())
}

fn autoname_orbit(sym: &hypergroup::IsometryGroup, point: Vector) -> Vec<(AbbrGenSeq, String)> {
    hypergroup::orbit_geometric_with_gen_seq(
        &sym.generator_motors()
            .iter()
            .map(|(g, m)| (hypergroup::GenSeq::new([g]), m.clone()))
            .collect_vec(),
        point,
    )
    .into_iter()
    .map(|(gen_seq, _, _)| gen_seq)
    .zip(crate::autonames())
    .collect()
}

const INF: Float = Float::INFINITY;

fn ft_cube(ndim: u8) -> Result<FactorPuzzleSpec> {
    if ndim > 5 {
        unimplemented!();
    }

    let facet_names = vec![
        (5, "A", 5),
        (4, "O", 4),
        (3, "F", 3),
        (2, "U", 2),
        (1, "R", 1),
        (1, "L", 0),
        (2, "D", 1),
        (3, "B", 2),
        (4, "I", 3),
        (5, "P", 4),
    ]
    .into_iter()
    .filter(|&(n, _, _)| n <= ndim)
    .enumerate()
    .map(|(i, (_, name, g))| {
        let gen_seq = if i == 0 {
            AbbrGenSeq::INIT
        } else {
            AbbrGenSeq::new([GeneratorId(g)], Some(i - 1))
        };
        (gen_seq, name.to_string())
    })
    .collect();

    const GIZMO_EDGE_FACTOR: f64 = 0.8;
    let edge_pole_distance = (1.0 + GIZMO_EDGE_FACTOR) / 2.0_f64.sqrt();
    let corner_pole_distance = (1.0 + 2.0 * GIZMO_EDGE_FACTOR) / 3.0_f64.sqrt();
    let s = hypuz_notation::Str::from_static_str;

    let mut named_point_set_orbits = vec![];
    if (2..=4).contains(&ndim) {
        named_point_set_orbits.push((vec![s("R"), s("U")], edge_pole_distance));
    }
    if (3..=4).contains(&ndim) {
        named_point_set_orbits.push((vec![s("R"), s("U"), s("F")], edge_pole_distance));
    }

    let facet_points = NamedPointOrbitSpec::new(Vector::unit(ndim - 1), facet_names);

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("ft_cube", vec![(ndim as i64).into()]).unwrap(),
        CatalogId::new("cube", vec![(ndim as i64).into()]).unwrap(),
        CoxeterMatrix::B(ndim)?,
        vec![facet_points.to_axes(
            vec![INF, 1.0 / 3.0, -1.0 / 3.0, -INF],
            if ndim == 4 {
                vec![
                    (vec![s("R")], 1.0),
                    (vec![s("R"), s("U")], edge_pole_distance),
                    (vec![s("R"), s("U"), s("F")], corner_pole_distance),
                ]
            } else {
                vec![]
            },
        )],
        vec![facet_points],
        named_point_set_orbits,
    ))
}

fn shallow_polygon(n: u16) -> Result<FactorPuzzleSpec> {
    let names = (0..n)
        .map(|i| {
            let name = hypuz_notation::family::SequentialUppercaseName(i as u32).to_string();
            let gen_seq = if i == 0 {
                AbbrGenSeq::INIT
            } else {
                AbbrGenSeq::new([1, 0].map(GeneratorId), Some(i as usize - 1))
            };
            (gen_seq, name)
        })
        .collect();

    let pi_div_n = std::f64::consts::PI as Float / n as Float;
    let half_edge_length = pi_div_n.tan();
    let edge_length = 2.0 * half_edge_length;
    let edge_height = (2.0 * pi_div_n).sin() * edge_length;
    let circumradius = pi_div_n.cos().recip();
    let cut_depth = 1.0 - edge_height / 3.0;

    const FACET_GIZMO_EDGE_FACTOR: f64 = 2.0 / 3.0;
    let s = hypuz_notation::Str::from_static_str;

    let edge_points = NamedPointOrbitSpec::new(Vector::unit(1) / half_edge_length, names);

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("shallow_polygon", vec![(n as i64).into()]).unwrap(),
        CatalogId::new("polygon", vec![(n as i64).into()]).unwrap(),
        CoxeterMatrix::I(n)?,
        vec![edge_points.to_axes(
            vec![INF, cut_depth / half_edge_length],
            vec![(vec![s("B")], 1.0)],
        )],
        vec![edge_points],
        vec![(
            vec![s("A"), s("B")],
            hypermath::util::lerp(circumradius, 1.0, FACET_GIZMO_EDGE_FACTOR) / half_edge_length,
        )],
    ))
}

fn shallow_line() -> Result<FactorPuzzleSpec> {
    line(vec![INF, 1.0 / 3.0, -1.0 / 3.0, -INF])
}

fn half_cut_line() -> Result<FactorPuzzleSpec> {
    line(vec![INF, 0.0, -INF])
}

fn line(cut_distances: Vec<Float>) -> Result<FactorPuzzleSpec> {
    let names = vec![
        (AbbrGenSeq::INIT, "A".to_string()),
        (AbbrGenSeq::new([GeneratorId(0)], Some(0)), "B".to_string()),
    ];

    let layer_count = cut_distances.len().saturating_sub(1);

    let vertex_points = NamedPointOrbitSpec::new(Vector::unit(0), names);

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("line", vec![(layer_count as i64).into()]).unwrap(),
        CatalogId::new("line", vec![]).unwrap(),
        CoxeterMatrix::A(1)?,
        vec![vertex_points.to_axes(cut_distances, vec![])],
        vec![vertex_points],
        vec![],
    ))
}

fn megaminx() -> Result<FactorPuzzleSpec> {
    let names = vec![
        ("F", None, None),
        ("U", Some(2), Some(0)),
        ("R", Some(1), Some(1)),
        ("L", Some(0), Some(2)),
        ("DR", Some(1), Some(3)),
        ("DL", Some(0), Some(4)),
        ("BR", Some(2), Some(4)),
        ("BL", Some(2), Some(5)),
        ("PR", Some(1), Some(7)),
        ("PL", Some(0), Some(8)),
        ("PD", Some(1), Some(9)),
        ("PB", Some(2), Some(10)),
    ]
    .into_iter()
    .map(|(name, g, end)| {
        let gen_seq = AbbrGenSeq::new(g.map(GeneratorId), end);
        (gen_seq, name.to_string())
    })
    .collect();

    let cut_distance = std::f64::consts::GOLDEN_RATIO.recip();

    const FACET_GIZMO_EDGE_FACTOR: f64 = 2.0 / 3.0;
    let s = hypuz_notation::Str::from_static_str;

    let facet_points = NamedPointOrbitSpec::new(Vector::unit(2), names);

    // let mul = (40.0 / (25.0 + 11.0 * 5.0_f64.sqrt())).sqrt();
    let inradius = ((25.0 + 11.0 * 5.0_f64.sqrt()) / 40.0).sqrt();
    let edge_radius = (3.0 + 5.0_f64.sqrt()) / 4.0;
    let circumradius = (3.0_f64.sqrt() + 15.0_f64.sqrt()) / 4.0;

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("ft_dodecahedron", vec![(1_i64).into()]).unwrap(),
        CatalogId::new("dodecahedron", vec![]).unwrap(),
        CoxeterMatrix::H3(),
        vec![facet_points.to_axes(
            vec![INF, cut_distance],
            vec![
                (vec![s("U")], std::f64::consts::GOLDEN_RATIO.recip()), // TODO: prove this number
                (vec![s("U"), s("R")], 0.65),                           // TODO: tweak number
            ],
        )],
        vec![facet_points],
        vec![
            (
                vec![s("F"), s("U")],
                (edge_radius / inradius) * 0.94, // TODO: what is maximum value?
            ),
            (
                vec![s("F"), s("U"), s("R")],
                (circumradius / inradius) * 0.92, // TODO: what is maximum value?
            ),
        ],
    ))
}

use std::f64::consts::GOLDEN_RATIO as PHI;

fn ft_120_cell_shallow() -> Result<FactorPuzzleSpec> {
    const SHALLOW_CUT_DEPTH: Float = 3.0 / 2.0 * (1.0 / PHI);
    // ft_120_cell(vec![INF, 0.95])
    ft_120_cell(vec![INF, SHALLOW_CUT_DEPTH])
}

fn ft_120_cell_evil() -> Result<FactorPuzzleSpec> {
    const EVIL_CUT_DEPTH: Float = 1.0 / PHI;
    ft_120_cell(vec![INF, EVIL_CUT_DEPTH])
}

fn ft_120_cell(cut_distances: Vec<Float>) -> Result<FactorPuzzleSpec> {
    let coxeter_matrix = CoxeterMatrix::H4();
    let isometry_group = coxeter_matrix.isometry_group()?;
    let axis_vector = (coxeter_matrix.mirror_basis()? * vector![0.0, 0.0, 0.0, 1.0])
        .normalize()
        .unwrap();
    let names = autoname_orbit(&isometry_group, axis_vector.clone());

    let layer_count = cut_distances.len().saturating_sub(1);

    let gizmo_facet_size = (std::f64::consts::PI / 10.0).tan();
    let s = hypuz_notation::Str::from_static_str;

    let facet_points = NamedPointOrbitSpec::new(axis_vector, names);

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("ft_120cell", vec![(layer_count as i64).into()]).unwrap(),
        CatalogId::new("120cell", vec![]).unwrap(),
        coxeter_matrix,
        vec![facet_points.to_axes(
            cut_distances,
            vec![
                // TODO: this is definitely wrong lmao
                (vec![s("B")], gizmo_facet_size),
                // (vec![s("R"), s("U")], edge_pole_distance),
                // (vec![s("R"), s("U"), s("F")], corner_pole_distance),
            ],
        )],
        vec![facet_points],
        // vec![
        //     (vec![s("R"), s("U")], edge_pole_distance),
        //     (vec![s("R"), s("U"), s("F")], edge_pole_distance),
        // ],
        vec![],
    ))
}

fn shallow_ft_simplex(ndim: u8) -> Result<FactorPuzzleSpec> {
    let gen_seqs = std::iter::chain(
        [AbbrGenSeq::INIT],
        (0..ndim)
            .rev()
            .enumerate()
            .map(|(i, m)| AbbrGenSeq::new([GeneratorId(m)], Some(i))),
    );
    let name_strings = (0..=ndim as u32).map(hypuz_notation::family::SequentialUppercaseName);
    let names = gen_seqs.zip(name_strings.map(|n| n.to_string())).collect();

    let facet_points = NamedPointOrbitSpec::new(Vector::unit(ndim - 1), names);

    Ok(FactorPuzzleSpec::new_ft(
        CatalogId::new("ft_simplex", vec![(ndim as i64).into()]).unwrap(),
        CatalogId::new("simplex", vec![(ndim as i64).into()]).unwrap(),
        CoxeterMatrix::A(ndim)?,
        // TODO: vertex axes
        vec![facet_points.to_axes(vec![INF, 0.0, -INF], vec![])], // TODO
        vec![facet_points],
        vec![], // TODO
    ))
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

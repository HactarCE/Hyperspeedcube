//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::sync::Arc;

use eyre::Result;
use hypergroup::{AbbrGenSeq, GeneratorId};
use hypermath::prelude::*;
use hyperpuzzle_core::group::{CoxeterMatrix, GroupElementId};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleAnimation, NdEuclidPuzzleStateRenderData};

mod builder;
mod geometry;
mod names;
mod twist_system;

use builder::ProductPuzzleBuilder;

pub use twist_system::SymmetricTwistSystemEngineData;

pub fn add_puzzles_to_catalog(catalog: &hyperpuzzle_core::Catalog) -> Result<()> {
    catalog.add_puzzle(Arc::new(PuzzleSpec {
        meta: Arc::new(PuzzleListMetadata {
            id: "symmetric_puzzle_test".to_string(),
            version: Version {
                major: 0,
                minor: 0,
                patch: 1,
            },
            name: "Symmetric Puzzle Test".to_string(),
            aliases: vec![],
            tags: TagSet::new(),
        }),
        build: Box::new(|build_ctx| {
            // IIFE to mimic try_block
            (|| -> Result<_> {
                ProductPuzzleBuilder::direct_product_identity()
                    // .direct_product(&ft_cube(5)?)?
                    .direct_product(&shallow_polygon(5)?)?
                    .direct_product(&shallow_polygon(6)?)?
                    // .direct_product(&shallow_ft_simplex(3)?)?
                    .build(Some(&build_ctx), &mut |e| eprintln!("{e}")) // TODO: better warn function
            })()
            .map(Redirectable::Direct)
            .map_err(|e| e.to_string())
        }),
    }))?;
    Ok(())
}

pub fn direct_product_vectors(
    a_ndim: u8,
    b_ndim: u8,
    a: impl VectorRef,
    b: impl VectorRef,
) -> Vector {
    std::iter::chain(a.iter_ndim(a_ndim), b.iter_ndim(b_ndim)).collect()
}

#[derive(Debug, Clone)]
pub struct ProductPuzzleState {
    ty: Arc<Puzzle>,
    twists: Arc<SymmetricTwistSystemEngineData>,
    piece_grip_signatures: Arc<PerPiece<PerAxis<Option<Layer>>>>,
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
            if self
                .piece_layer_on_axis(piece, axis)
                .is_some_and(|l| layers.contains(l))
            {
                WhichSide::Inside
            } else {
                WhichSide::Outside
            }
        })
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        Some(LayerMask::from_layer(
            self.piece_layer_on_axis(piece, axis)?,
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

    fn piece_layer_on_axis(&self, piece: Piece, axis: Axis) -> Option<Layer> {
        let attitude = self.piece_attitudes[piece];
        let inverse_attitude = self.twists.group.inverse(attitude);
        self.piece_grip_signatures[piece][self.twists.group_action.act(inverse_attitude, axis)]
    }
}

fn autonames() -> impl Iterator<Item = String> {
    (0..)
        .map(hypuz_notation::family::SequentialUppercaseName)
        .map(|prefix| prefix.to_string())
}

const INF: Float = Float::INFINITY;

fn ft_cube(ndim: u8) -> Result<ProductPuzzleBuilder> {
    if ndim > 5 {
        unimplemented!();
    }

    let names = vec![
        (5, "A", None),
        (4, "O", Some(4)),
        (3, "F", Some(3)),
        (2, "U", Some(2)),
        (1, "R", Some(1)),
        (1, "L", Some(0)),
        (2, "D", Some(1)),
        (3, "B", Some(2)),
        (4, "I", Some(3)),
        (5, "P", Some(4)),
    ]
    .into_iter()
    .filter(|&(n, _, _)| n <= ndim)
    .enumerate()
    .map(|(i, (_, name, mirror))| {
        let gen_seq = match mirror {
            Some(g) => AbbrGenSeq::new([GeneratorId(g)], Some(i - 1)),
            None => AbbrGenSeq::INIT,
        };
        (gen_seq, name.to_string())
    })
    .collect();

    ProductPuzzleBuilder::new_ft(
        ndim,
        CoxeterMatrix::B(ndim)?.isometry_group()?,
        &[(
            Vector::unit(ndim - 1),
            vec![INF, 1.0 / 3.0, -1.0 / 3.0, -INF],
            names,
        )],
    )
}

fn shallow_polygon(n: u16) -> Result<ProductPuzzleBuilder> {
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
    let edge_length = 2.0 * pi_div_n.tan();
    let edge_depth = (2.0 * pi_div_n).sin() * edge_length;
    let cut_depth = 1.0 - edge_depth / 3.0;
    let axes = [(Vector::unit(1), vec![INF, cut_depth], names)];
    ProductPuzzleBuilder::new_ft(2, CoxeterMatrix::I(n)?.isometry_group()?, &axes)
}

fn shallow_line() -> Result<ProductPuzzleBuilder> {
    line(vec![INF, 1.0 / 3.0, -1.0 / 3.0, -INF])
}

fn half_cut_line() -> Result<ProductPuzzleBuilder> {
    line(vec![INF, 0.0, -INF])
}

fn line(cut_depths: Vec<Float>) -> Result<ProductPuzzleBuilder> {
    let names = vec![
        (AbbrGenSeq::INIT, "A".to_string()),
        (AbbrGenSeq::new([GeneratorId(0)], Some(0)), "B".to_string()),
    ];
    let axes = [(Vector::unit(0), cut_depths, names)];
    ProductPuzzleBuilder::new_ft(1, CoxeterMatrix::A(1)?.isometry_group()?, &axes)
}

fn megaminx() -> Result<ProductPuzzleBuilder> {
    let names = vec![
        ("F", None, None),
        ("U", Some(2), Some(0)),
        ("R", Some(1), Some(1)),
        ("L", Some(0), Some(2)),
        ("DR", Some(1), Some(3)),
        ("DL", Some(0), Some(4)),
        ("BR", Some(2), Some(4)),
        ("BL", Some(2), Some(5)),
        ("PR", Some(1), Some(6)),
        ("PL", Some(0), Some(8)),
        ("PD", Some(1), Some(9)),
        ("PB", Some(2), Some(10)),
    ]
    .into_iter()
    .map(|(name, mirror, end)| {
        let gen_seq = AbbrGenSeq::new(mirror.map(GeneratorId), end);
        (gen_seq, name.to_string())
    })
    .collect();

    let symmetry = CoxeterMatrix::H3().isometry_group()?;
    let cut_depth = std::f64::consts::GOLDEN_RATIO.recip();
    let axes = [(Vector::unit(2), vec![INF, cut_depth], names)];
    ProductPuzzleBuilder::new_ft(3, symmetry, &axes)
}

fn shallow_ft_simplex(ndim: u8) -> Result<ProductPuzzleBuilder> {
    let gen_seqs = std::iter::chain(
        [AbbrGenSeq::INIT],
        (0..ndim)
            .rev()
            .enumerate()
            .map(|(i, m)| AbbrGenSeq::new([GeneratorId(m)], Some(i))),
    );
    let name_strings = (0..=ndim as u32).map(hypuz_notation::family::SequentialUppercaseName);
    let names = gen_seqs.zip(name_strings.map(|n| n.to_string())).collect();

    let axes = [(Vector::unit(ndim - 1), vec![INF, 0.0, -INF], names)];
    ProductPuzzleBuilder::new_ft(ndim, CoxeterMatrix::A(ndim)?.isometry_group()?, &axes)
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

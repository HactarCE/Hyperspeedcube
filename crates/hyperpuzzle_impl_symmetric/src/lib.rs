//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::sync::Arc;

use eyre::Result;
use hypermath::prelude::*;
use hyperpuzzle_core::group::{CoxeterMatrix, GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::NdEuclidPuzzleStateRenderData;

mod builder;
mod geometry;
mod names;

use builder::ProductPuzzleBuilder;

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
                    .direct_product(&shallow_polygon(5)?)?
                    .direct_product(&shallow_polygon(6)?)?
                    .build()
            })()
            .map(Redirectable::Direct)
            .map_err(|e| e.to_string())
        }),
    }))?;
    Ok(())
}

hypuz_util::typed_index_struct! {
    pub struct NamedPoint(u16);
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
pub struct ProductPuzzle {
    ty: Arc<Puzzle>,
    grip_group: IsometryGroup,
    attitudes: PerPiece<GroupElementId>,
}

impl PuzzleState for ProductPuzzle {
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
        todo!()
    }

    fn do_twist_dyn(&self, twist: &Move) -> std::result::Result<BoxDynPuzzleState, Vec<Piece>> {
        todo!()
    }

    fn is_solved(&self) -> bool {
        true
    }

    fn compute_grip(&self, axis: Axis, layers: &LayerMask) -> PerPiece<WhichSide> {
        todo!() // TODO
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!() // TODO
    }

    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!() // TODO
    }

    fn render_data(&self) -> BoxDynPuzzleStateRenderData {
        NdEuclidPuzzleStateRenderData {
            piece_transforms: self.attitudes.map_ref(|_, &e| self.grip_group.motor(e)),
        }
        .into()
    }

    fn partial_twist_render_data(&self, twist: &Move, t: f32) -> BoxDynPuzzleStateRenderData {
        todo!()
    }

    fn animated_render_data(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData {
        todo!()
    }
}

fn autonames() -> impl Iterator<Item = String> {
    (0..)
        .map(hyperpuzzle_core::notation::family::UppercaseGreekPrefix)
        .map(|prefix| prefix.to_string())
}

fn rubiks_cube() -> Result<ProductPuzzleBuilder> {
    ProductPuzzleBuilder::new(
        3,
        CoxeterMatrix::B(3)?.isometry_group()?,
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0 / 3.0).unwrap()],
    )
}

fn shallow_polygon(n: u16) -> Result<ProductPuzzleBuilder> {
    let pi_div_n = std::f64::consts::PI as Float / n as Float;

    let edge_length = 2.0 * pi_div_n.tan();
    let edge_depth = (2.0 * pi_div_n).sin() * edge_length;
    let cut_depth = 1.0 - edge_depth / 3.0;
    ProductPuzzleBuilder::new(
        2,
        CoxeterMatrix::I(n)?.isometry_group()?,
        &[Hyperplane::new(vector![0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![0.0, 1.0], cut_depth).unwrap()],
    )
}

fn shallow_line() -> Result<ProductPuzzleBuilder> {
    ProductPuzzleBuilder::new(
        1,
        CoxeterMatrix::A(1)?.isometry_group()?,
        &[Hyperplane::new(vector![1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![1.0], 1.0 / 3.0).unwrap()],
    )
}

fn half_cut_line() -> Result<ProductPuzzleBuilder> {
    ProductPuzzleBuilder::new(
        1,
        CoxeterMatrix::A(1)?.isometry_group()?,
        &[Hyperplane::new(vector![1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![1.0], 0.0).unwrap()],
    )
}

fn megaminx() -> Result<ProductPuzzleBuilder> {
    ProductPuzzleBuilder::new(
        3,
        CoxeterMatrix::H3().isometry_group()?,
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(
            vector![0.0, 0.0, 1.0],
            std::f64::consts::GOLDEN_RATIO.recip(),
        )
        .unwrap()],
    )
}

fn simplex_a(ndim: u8) -> Result<ProductPuzzleBuilder> {
    ProductPuzzleBuilder::new(
        ndim,
        CoxeterMatrix::A(ndim)?.isometry_group()?,
        &[Hyperplane::new(Vector::unit(ndim - 1), 1.0).unwrap()],
        &[Hyperplane::new(Vector::unit(ndim - 1), 0.0).unwrap()],
    )
}

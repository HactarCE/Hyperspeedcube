//! Flat Hypercube puzzle simulation backend for Hyperspeedcube.
//!
//! Supports up to 10 dimensions.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;
use std::{fmt, ops::Mul};

use hypermath::{Sign, WhichSide, smallvec::SmallVec};
use hyperpuzzle_core::prelude::*;

pub const MAX_NDIM: usize = 10;

/// Prelude of common imports.
pub mod prelude {
    pub use crate::{
        FlatPuzzleAnimation, FlatPuzzleGeometry, FlatPuzzleState, FlatPuzzleStateRenderData,
        FlatPuzzleUiData,
    };
}

#[derive(Debug)]
pub struct FlatPuzzleGeometry {
    pub puzzle: FlatPuzzle,
}

#[derive(Debug)]
pub struct FlatPuzzleStateRenderData {
    pub ndim: u8,
    pub max_layer_count: u8,
    pub piece_positions: PerPiece<[u8; MAX_NDIM]>,
    pub sticker_facets: PerSticker<Facet>,
    pub anim: Option<FlatPuzzleAnimation>,
    pub t: f32,
}
impl PuzzleStateRenderData for FlatPuzzleStateRenderData {}

#[derive(Debug)]
pub struct FlatTwistSystemEngineData {
    pub twist_geometry_infos: PerTwist<(Facet, Dim, Dim)>,
}
impl TwistSystemEngineData for FlatTwistSystemEngineData {}

/// Dimension (X, Y, Z, W, V, U, ...)
///
/// Dimensions up to 10 are supported.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Dim(pub u8);
impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Self::NAMES.get(self.0 as usize) {
            Some(name) => write!(f, "{name}"),
            None => write!(f, "{}", self.0),
        }
    }
}
impl Dim {
    pub const NAMES: [char; 6] = ['x', 'y', 'z', 'w', 'v', 'u'];

    pub const X: Dim = Dim(0);
    pub const Y: Dim = Dim(1);
    pub const Z: Dim = Dim(2);
    pub const W: Dim = Dim(3);
    pub const V: Dim = Dim(4);
    pub const U: Dim = Dim(5);
}

/// Facet (axis, sign)
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Facet(pub u8);
impl fmt::Debug for Facet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.sign(), self.dim())
    }
}
impl fmt::Display for Facet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Self::NAMES.as_flattened().get(self.0 as usize) {
            Some(name) => write!(f, "{name}"),
            None => write!(f, "{self:?}"),
        }
    }
}
impl Facet {
    pub const NAMES: [[char; 2]; MAX_NDIM] = [
        ['R', 'L'],
        ['U', 'D'],
        ['F', 'B'],
        ['O', 'I'],
        ['A', 'P'],
        ['Γ', 'Δ'],
        ['Θ', 'Λ'],
        ['Ξ', 'Π'],
        ['Σ', 'Φ'],
        ['Ψ', 'Ω'],
    ];

    pub const R: Self = Self(0);
    pub const L: Self = Self(1);
    pub const U: Self = Self(2);
    pub const D: Self = Self(3);
    pub const F: Self = Self(4);
    pub const B: Self = Self(5);
    pub const O: Self = Self(6);
    pub const I: Self = Self(7);
    pub const A: Self = Self(8);
    pub const P: Self = Self(9);
    pub const Γ: Self = Self(10);
    pub const Δ: Self = Self(11);
    pub const Θ: Self = Self(12);
    pub const Λ: Self = Self(13);
    pub const Ξ: Self = Self(14);
    pub const Π: Self = Self(15);
    pub const Σ: Self = Self(16);
    pub const Φ: Self = Self(17);
    pub const Ψ: Self = Self(18);
    pub const Ω: Self = Self(19);

    pub fn new(dim: Dim, sign: Sign) -> Self {
        Self(dim.0 << 1 | sign as u8)
    }
    pub fn dim(self) -> Dim {
        Dim(self.0 >> 1)
    }
    pub fn sign(self) -> Sign {
        use Sign::{Neg, Pos};
        if self.0 & 1 == 0 { Pos } else { Neg }
    }

    pub fn color(self) -> Color {
        Color(self.0 as u16)
    }
    pub fn from_color(color: Color) -> Self {
        Self(color.0 as u8)
    }

    pub fn hyperpuzzle_axis(self) -> Axis {
        Axis(self.0 as u16)
    }

    #[must_use]
    pub fn opposite(self) -> Self {
        Self(self.0 ^ 1)
    }
}
impl Mul<Sign> for Facet {
    type Output = Facet;

    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Pos => self,
            Sign::Neg => self.opposite(),
        }
    }
}

#[derive(Debug)]
pub struct FlatPuzzle {
    /// Cuboid dimensions.
    pub dimensions: [u8; MAX_NDIM],
}

pub struct FlatPuzzleUiData {
    pub geom: Arc<FlatPuzzleGeometry>,
}
impl PuzzleUiData for FlatPuzzleUiData {}

/// Animation for a Flat puzzle.
#[derive(Debug, Clone)]
pub struct FlatPuzzleAnimation {
    /// Set of pieces affected by the animation.
    pub pieces: PieceMask,
    /// Dimension to rotate from.
    pub from: Dim,
    /// Dimension to rotate toward.
    pub to: Dim,
}
impl PuzzleAnimation for FlatPuzzleAnimation {
    fn clone_dyn(&self) -> BoxDynPuzzleAnimation
    where
        Self: Sized,
    {
        self.clone().into()
    }
}

/// Instance of a puzzle with a particular state.
#[derive(Clone)]
pub struct FlatPuzzleState {
    /// Immutable puzzle type info.
    puzzle_type: Arc<Puzzle>,
    /// Layer count along each axis.
    ///
    /// Extra layers are 0.
    size: [u8; MAX_NDIM],
    /// Maximum layer count along any axis.
    max_layer_count: u8,
    /// Transform for each piece.
    piece_transforms: PerPiece<PieceTransform>,
}
impl fmt::Debug for FlatPuzzleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlatPuzzleState")
            .field("puzzle_type", &self.puzzle_type.meta.name)
            .field("dimensions", &self.size)
            .field("piece_transforms", &self.piece_transforms)
            .finish()
    }
}
impl FlatPuzzleState {
    pub fn ndim(&self) -> u8 {
        self.size.iter().position(|&n| n == 0).unwrap_or(MAX_NDIM) as u8
    }
}
impl PuzzleState for FlatPuzzleState {
    fn ty(&self) -> &Arc<Puzzle> {
        &self.puzzle_type
    }

    fn clone_dyn(&self) -> BoxDynPuzzleState {
        self.clone().into()
    }

    fn do_twist(&self, twist: LayeredTwist) -> Result<Self, Vec<Piece>>
    where
        Self: Sized,
    {
        let (facet, from, to) = self
            .puzzle_type
            .twists
            .engine_data
            .downcast_ref::<FlatTwistSystemEngineData>()
            .unwrap()
            .twist_geometry_infos[twist.transform];
        let grip = self.compute_grip(Axis(facet.dim().0 as u16), twist.layers);
        let from = from.0 as usize;
        let to = to.0 as usize;

        let mut piece_transforms = self.piece_transforms.clone();
        for (piece, piece_transform) in &mut piece_transforms {
            if grip[piece] == WhichSide::Inside {
                piece_transform.0[to] = piece_transform.0[to].opposite();
                piece_transform.0.swap(from, to);
            }
        }

        Ok(Self {
            puzzle_type: Arc::clone(&self.puzzle_type),
            size: self.size,
            max_layer_count: self.max_layer_count,
            piece_transforms,
        })
    }

    fn do_twist_dyn(&self, twist: LayeredTwist) -> Result<BoxDynPuzzleState, Vec<Piece>> {
        self.do_twist(twist).map(BoxDynPuzzleState::new)
    }

    fn is_solved(&self) -> bool {
        let mut color_map = [None; MAX_NDIM * 2];
        for (_sticker, sticker_info) in &self.puzzle_type.stickers {
            let piece = sticker_info.piece;
            let transform = self.piece_transforms[piece];
            let sticker_original_facet = Facet::from_color(sticker_info.color);
            let sticker_new_facet = transform.transform_facet(sticker_original_facet);
            match &mut color_map[sticker_new_facet.0 as usize] {
                it @ None => *it = Some(sticker_info.color),
                Some(it) if *it == sticker_info.color => (),
                Some(_) => return false,
            }
        }
        true
    }

    fn compute_grip(&self, axis: Axis, layers: LayerMask) -> PerPiece<WhichSide> {
        let facet = Facet(axis.0 as u8);
        let dim = facet.dim();
        let Some(&layer_count) = self.size.get(dim.0 as usize) else {
            return self.piece_transforms.map_ref(|_, _| WhichSide::Outside);
        };
        let layers = match facet.sign() {
            Sign::Pos => layers.reverse(layer_count),
            Sign::Neg => layers,
        };
        iter_piece_positions(self.size)
            .map(|pos| {
                if layers.contains(Layer(pos[dim.0 as usize])) {
                    WhichSide::Inside
                } else {
                    WhichSide::Outside
                }
            })
            .collect()
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!()
    }

    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!()
    }

    fn render_data(&self) -> BoxDynPuzzleStateRenderData {
        let mut initial_piece_positions = iter_piece_positions(self.size);
        let piece_positions = self.piece_transforms.map_ref(|piece, piece_transform| {
            let init_pos = initial_piece_positions.next().unwrap();
            let mut ret = [0; MAX_NDIM];
            for i in 0..self.ndim() {
                let f = piece_transform.0[i as usize];
                ret[i as usize] = init_pos[f.dim().0 as usize];
                if f.sign() == Sign::Neg {
                    ret[i as usize] = self.max_layer_count - 1 - ret[i as usize];
                }
            }
            ret
        });

        let sticker_facets = self.ty().stickers.map_ref(|_sticker, sticker_info| {
            self.piece_transforms[sticker_info.piece]
                .transform_facet(Facet::from_color(sticker_info.color))
        });

        FlatPuzzleStateRenderData {
            ndim: self.ndim(),
            max_layer_count: self.max_layer_count,
            piece_positions,
            sticker_facets,
            anim: None,
            t: 0.0,
        }
        .into()
    }

    fn partial_twist_render_data(
        &self,
        twist: LayeredTwist,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData {
        let axis = self.puzzle_type.twists.twists[twist.transform].axis;
        let pieces = self.compute_gripped_pieces(axis, twist.layers);
        let (_facet, from, to) = self
            .puzzle_type
            .twists
            .engine_data
            .downcast_ref::<FlatTwistSystemEngineData>()
            .unwrap()
            .twist_geometry_infos[twist.transform];
        let anim = FlatPuzzleAnimation { pieces, from, to };
        self.animated_render_data(&anim.into(), t)
    }

    fn animated_render_data(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData {
        let anim = anim
            .downcast_ref::<FlatPuzzleAnimation>()
            .expect("expected FlatPuzzleAnimation");

        FlatPuzzleStateRenderData {
            anim: Some(anim.clone()),
            t,
            ..*self.render_data().downcast().unwrap()
        }
        .into()
    }
}

impl FlatPuzzleState {
    pub fn twist_anim(&self, twist: LayeredTwist) -> FlatPuzzleAnimation {
        let axis = self.puzzle_type.twists.twists[twist.transform].axis;
        let pieces = self.compute_gripped_pieces(axis, twist.layers);
        let (_facet, from, to) = self
            .puzzle_type
            .twists
            .engine_data
            .downcast_ref::<FlatTwistSystemEngineData>()
            .unwrap()
            .twist_geometry_infos[twist.transform];
        FlatPuzzleAnimation { pieces, from, to }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceTransform([Facet; 16]);
impl Default for PieceTransform {
    fn default() -> Self {
        Self(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
                .map(|dim| Facet::new(Dim(dim), Sign::Pos)),
        )
    }
}
impl PieceTransform {
    pub fn transform_facet(self, facet: Facet) -> Facet {
        self.0[facet.dim().0 as usize] * facet.sign()
    }
}

// TODO: make this a method on some struct
pub fn iter_piece_positions(size: [u8; MAX_NDIM]) -> impl Iterator<Item = [u8; MAX_NDIM]> {
    let ndim = size.iter().position(|&n| n == 0).unwrap_or(MAX_NDIM);
    std::iter::successors(Some([0; MAX_NDIM]), move |&(mut pos): &[u8; MAX_NDIM]| {
        for i in 0..ndim {
            pos[i] += 1;
            if pos[i] < size[i] {
                return Some(pos);
            }
            pos[i] = 0;
        }
        None
    })
}

pub fn load_puzzles(catalog: &Catalog) {
    let layer_count = 3;
    let ndim = 4;
    let mut size = [0; MAX_NDIM];
    size[..ndim].fill(layer_count);

    let meta = Arc::new(PuzzleListMetadata {
        id: "flat_hypercube_3_4".to_string(),
        version: Version {
            major: 0,
            minor: 0,
            patch: 1,
        },
        name: "Flat Hypercube 3^4".to_string(),
        aliases: vec![],
        tags: TagSet::new(), // TODO
    });

    let colors = catalog.build_blocking("hypercube").unwrap();

    catalog
        .add_puzzle(Arc::new(PuzzleSpec {
            meta: meta.clone(),
            build: Box::new(move |_build_ctx| {
                let meta = meta.clone();

                let piece_types: PerPieceType<_> = (0..=ndim)
                    .map(|sticker_count| PieceTypeInfo {
                        name: format!("{sticker_count}c"),
                        display: format!("{sticker_count}c"),
                    })
                    .collect();
                let mut piece_type_hierarchy = PieceTypeHierarchy::new(piece_types.len());
                for (piece_type_id, piece_type_info) in &piece_types {
                    piece_type_hierarchy
                        .set_piece_type_id(&piece_type_info.name, piece_type_id)
                        .unwrap();
                }

                let mut pieces = PerPiece::new();
                let mut stickers = PerSticker::new();
                for pos in iter_piece_positions(size) {
                    let piece = pieces.next_idx().unwrap();
                    let stickers: SmallVec<_> = pos
                        .into_iter()
                        .enumerate()
                        .take(ndim)
                        .flat_map(|(axis, coord)| {
                            let dim = Dim(axis as u8);
                            [
                                (coord == 0).then_some(Sign::Neg),
                                (coord == size[axis] - 1).then_some(Sign::Pos),
                            ]
                            .into_iter()
                            .filter_map(move |sign| {
                                let color = Facet::new(dim, sign?).color();
                                Some(StickerInfo { piece, color })
                            })
                        })
                        .map(|sticker_info| stickers.push(sticker_info).unwrap())
                        .collect();
                    let sticker_count = stickers.len();
                    pieces
                        .push(PieceInfo {
                            stickers,
                            piece_type: PieceType(sticker_count as u16),
                        })
                        .unwrap();
                }

                let piece_type_masks = piece_types
                    .iter()
                    .map(|(piece_type, piece_type_info)| {
                        (
                            piece_type_info.name.clone(),
                            PieceMask::from_iter(
                                pieces.len(),
                                pieces.iter_filter(|_piece, piece_info| {
                                    piece_info.piece_type == piece_type
                                }),
                            ),
                        )
                    })
                    .collect();

                let (twist_infos, twist_geometry_infos) = generate_twists(ndim);

                let signs = [Sign::Neg, Sign::Pos];
                let dims = (0..ndim as u8).map(Dim);
                let facets = itertools::iproduct!(dims.clone(), signs)
                    .map(|(dim, sign)| Facet::new(dim, sign));
                let scramble_twists = twist_infos.iter_keys().collect();

                let mut axis_names = NameSpecBiMapBuilder::new();
                for facet in facets.clone() {
                    axis_names
                        .set(facet.hyperpuzzle_axis(), Some(facet.to_string()))
                        .unwrap();
                }

                let colors = Arc::clone(&colors);

                Ok(Redirectable::Direct(Arc::new_cyclic(move |this| Puzzle {
                    this: this.clone(),
                    meta,
                    view_prefs_set: None,
                    pieces,
                    stickers,
                    piece_types,
                    piece_type_hierarchy,
                    piece_type_masks,
                    colors,
                    scramble_twists,
                    full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
                    notation: Notation {},
                    axis_layers: PerAxis::new(), // shouldn't be needed
                    axis_opposites: facets
                        .clone()
                        .map(|facet| Some(facet.opposite().hyperpuzzle_axis()))
                        .collect(),
                    twists: Arc::new(TwistSystem {
                        id: "flat_hypercube".to_string(),
                        name: "Flat Hypercube".to_string(),
                        axes: Arc::new(AxisSystem {
                            names: Arc::new(axis_names.build(ndim * 2).unwrap()),
                            orbits: vec![],
                        }),
                        names: Default::default(),
                        twists: twist_infos,
                        directions: Default::default(),
                        vantage_groups: Default::default(),
                        vantage_sets: Default::default(),
                        engine_data: FlatTwistSystemEngineData {
                            twist_geometry_infos,
                        }
                        .into(),
                    }),
                    ui_data: FlatPuzzleUiData {
                        geom: Arc::new(FlatPuzzleGeometry {
                            puzzle: FlatPuzzle { dimensions: size },
                        }),
                    }
                    .into(),
                    new: Box::new(move |puzzle_type| {
                        let piece_transforms =
                            puzzle_type.pieces.map_ref(|_, _| PieceTransform::default());
                        FlatPuzzleState {
                            puzzle_type,
                            size,
                            max_layer_count: size.into_iter().max().unwrap_or(1),
                            piece_transforms,
                        }
                        .into()
                    }),
                })))
            }),
        }))
        .unwrap();
    // catalog.add_puzzle_generator(Arc::new(PuzzleSpecGenerator {
    //     meta: Arc::new(PuzzleListMetadata {
    //         id: "flat_hypercube".to_string(),
    //         version: Version {
    //             major: 0,
    //             minor: 1,
    //             patch: 0,
    //         },
    //         name: "Flat Hypercube".to_string(),
    //         aliases: vec![],
    //         tags: TagSet::new(), // TODO
    //     }),
    //     params: vec![GeneratorParam {
    //         name: "todo!()",
    //         ty: todo!(),
    //         default: todo!(),
    //     }],
    //     examples: (),
    //     generate: (),
    // }));
}

fn generate_twists(ndim: usize) -> (PerTwist<TwistInfo>, PerTwist<(Facet, Dim, Dim)>) {
    let signs = [Sign::Neg, Sign::Pos];
    let dims = (0..ndim as u8).map(Dim);
    let facets = itertools::iproduct!(dims.clone(), signs).map(|(dim, sign)| Facet::new(dim, sign));
    let mut twist_infos = PerTwist::new();
    let mut twist_geometry_infos = PerTwist::new();
    let mut map = HashMap::new();
    for (facet, from, to) in itertools::iproduct!(facets.clone(), dims.clone(), dims.clone())
        .filter(|&(facet, from, to)| facet.dim() != from && facet.dim() != to && from != to)
    {
        let twist_info = TwistInfo {
            qtm: 1,
            axis: facet.hyperpuzzle_axis(),
            reverse: Twist(0),
            include_in_scrambles: true,
        };
        twist_geometry_infos.push((facet, from, to)).unwrap();
        let twist_id = twist_infos.push(twist_info).unwrap();
        map.insert((facet, from, to), twist_id);
    }
    for (&(facet, from, to), &id) in &map {
        twist_infos[id].reverse = map[&(facet, to, from)];
    }
    (twist_infos, twist_geometry_infos)
}

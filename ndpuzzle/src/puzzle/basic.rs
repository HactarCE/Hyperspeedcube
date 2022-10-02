use anyhow::Result;
use itertools::Itertools;
use std::sync::Arc;

use crate::math::*;
use crate::polytope::*;
use crate::schlafli::SchlafliSymbol;
use crate::spec::*;

use super::PuzzleState;
use super::PuzzleType;

const EPSILON: f32 = 0.001;

pub fn build_puzzle(spec: &BasicPuzzleSpec) -> Result<PuzzleData> {
    let shape_spec = &spec.shape[0];
    let shape_generators = shape_spec
        .symmetries
        .iter()
        .flat_map(|sym| match sym {
            SymmetriesSpec::Schlafli(string) => SchlafliSymbol::from_string(&string).generators(),
        })
        .collect_vec();
    // let m1 = Matrix::from_cols(shape_schlafli.mirrors().iter().rev().map(|v| &v.0))
    //     .inverse()
    //     .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
    //     .transpose();
    let poles = shape_spec
        .seeds
        .iter()
        .map(|v| v.clone().resize(spec.ndim))
        .collect::<Vec<_>>();
    let (mut arena, facets) = generate_polytope(spec.ndim, &shape_generators, &poles)?;

    let mut axes = vec![];
    for twist_spec in &spec.twists {
        let axis_generators = twist_spec
            .symmetries
            .iter()
            .flat_map(|sym| match sym {
                SymmetriesSpec::Schlafli(string) => {
                    SchlafliSymbol::from_string(&string).generators()
                }
            })
            .collect_vec();
        // let m2 = Matrix::from_cols(twist_schlafli.mirrors().iter().rev().map(|v| &v.0))
        //     .inverse()
        //     .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
        //     .transpose();
        let base_axes = twist_spec
            .axes
            .iter()
            .map(
                |AxisSpec {
                     normal,
                     cuts,
                     twist_generators,
                 }| {
                    // let normal = m2.transform(normal.clone().resize(spec.ndim));
                    let normal = normal.clone().resize(spec.ndim).normalize().expect("msg");
                    let mut distances = cuts.clone();
                    distances.sort_by(f32::total_cmp);
                    distances.reverse();
                    Axis {
                        normal,
                        distances,
                        transforms: twist_generators
                            .iter()
                            .map(|gen| parse_transform(gen).expect("oops"))
                            .collect(),
                    }
                },
            )
            .collect::<Vec<_>>();
        axes.extend(build_axes(&&axis_generators, &base_axes)?);
    }
    for axis in &axes {
        for i in 0..axis.distances.len() {
            arena.slice_by_plane(&axis.plane(i), false)?;
        }
    }
    let piece_ids = arena
        .roots
        .iter()
        .copied()
        .filter(|&p| !arena.is_piece_internal(p).expect("Root did not exist"))
        .collect_vec();
    let sticker_ids = piece_ids
        .iter()
        .flat_map(|&p| arena.polytope_facet_ids(p, true).expect("Bad children"))
        .collect_vec();

    Ok(PuzzleData {
        arena,
        piece_ids,
        sticker_ids,
        axes,
        facets,
    })
}

pub fn build_axes(twist_generators: &[Matrix], base_axes: &[Axis]) -> Result<Vec<Axis>> {
    let mut axes: Vec<Axis> = base_axes.to_vec();
    let mut transforms = axes
        .iter()
        .enumerate()
        .flat_map(|(i, axis)| {
            axis.transforms
                .iter()
                .map(move |transform| (i, transform.clone()))
        })
        .collect_vec();
    let mut next_unprocessed = 0;
    while next_unprocessed < transforms.len() {
        for gen in twist_generators {
            let curr_twist = &transforms[next_unprocessed];
            let curr_axis = &axes[curr_twist.0];
            let new_normal = gen * &curr_axis.normal;
            let new_transform =
                &(gen * &curr_twist.1) * &gen.inverse().ok_or(PolytopeError::BadMatrix)?;
            let new_axis = Axis {
                normal: new_normal,
                distances: curr_axis.distances.clone(),
                transforms: vec![],
            };
            let new_i = (0..axes.len())
                .find(|&index| axes[index].normal.approx_eq(&new_axis.normal, EPSILON))
                .unwrap_or_else(|| {
                    axes.push(new_axis);
                    axes.len() - 1
                });
            if transforms
                .iter()
                .all(|(i, t)| !(t.approx_eq(&new_transform, EPSILON) && *i == new_i))
            {
                transforms.push((new_i, new_transform.clone()));
                axes[new_i].add_transform(new_transform);
            }
        }
        next_unprocessed += 1;
    }

    Ok(axes)
}

pub fn puzzle_type(spec: BasicPuzzleSpec) -> Result<Arc<PuzzleType>> {
    let puzzle_data = build_puzzle(&spec)?;

    let mut piece_infos = vec![];
    let mut sticker_infos = vec![];
    for &piece in &puzzle_data.piece_ids {
        let i = sticker_infos.len() as u16;
        let stickers = puzzle_data.arena.polytope_facet_ids(piece, true)?;
        sticker_infos.extend(stickers.iter().map(|s| super::StickerInfo {
            piece: super::Piece(piece_infos.len() as u16),
            color: super::Facet(0),
        }));
        let j = sticker_infos.len() as u16;
        piece_infos.push(super::PieceInfo {
            stickers: (i..j).map(super::Sticker).collect(),
            piece_type: super::PieceType(0),
        })
    }
    Ok(Arc::new_cyclic(|this| PuzzleType {
        this: this.clone(),
        name: spec.name,
        ndim: spec.ndim,
        shape: Arc::new(super::PuzzleShape {
            name: "Todo".to_string(),
            ndim: spec.ndim,
            facets: puzzle_data
                .facets()
                .iter()
                .map(|facet| super::FacetInfo {
                    name: format!("{:?}", facet),
                })
                .collect(),
        }),
        twists: Arc::new(super::PuzzleTwists {
            name: "Todo".to_string(),
            axes: puzzle_data
                .axes()
                .iter()
                .map(|axis| super::TwistAxisInfo {
                    symbol: format!("{:?}", axis.normal),
                    layer_count: axis.distances.len() as u8,
                    opposite: None,
                })
                .collect(),
            directions: vec![],
            orientations: vec![Rotor::identity()],
        }),
        family_name: "Fun".to_string(),
        projection_type: super::ProjectionType::_4D,
        radius: spec.ndim as f32,
        layer_count: 9,
        pieces: piece_infos,
        stickers: sticker_infos,
        piece_types: vec![super::PieceTypeInfo::new("Piece".to_string())],
        scramble_moves_count: 3,
        notation: super::NotationScheme {
            axis_names: vec![],
            direction_names: vec![],
            block_suffix: None,
            aliases: vec![],
        },
        new: Box::new(move |ty| {
            Box::new(Puzzle {
                data: puzzle_data.clone(),
                ty,
            })
        }),
    }))
}

#[derive(Debug, Clone)]
pub struct PuzzleData {
    arena: PolytopeArena,
    piece_ids: Vec<PolytopeId>,
    sticker_ids: Vec<PolytopeId>,
    axes: Vec<Axis>,
    facets: Vec<Vector>,
}
impl PuzzleData {
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    pub fn facets(&self) -> &[Vector] {
        &self.facets
    }

    pub fn apply_twist(&mut self, twist: super::Twist) -> Result<Result<(), Vec<PolytopeId>>> {
        let axis = &self.axes[twist.axis.0 as usize];
        let transform = &axis.transforms[twist.direction.0 as usize];
        let spans = self.arena.axis_spans(&axis.normal)?;
        let layer_spans = spans.into_iter().map(|(p, s)| {
            (
                p,
                super::LayerMask(
                    (1 << (axis.layer_from_depth(s.below) + 1))
                        - (1 << axis.layer_from_depth(s.above)),
                ),
            )
        });

        let mut pieces = vec![];
        let mut blocking = vec![];
        for (p, layer_span) in layer_spans {
            if layer_span & twist.layers == layer_span {
                pieces.push(p);
            } else if layer_span & twist.layers != super::LayerMask(0) {
                blocking.push(p);
            }
        }
        if !blocking.is_empty() {
            return Ok(Err(blocking));
        }

        for p in pieces {
            self.arena.transform_polytope(p, &transform)?;
        }
        Ok(Ok(()))
    }

    pub fn remove_internal(&mut self) -> Result<()> {
        self.arena.remove_internal()
    }

    pub fn polygons(&self) -> Result<Vec<(PolytopeId, Vec<Polygon>)>> {
        self.arena.polygons(true)
    }
}

#[derive(Debug, Clone)]
pub struct Puzzle {
    data: PuzzleData,
    ty: Arc<PuzzleType>,
}
impl PuzzleState for Puzzle {
    fn ty(&self) -> &Arc<PuzzleType> {
        &self.ty
    }

    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        Box::new(self.clone())
    }

    fn twist(&mut self, twist: super::Twist) -> Result<(), &'static str> {
        match self.data.apply_twist(twist) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => Err("Twist was blocked"),
            _ => Err("Couldn't apply twist"),
        }
    }

    fn layer_from_twist_axis(&self, twist_axis: super::TwistAxis, piece: super::Piece) -> u8 {
        0
    }

    fn sticker_geometry(
        &self,
        sticker: super::Sticker,
        params: &super::StickerGeometryParams,
    ) -> Option<super::StickerGeometry> {
        let sticker = self.data.sticker_ids[sticker.0 as usize];
        let mut verts = vec![];
        let mut polygon_indices = vec![];
        // Including internal because sticker
        self.data
            .arena
            .polytope_polygons(sticker, false)
            .ok()?
            .iter()
            .for_each(|p| {
                if let Some(new_verts) = p
                    .verts
                    .iter()
                    .map(|v| params.project_4d(v))
                    .collect::<Option<Vec<_>>>()
                {
                    let i = verts.len() as u16;
                    verts.extend_from_slice(&new_verts);
                    let j = verts.len() as u16;
                    polygon_indices.push((i..j).collect_vec().into_boxed_slice());
                    polygon_indices.push((i..j).rev().collect_vec().into_boxed_slice());
                };
            });
        let poly_count = polygon_indices.len();
        Some(super::StickerGeometry {
            verts,
            polygon_indices,
            polygon_twists: vec![
                super::ClickTwists {
                    cw: None,
                    ccw: None,
                    recenter: None,
                };
                poly_count
            ],
        })
    }

    fn is_solved(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Twist {
    pub layer: u8,
    pub transform: Matrix,
}
impl Twist {
    pub fn approx_eq(&self, other: Twist, epsilon: f32) -> bool {
        self.layer == other.layer && self.transform.approx_eq(&other.transform, epsilon)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub normal: Vector,
    pub distances: Vec<f32>,
    pub transforms: Vec<Matrix>,
}
impl Axis {
    fn add_transform(&mut self, transform: Matrix) {
        self.transforms.push(transform);
    }

    pub fn plane(&self, layer: usize) -> Hyperplane {
        Hyperplane {
            normal: self.normal.clone(),
            distance: self.distances[layer],
        }
    }

    pub fn layer_from_depth(&self, depth: f32) -> u8 {
        // distances is sorted in descending order
        self.distances
            .binary_search_by(|probe| depth.total_cmp(probe))
            .unwrap_or_else(|i| i) as u8
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistId {
    pub layer: usize,
    pub transform: usize,
}

pub struct AxisId(pub usize);

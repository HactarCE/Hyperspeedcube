use anyhow::{anyhow, Context, Result};
use approx::abs_diff_eq;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::spec::*;
use super::PuzzleShape;
use crate::math::*;
use crate::polytope::*;

use super::PuzzleState;
use super::PuzzleType;

const NO_INTERNAL: bool = true;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BasicPuzzleSpec {
    pub name: String,
    pub shape: ShapeSpec,
    pub twists: Vec<TwistsSpec>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TwistsSpec {
    #[serde(default)]
    pub symmetry: SymmetrySpecList,
    pub axes: Vec<AxisSpec>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AxisSpec {
    pub normal: Vector,
    pub cuts: Vec<f32>,
    #[serde(default)]
    pub twist_generators: Vec<String>,
}

impl BasicPuzzleSpec {
    pub fn build(&self) -> Result<PuzzleData> {
        // Build the base shape.
        let (shape, mut polytopes) = self.shape.build()?;

        // Slice for each layer of each twist axis.
        let mut axes = vec![];
        for twists_spec in &self.twists {
            for axis_spec in &twists_spec.axes {
                let normals = twists_spec.symmetry.generate(vec![axis_spec
                    .normal
                    .normalize()
                    .context("axis normal must not be zero")?])?;
                for normal in &normals {
                    for &distance in &axis_spec.cuts {
                        polytopes.slice_internal(&Hyperplane {
                            normal: normal.clone(),
                            distance,
                        })?;
                    }
                }
            }
        }

        let piece_ids = polytopes
            .roots
            .iter()
            .copied()
            // .filter(|&p| !polytopes.is_internal(p).expect("root did not exist"))
            .collect_vec();
        let sticker_ids = piece_ids
            .iter()
            .flat_map(|&p| {
                polytopes
                    .polytope_facet_ids(p, NO_INTERNAL)
                    .expect("bad children")
            })
            .collect_vec();

        let sticker_polys = sticker_ids
            .iter()
            .map(|&p| polytopes.polytope_polygons(p, NO_INTERNAL))
            .try_collect()?;

        Ok(PuzzleData {
            polytopes,
            piece_ids,
            sticker_ids,
            sticker_polys,
            axes,
            shape: Arc::new(shape),
        })
    }
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
            let new_transform = &(gen * &curr_twist.1) * &gen.inverse().context("bad matrix")?;
            let new_axis = Axis {
                normal: new_normal,
                distances: curr_axis.distances.clone(),
                transforms: vec![],
            };
            let new_i = (0..axes.len())
                .find(|&index| abs_diff_eq!(axes[index].normal, new_axis.normal))
                .unwrap_or_else(|| {
                    axes.push(new_axis);
                    axes.len() - 1
                });
            if transforms
                .iter()
                .all(|(i, t)| !(abs_diff_eq!(*t, new_transform) && *i == new_i))
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
    let ndim = spec.shape.ndim;
    let puzzle_data = spec.build()?;

    let mut piece_infos = vec![];
    let mut sticker_infos = vec![];
    for &piece in &puzzle_data.piece_ids {
        let i = sticker_infos.len() as u16;
        let stickers = puzzle_data
            .polytopes
            .polytope_facet_ids(piece, NO_INTERNAL)?;
        sticker_infos.extend(stickers.iter().map(|&id| {
            super::StickerInfo {
                piece: super::Piece(piece_infos.len() as u16),
                color: puzzle_data
                    .polytopes
                    .polytope_location(id)
                    .unwrap_or(super::Facet(0)),
            }
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
        shape: Arc::clone(&puzzle_data.shape),
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
            orientations: vec![Rotor::ident()],
        }),
        family_name: "Fun".to_string(),
        projection_type: match puzzle_data.shape.ndim {
            0..=3 => super::ProjectionType::_3D,
            _ => super::ProjectionType::_4D,
        },
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
    polytopes: PolytopeArena,
    piece_ids: Vec<PolytopeId>,
    sticker_ids: Vec<PolytopeId>,
    sticker_polys: Vec<Vec<Polygon>>,
    axes: Vec<Axis>,
    shape: Arc<PuzzleShape>,
}
impl PuzzleData {
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    pub fn apply_twist(&mut self, twist: super::Twist) -> Result<Result<(), Vec<PolytopeId>>> {
        let axis = &self.axes[twist.axis.0 as usize];
        let transform = &axis.transforms[twist.direction.0 as usize];
        let spans = self.polytopes.axis_spans(&axis.normal)?;
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
            self.polytopes.transform_polytope(p, &transform)?;
        }
        Ok(Ok(()))
    }

    pub fn remove_internal(&mut self) -> Result<()> {
        self.polytopes.remove_internal()
    }

    pub fn polygons(&self) -> Result<Vec<(PolytopeId, Vec<Polygon>)>> {
        self.polytopes.polygons(NO_INTERNAL)
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
        let mut verts = vec![];
        let mut polygon_indices = vec![];
        // Including internal because sticker
        self.data.sticker_polys[sticker.0 as usize]
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
impl approx::AbsDiffEq for Twist {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.transform.abs_diff_eq(&other.transform, epsilon)
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

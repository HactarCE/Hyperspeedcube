use anyhow::bail;
use anyhow::{Context, Result};
use approx::{abs_diff_eq, AbsDiffEq};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::Piece;
use super::PuzzleTwists;
use super::TwistAxisInfo;
use super::TwistDirectionInfo;
use super::{spec::*, TwistDirection};
use super::{PuzzleInfo, TwistCut};
use crate::math::*;
use crate::polytope::*;

use super::PuzzleState;
use super::PuzzleType;

const NO_INTERNAL: bool = true;

const MAX_TWIST_PERIOD: usize = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct JumblingPuzzleSpec {
    pub name: String,
    pub shape: ShapeSpec,
    #[serde(default)]
    pub twists: Vec<TwistsSpec>,
}

impl JumblingPuzzleSpec {
    pub fn build(&self) -> Result<Arc<PuzzleType>> {
        // Build the base shape.
        let (shape, mut polytopes) = self.shape.build()?;
        let twists = match self.twists.as_slice() {
            [] => PuzzleTwists {
                name: "none".to_string(),
                axes: vec![],
                directions: vec![],
                orientations: vec![Rotor::ident()],
            },
            [twists_spec] => twists_spec.build()?,
            _ => bail!("multiple twists specs is not yet implemented"),
        };
        let ndim = shape.ndim;

        // Slice for each layer of each twist axis.
        let mut axes = vec![];
        for twists_spec in &self.twists {
            for axis_spec in &twists_spec.axes {
                axes.push(Axis {
                    normal: axis_spec.normal.clone(),
                    distances: axis_spec.cuts.clone(),
                });
                let normals = twists_spec.symmetry.generate(
                    vec![axis_spec
                        .normal
                        .normalize()
                        .context("axis normal must not be zero")?],
                    |r, v| r * v,
                )?;
                for (_transform, normal) in normals {
                    for &distance in &axis_spec.cuts {
                        polytopes.slice_internal(&Hyperplane {
                            normal: normal.clone(),
                            distance,
                        })?;
                    }
                }
            }
        }

        let piece_polytope_ids = polytopes.roots.iter().copied().collect_vec();

        let mut piece_infos = vec![];
        let mut sticker_infos = vec![];
        for &piece in &piece_polytope_ids {
            let i = sticker_infos.len() as u16;
            let stickers = polytopes.polytope_facet_ids(piece, NO_INTERNAL)?;

            sticker_infos.extend(stickers.iter().map(|&id| {
                let IndexedPolygons { verts, polys } = polytopes
                    .polytope_indexed_polygons(id, NO_INTERNAL)
                    .unwrap_or_default();

                super::StickerInfo {
                    piece: super::Piece(piece_infos.len() as u16),
                    color: polytopes.polytope_location(id).unwrap_or(super::Facet(0)),

                    points: verts,
                    polygons: polys.into_iter().map(|p| p.0).collect(),
                    sticker_shrink_origin: Vector::EMPTY,
                }
            }));
            let j = sticker_infos.len() as u16;
            piece_infos.push(super::PieceInfo {
                stickers: (i..j).map(super::Sticker).collect(),
                piece_type: super::PieceType(0),

                points: polytopes.polytope_vertices(piece)?,
            })
        }

        let piece_count = piece_infos.len();

        Ok(Arc::new_cyclic(|this| PuzzleType {
            this: this.clone(),
            name: self.name.clone(),
            shape: Arc::new(shape),
            twists: Arc::new(twists),
            family_name: "Fun".to_string(),
            projection_type: match ndim {
                0..=3 => super::ProjectionType::_3D,
                _ => super::ProjectionType::_4D,
            },
            layer_count: 9,
            pieces: piece_infos,
            stickers: sticker_infos,
            piece_types: vec![super::PieceTypeInfo::new("Piece".to_string())],
            scramble_moves_count: 100,
            notation: super::NotationScheme {
                axis_names: vec![],
                direction_names: vec![],
                block_suffix: None,
                aliases: vec![],
            },
            new: Box::new(move |ty| {
                Box::new(JumblingPuzzle {
                    ty,
                    piece_states: vec![Rotoreflector::ident(); piece_count],
                })
            }),
        }))
    }
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
impl TwistsSpec {
    pub fn build(&self) -> Result<PuzzleTwists> {
        let mut axes = vec![];
        let mut directions = vec![];

        let mut sym = 'A';
        for axis in &self.axes {
            let base_frame: Rotoreflector = Rotor::from_vec_to_vec(Vector::unit(0), &axis.normal)
                .unwrap_or_else(|| {
                    Rotor::from_vec_to_vec(Vector::unit(1), &axis.normal).unwrap()
                        * Rotor::from_vec_to_vec(Vector::unit(0), Vector::unit(1)).unwrap()
                })
                .into();

            for (reference_frame, _normal) in self
                .symmetry
                .generate(vec![axis.normal.clone()], |r, v| r * v)?
            {
                axes.push(TwistAxisInfo {
                    symbol: sym.to_string(),
                    cuts: axis
                        .cuts
                        .iter()
                        .map(|&radius| TwistCut::Planar { radius })
                        .collect(),
                    opposite: None,

                    reference_frame: reference_frame * &base_frame,
                });
                sym = ((sym as u8) + 1) as char;
            }

            let reverse_base_frame = base_frame.reverse();

            let generators = self.symmetry.generators()?;
            let mut periodic_twists = axis
                .twist_generators
                .iter()
                .map(|s| {
                    PeriodicTwist::new(
                        parse_transform(s).with_context(|| format!("invalid transform: {s:?}"))?,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let mut unprocessed_idx = 0;
            while unprocessed_idx < periodic_twists.len() {
                for gen in &generators {
                    let old = &periodic_twists[unprocessed_idx];
                    let mut new = old.transform_by(gen);
                    if gen.is_reflection() {
                        new = new.reverse();
                    }
                    if !periodic_twists.iter().any(|old| abs_diff_eq!(*old, new)) {
                        periodic_twists.push(new);
                    }
                }
                unprocessed_idx += 1;
            }

            for periodic_twist in periodic_twists {
                let transforms = periodic_twist
                    .transforms
                    .iter()
                    .map(|t| reverse_base_frame.transform_rotoreflector_uninverted(t))
                    .collect_vec();

                let first = &transforms[0];
                if !abs_diff_eq!(first.matrix().col(0).to_vector(), Vector::unit(0)) {
                    continue; // does not preserve X axis
                }

                let i = directions.len();

                let transform_count = transforms.len();

                directions.extend(
                    transforms
                        .into_iter()
                        .enumerate()
                        .zip((0..transform_count).rev())
                        .map(|((idx, transform), rev_idx)| TwistDirectionInfo {
                            symbol: (i + idx).to_string(),
                            name: (i + idx).to_string(),
                            qtm: 1,
                            rev: TwistDirection((i + rev_idx) as u8),

                            transform,
                        }),
                );
            }
        }

        Ok(PuzzleTwists {
            name: "unnamed twist set".to_string(),
            axes,
            directions,
            orientations: vec![Rotor::ident()],
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PeriodicTwist {
    transforms: Vec<Rotoreflector>,
}
impl PeriodicTwist {
    fn new(r: Rotoreflector) -> Result<Self> {
        let transforms = std::iter::successors(Some(r.clone()), |a| {
            Some(&r * a).filter(|b| !abs_diff_eq!(*b, Rotoreflector::ident()))
        })
        .take(MAX_TWIST_PERIOD + 1)
        .collect_vec();
        if transforms.len() > MAX_TWIST_PERIOD {
            dbg!(transforms);
            bail!("nonperiodic twist (or period is too big)");
        }

        Ok(Self { transforms })
    }

    fn transform_by(&self, r: &Rotoreflector) -> Self {
        Self {
            transforms: self
                .transforms
                .iter()
                .map(|t| r.transform_rotoreflector_uninverted(t))
                .collect(),
        }
    }

    #[must_use]
    fn reverse(mut self) -> Self {
        self.transforms.reverse();
        self
    }
}
impl AbsDiffEq for PeriodicTwist {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.transforms[0].abs_diff_eq(other.transforms.first().unwrap(), epsilon)
            || self.transforms[0].abs_diff_eq(other.transforms.last().unwrap(), epsilon)
    }
}

#[derive(Debug, Clone)]
pub struct JumblingPuzzle {
    ty: Arc<PuzzleType>,
    piece_states: Vec<Rotoreflector>,
}
impl PuzzleState for JumblingPuzzle {
    fn ty(&self) -> &Arc<PuzzleType> {
        &self.ty
    }

    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        Box::new(self.clone())
    }

    fn twist(&mut self, twist: super::Twist) -> Result<(), &'static str> {
        let reference_frame = &self.ty.info(twist.axis).reference_frame;
        let transform = reference_frame
            .transform_rotoreflector_uninverted(&self.ty.info(twist.direction).transform);
        for piece in (0..self.ty.pieces.len() as u16).map(Piece) {
            if twist.layers[self.layer_from_twist_axis(twist.axis, piece)] {
                self.piece_states[piece.0 as usize] =
                    &transform * &self.piece_states[piece.0 as usize];
            }
        }
        Ok(())
    }

    fn piece_transform(&self, p: super::Piece) -> Matrix {
        self.piece_states[p.0 as usize].matrix().clone()
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
}
impl Axis {
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

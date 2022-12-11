//! Jumbling puzzle engine.

use ahash::AHashMap;
use anyhow::bail;
use anyhow::{Context, Result};
use approx::{abs_diff_eq, AbsDiffEq};
use itertools::Itertools;
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::sync::Arc;

use super::{spec::*, *};
use crate::math::*;
use crate::polytope::*;

const NO_INTERNAL: bool = true;

const MAX_TWIST_PERIOD: usize = 10;

/// Specification for a jumbling puzzle.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct JumblingPuzzleSpec {
    /// Human-friendly name of the puzzle.
    pub name: String,
    /// Puzzle shape specification.
    pub shape: ShapeSpec,
    /// Puzzle twists specifications.
    #[serde(default)]
    pub twists: TwistsSpec,
}

impl JumblingPuzzleSpec {
    /// Constructs a puzzle type from its spec.
    pub fn build(&self) -> Result<Arc<PuzzleType>> {
        // Build the base shape.
        let (shape, mut polytopes) = self.shape.build()?;
        let twists = self.twists.build()?;
        let ndim = shape.ndim;

        // Slice for each layer of each twist axis.
        for axis in &twists.axes {
            for cut in &axis.cuts {
                let TwistCut::Planar { radius } = cut;
                polytopes.slice_internal(&Hyperplane {
                    normal: axis.normal.clone(),
                    distance: *radius,
                })?;
            }
        }

        let mut piece_infos = vec![];
        let mut sticker_infos = vec![];
        for piece in polytopes.roots() {
            let shrink_vectors = piece.shrink_vectors(ShrinkStrategy::default())?;

            let i = sticker_infos.len() as u16;
            for sticker in piece.children()? {
                let color = match sticker.facet_set()?.iter().next() {
                    Some(facet) => facet,
                    None if NO_INTERNAL => continue,
                    None => Facet(0), // TODO: make facet optional
                };

                let point_polytopes = sticker.descendent_points()?.collect_vec();

                // Map from the IDs used by `PolytopeArena` to the IDs within
                // this sticker.
                let point_id_map: AHashMap<PolytopeRef<'_>, u16> = point_polytopes
                    .iter()
                    .enumerate()
                    .map(|(i, &vert)| (vert, i as u16))
                    .collect();

                let points = point_polytopes
                    .iter()
                    .map(|point| point.point().cloned())
                    .try_collect()?;

                let shrink_vectors = point_polytopes
                    .iter()
                    .map(|&point| {
                        shrink_vectors
                            .get(sticker, point)
                            .cloned()
                            .context("missing shrink vector")
                    })
                    .try_collect()?;

                let polygons = sticker
                    .descendents_with_rank_at_least(2)?
                    .into_iter()
                    .filter(|p| p.rank() == 2)
                    .map(|polygon| {
                        polygon
                            .polygon_verts()?
                            .map(|point| point_id_map.get(&point).copied().context("missing point"))
                            .collect()
                    })
                    .try_collect()?;

                sticker_infos.push(StickerInfo {
                    piece: Piece(piece_infos.len() as u16),
                    color,

                    points,
                    shrink_vectors,
                    polygons,
                });
            }
            let j = sticker_infos.len() as u16;

            piece_infos.push(PieceInfo {
                stickers: (i..j).map(Sticker).collect(),
                piece_type: PieceType(0),

                points: piece
                    .descendent_points()?
                    .map(|point| point.point().cloned())
                    .try_collect()?,
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
                0..=3 => ProjectionType::_3D,
                _ => ProjectionType::_4D,
            },
            layer_count: 9,
            pieces: piece_infos,
            stickers: sticker_infos,
            piece_types: vec![PieceTypeInfo {
                name: "Piece".to_string(),
            }],
            scramble_moves_count: 100,
            notation: NotationScheme {
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

/// Specification for a set of twists.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TwistsSpec {
    /// Human-friendly name of the twists set.
    pub name: Option<String>,
    /// Symmetry for the set of twists.
    #[serde(default)]
    pub symmetry: SymmetrySpecList,
    /// Twist axis specifications.
    pub axes: Vec<AxisSpec>,
}
impl Default for TwistsSpec {
    fn default() -> Self {
        Self {
            name: Some("none".to_string()),
            symmetry: SymmetrySpecList(vec![]),
            axes: vec![],
        }
    }
}
impl TwistsSpec {
    /// Constructs a twist set from its spec.
    pub fn build(&self) -> Result<PuzzleTwists> {
        let mut axes = vec![];
        let mut directions = vec![];

        let mut namer = Namer {
            type_of_thing: "twist axis",
            prefix_iter: crate::util::letters_upper(),
            by_name: AHashMap::new(),
        };
        for axis in &self.axes {
            for pair in axis.cuts.windows(2) {
                if pair[0] <= pair[1] {
                    bail!(
                        "cuts must be sorted by depth from largest to smallest: {:?}",
                        axis.cuts,
                    );
                }
            }

            let base_frame: Rotoreflector = Rotor::from_vec_to_vec(Vector::unit(0), &axis.normal)
                .unwrap_or_else(|| {
                    Rotor::from_vec_to_vec(Vector::unit(1), &axis.normal).unwrap()
                        * Rotor::from_vec_to_vec(Vector::unit(0), Vector::unit(1)).unwrap()
                })
                .into();

            let seed_normal = axis
                .normal
                .normalize()
                .context("axis normal must not be zero")?;
            let normals = self.symmetry.generate([seed_normal], |r, v| r * v)?;
            let axis_ids = axes.len()..axes.len() + normals.len();
            let names = namer.with_names(&axis.names, axis_ids.map(|i| TwistAxis(i as _)))?;

            for ((name, _axis_id), (reference_frame, normal)) in names.into_iter().zip(normals) {
                axes.push(TwistAxisInfo {
                    symbol: name.to_string(),
                    cuts: axis
                        .cuts
                        .iter()
                        .map(|&radius| TwistCut::Planar { radius })
                        .collect(),
                    opposite: None,

                    normal,
                    reference_frame: (reference_frame * &base_frame).reverse(),
                });
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

/// Specification for a set of identical twist axes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AxisSpec {
    /// Twist axis normal vector.
    pub normal: Vector,
    /// Cut depths from the origin, sorting from outermost (positive) to
    /// innermost (negative).
    #[serde(deserialize_with = "deserialize_cut_depths")]
    pub cuts: Vec<f32>,
    /// Twist generators.
    #[serde(default)]
    pub twist_generators: Vec<String>,

    /// Twist axis names.
    #[serde(flatten)]
    pub names: NameSetSpec,
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
struct JumblingPuzzle {
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

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        let reference_frame = &self.ty.info(twist.axis).reference_frame;
        let transform = reference_frame
            .reverse()
            .transform_rotoreflector_uninverted(&self.ty.info(twist.direction).transform);
        for piece in (0..self.ty.pieces.len() as u16).map(Piece) {
            if twist.layers[self.layer_from_twist_axis(twist.axis, piece)] {
                self.piece_states[piece.0 as usize] =
                    &transform * &self.piece_states[piece.0 as usize];
            }
        }
        Ok(())
    }

    fn piece_transform(&self, p: Piece) -> Matrix {
        self.piece_states[p.0 as usize]
            .matrix()
            .at_ndim(self.ty.ndim())
    }

    fn is_solved(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
struct JumblingTwist {
    layer: u8,
    transform: Matrix,
}
impl approx::AbsDiffEq for JumblingTwist {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        crate::math::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.transform.abs_diff_eq(&other.transform, epsilon)
    }
}

fn parse_cut_depths(strings: &[impl AsRef<str>]) -> Result<Vec<f32>> {
    lazy_static! {
        // ^([^ ]+)\s+from\s+([^ ]+)\s+to\s+([^ ]+)$
        // ^                                       $    match whole string
        //  ([^ ]+)          ([^ ]+)        ([^ ]+)     match numbers
        //         \s+from\s+       \s+to\s+            match literal words
        static ref CUT_SEQ_REGEX: Regex =
            Regex::new(r#"^([^ ]+)\s+from\s+([^ ]+)\s+to\s+([^ ]+)$"#).unwrap();
    }

    let mut ret = vec![];
    for s in strings {
        let s = s.as_ref().trim();
        if let Some(captures) = CUT_SEQ_REGEX.captures(s) {
            let n = parse_u8_cut_count(&captures[1])?;
            let a = parse_f32(&captures[2])?;
            let b = parse_f32(&captures[3])?;
            ret.extend(
                (1..=n)
                    .map(|i| i as f32 / (n + 1) as f32)
                    .map(|t| util::mix(a, b, t)),
            )
        } else if let Ok(n) = parse_f32(s) {
            ret.push(n)
        } else {
            bail!("expected floating-point number or range 'N from A to B'");
        }
    }
    Ok(ret)
}

fn parse_u8_cut_count(s: &str) -> Result<u8> {
    s.trim()
        .parse()
        .with_context(|| format!("expected integer number of cuts; got {s:?}"))
}
fn parse_f32(s: &str) -> Result<f32> {
    s.trim()
        .parse()
        .with_context(|| format!("expected floating-point number; got {s:?}"))
}

fn deserialize_cut_depths<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<f32>, D::Error> {
    parse_cut_depths(&Vec::<String>::deserialize(deserializer)?).map_err(D::Error::custom)
}

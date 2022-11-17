use anyhow::bail;
use anyhow::{Context, Result};
use approx::{abs_diff_eq, AbsDiffEq};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tinyset::Set64;

use super::PuzzleTwists;
use super::TwistAxisInfo;
use super::TwistDirectionInfo;
use super::{spec::*, TwistDirection};
use super::{Facet, Piece};
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

        let piece_centers = polytopes.compute_centroids()?;
        let adj_facets = polytopes.adj_facets()?;
        let mut facet_meet_cache: HashMap<Set64<Facet>, FacetMeet> = HashMap::new();

        let piece_polytope_ids = polytopes.roots.iter().copied().collect_vec();

        let mut piece_infos = vec![];
        let mut sticker_infos = vec![];
        for &piece in &piece_polytope_ids {
            let i = sticker_infos.len() as u16;
            let stickers = polytopes.polytope_facet_ids(piece, NO_INTERNAL)?;
            let piece_facets: Set64<Facet> = stickers
                .iter()
                .filter_map(|&s| polytopes.polytope_location(s).ok())
                .collect();
            let adj_facets: BTreeMap<PolytopeId, Set64<Facet>> = adj_facets
                .iter()
                .map(|(&p, facets)| {
                    (
                        p,
                        facets.iter().filter(|f| piece_facets.contains(f)).collect(),
                    )
                })
                .collect();

            let piece_center = piece_centers.get(&piece).context("missing piece center")?;

            for &id in &stickers {
                let color = polytopes.polytope_location(id).unwrap_or(super::Facet(0));
                let mut points: Vec<Vector> = vec![];
                let mut shrink_vectors: Vec<Vector> = vec![];
                let mut vertex_map = HashMap::new();
                let mut polygons: Vec<Vec<u16>> = vec![];
                for point_ids in polytopes.polytope_polygons(id, NO_INTERNAL)? {
                    let mut polygon = vec![];
                    for point_id in point_ids {
                        let vertex_id_within_sticker =
                            vertex_map.entry(point_id).or_insert_with(|| {
                                let vector =
                                    polytopes.get_point(point_id).expect("TODO: don't panic");
                                points.push(vector.clone());

                                let shrink_vector = piece_center - vector;
                                shrink_vectors.push(
                                    // 1. Get all the facets that contain this point.
                                    if let Some(facet_set) = adj_facets.get(&point_id) {
                                        // 2. Find all polytopes contained by a
                                        //    superset of those facets.
                                        let mut desc = HashSet::new();
                                        polytopes
                                            .add_descendents_to_set(piece, &mut desc)
                                            .expect("TODO don't panic");
                                        let polytopes_with_superset = desc
                                            .iter()
                                            .filter_map(|p| Some((p, adj_facets.get(p)?)))
                                            .filter(|(_, adj)| {
                                                facet_set.iter().all(|f| adj.contains(f))
                                            })
                                            .collect_vec();
                                        // 3. Within each rank, get the
                                        //    polytopes with the largest
                                        //    superset.
                                        let polytopes_with_largest_superset_per_rank = (0..ndim)
                                            .map(|rank| {
                                                let size_of_largest_superset =
                                                    polytopes_with_superset
                                                        .iter()
                                                        .filter(|(&p, _)| {
                                                            polytopes.get_rank(p).unwrap() == rank
                                                        })
                                                        .map(|(_, adj)| adj.len())
                                                        .max()
                                                        .unwrap_or(0);
                                                polytopes_with_superset
                                                    .iter()
                                                    .filter(|(&p, adj)| {
                                                        polytopes.get_rank(p).unwrap() == rank
                                                            && adj.len() == size_of_largest_superset
                                                    })
                                                    .collect_vec()
                                            })
                                            .collect_vec();

                                        // I'm feeling lazy, so average the
                                        // points with the largest superset.
                                        let center = polytopes_with_largest_superset_per_rank[0]
                                            .iter()
                                            .map(|(&p, _)| polytopes.get_point(p).unwrap())
                                            .sum::<Vector>()
                                            / polytopes_with_largest_superset_per_rank[0].len()
                                                as f32;

                                        center - vector
                                    } else {
                                        shrink_vector
                                    },
                                );

                                points.len() as u16 - 1
                            });

                        polygon.push(*vertex_id_within_sticker);
                    }
                    polygons.push(polygon);
                }

                sticker_infos.push(super::StickerInfo {
                    piece: super::Piece(piece_infos.len() as u16),
                    color,

                    points,
                    shrink_vectors,
                    polygons,
                });
            }
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
            for pair in axis.cuts.windows(2) {
                if pair[0] <= pair[1] {
                    bail!("cuts must be sorted by depth: {:?}", axis.cuts);
                }
            }

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

                    reference_frame: (reference_frame * &base_frame).reverse(),
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

    fn piece_transform(&self, p: super::Piece) -> Matrix {
        self.piece_states[p.0 as usize]
            .matrix()
            .at_ndim(self.ty.ndim())
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

/// Subspace at the intersection of a set of facets.
struct FacetMeet {
    /// Dual of that intersection.
    dual: Multivector,
}
impl FacetMeet {
    pub fn from_normals<V: VectorRef>(normals: impl IntoIterator<Item = V>) -> Self {
        // The intersection ("meet") is the dual of the exterior product. This
        // dual is much easier to work with in this case.

        let dual = normals
            .into_iter()
            .fold(Multivector::scalar(1.0), |m, normal| {
                // Compute the exterior product.
                let new_result = &m ^ &Multivector::from(normal);
                // If the exterior product is zero, then the new normal is
                // parallel to `m` so we don't need it.
                if new_result.is_approx_zero() {
                    m
                } else {
                    new_result
                }
            })
            .normalize()
            .unwrap_or(Multivector::scalar(1.0));

        Self { dual }
    }

    /// Projects a vector onto the subspace.
    pub fn project_vector(&self, vector: impl VectorRef) -> Vector {
        let ret = ((&self.dual ^ &Multivector::from(&vector)) * self.dual.conjugate())
            .grade_project_to_vector();
        dbg!(ret.mag(), vector.mag());
        ret
    }
}

use itertools::Itertools;
use slab::Slab;
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

use crate::math::*;

pub type PolytopeResult<T> = Result<T, PolytopeError>;

const EPSILON: f32 = 0.001;

/// Generates a polytope from a set of generators and base facets.
pub fn generate_polytope(
    ndim: u8,
    generators: &[Matrix<f32>],
    base_facets: &[Vector<f32>],
) -> PolytopeResult<Vec<Polygon>> {
    let radius = base_facets
        .iter()
        .map(|pole| pole.mag())
        .reduce(f32::max)
        .expect("no base facets");
    let initial_radius = radius * 2.0 * ndim as f32;
    // TODO: check if radius is too small (any original point remains).
    let mut arena = PolytopeArena::new_cube(ndim, initial_radius);

    let mut facet_poles: Vec<Vector<f32>> = base_facets.to_vec();
    let mut next_unprocessed = 0;
    while next_unprocessed < facet_poles.len() {
        for gen in generators {
            let new_pole = gen.transform(facet_poles[next_unprocessed].clone().resize(ndim));
            if facet_poles
                .iter()
                .all(|pole| !pole.approx_eq(&new_pole, EPSILON))
            {
                facet_poles.push(new_pole);
            }
        }
        next_unprocessed += 1;
    }
    for pole in &facet_poles {
        arena.slice_by_plane(
            &Hyperplane {
                normal: pole.normalise(),
                distance: pole.mag(),
            },
            true,
        )?;
    }
    arena.polygons()
}

/// Arena of polytopes that can be split.
pub struct PolytopeArena {
    /// Unordered set of polytopes.
    polytopes: Slab<Polytope>,
    /// Root polytopes.
    roots: Vec<PolytopeId>,
}
impl fmt::Debug for PolytopeArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PolytopeArena")
            .field("polytopes", &self.polytopes.iter().collect_vec())
            .field("roots", &self.roots)
            .finish()
    }
}
impl PolytopeArena {
    /// Constructs a polytope arena containing a hypercube.
    pub fn new_cube(ndim: u8, radius: f32) -> Self {
        // Based on Andrey Astrelin's implementation of `GenCube()` in MPUlt
        // (FaceCuts.cs)

        // Make a 3^NDIM grid of polytopes to construct a hypercube. The corners
        // are vertices. Between those are edges, etc.
        //
        // ```
        // • - •
        // | # |
        // • - •
        // ```

        let mut ret = Self {
            polytopes: Slab::new(),
            roots: vec![PolytopeId(3_u32.pow(ndim as _) / 2)], // center of the 3^NDIM cube
        };

        let powers_of_3 = || std::iter::successors(Some(1), |x| Some(x * 3));

        for i in 0..3_u32.pow(ndim as _) {
            let rank = base_3_expansion(i, ndim)
                .filter(|&digit| digit == 1)
                .count() as u8;

            let contents = if rank == 0 {
                // This is a vertex.
                let point = base_3_expansion(i, ndim)
                    .map(|digit| (digit as f32 - 1.0) * radius)
                    .collect();
                PolytopeContents::new_point(point)
            } else {
                // This is a branch node.
                let children = powers_of_3()
                    .zip(base_3_expansion(i, ndim))
                    // For each axis we are straddling ...
                    .filter(|&(_, digit)| digit == 1)
                    // ... add two children along that axis.
                    .flat_map(|(power_of_3, _)| {
                        [
                            PolytopeId(i - power_of_3 as u32),
                            PolytopeId(i + power_of_3 as u32),
                        ]
                    })
                    .collect();
                PolytopeContents::new_branch(rank, children)
            };

            let parents = powers_of_3()
                .zip(base_3_expansion(i, ndim))
                // For each axis we are not straddling ...
                .filter(|&(_, digit)| digit != 1)
                // ... add the parent that straddles that axis.
                .map(|(power_of_3, digit)| i - power_of_3 * digit + power_of_3)
                .map(PolytopeId)
                .collect();

            ret.add(Polytope { parents, contents });
        }

        ret
    }

    /// Returns a polytope by ID.
    fn get(&self, id: PolytopeId) -> PolytopeResult<&Polytope> {
        self.polytopes
            .get(id.0 as _)
            .ok_or(PolytopeError::NullPolytope)
    }
    /// Returns a mutable reference to a polytope by ID.
    fn get_mut(&mut self, id: PolytopeId) -> PolytopeResult<&mut Polytope> {
        self.polytopes
            .get_mut(id.0 as _)
            .ok_or(PolytopeError::NullPolytope)
    }

    /// Adds a polytope to the arena.
    fn add(&mut self, polytope: Polytope) -> PolytopeId {
        let idx = self.polytopes.insert(polytope);
        PolytopeId(idx as _)
    }
    /// Adds a point to the arena.
    fn add_point(&mut self, point: Vector<f32>, slice_result: Option<SliceResult>) -> PolytopeId {
        self.add(Polytope {
            parents: smallvec![],
            contents: PolytopeContents::Point {
                point,
                slice_result,
            },
        })
    }
    /// Adds a non-point polytope to the arena.
    fn add_branch(
        &mut self,
        rank: u8,
        children: SmallVec<[PolytopeId; 4]>,
        slice_result: Option<SliceResult>,
    ) -> PolytopeResult<PolytopeId> {
        if children.is_empty() {
            return Err(PolytopeError::BadChildCount {
                rank,
                child_count: 0,
            });
        }

        let ret = self.add(Polytope {
            parents: smallvec![],
            contents: PolytopeContents::Branch {
                rank,
                children: children.clone(),
                slice_result,
            },
        });

        for child in children {
            let child = self.get_mut(child)?;
            if child.rank() + 1 != rank {
                return Err(PolytopeError::BadChildRank {
                    parent_rank: rank,
                    child_rank: child.rank(),
                });
            }
            child.parents.push(ret);
        }
        Ok(ret)
    }
    /// Adds a child to a parent polytope, and adds the parent to the child.
    fn add_child(&mut self, parent: PolytopeId, child: PolytopeId) -> PolytopeResult<()> {
        let parent = self.get_mut(parent)?;
        match &mut parent.contents {
            PolytopeContents::Point { .. } => Err(PolytopeError::BadChildRank {
                parent_rank: parent.rank(),
                child_rank: self.get(child)?.rank(),
            }),
            PolytopeContents::Branch { children, .. } => {
                children.push(child);
                self.get_mut(child)?.parents.push(child);
                Ok(())
            }
        }
    }
    /// Recursively delete a polytope.
    fn delete_polytope(&mut self, id: PolytopeId) {
        if let Ok(children) = self.get(id).and_then(|p| p.children().cloned()) {
            for child in children {
                self.delete_polytope(child);
            }
        }
        self.polytopes.try_remove(id.0 as usize);
    }
    /// Returns a list of all polygons (rank-2 polytopes) in the arena.
    pub fn polygons(&self) -> PolytopeResult<Vec<Polygon>> {
        self.polytopes
            .iter()
            .filter(|(_idx, p)| p.rank() == 2)
            // For each polygon ...
            .map(|(_idx, p)| {
                // Get a list of edges in no particular order.
                let edges: Vec<[PolytopeId; 2]> =
                    p.children()?
                        .iter()
                        .map(|&p| -> PolytopeResult<[PolytopeId; 2]> {
                            let edge = self.get(p)?;
                            let endpoints = edge.children()?;
                            // Unpack the edge into the point on either end.
                            let [a, b] = *<&[PolytopeId; 2]>::try_from(endpoints.as_slice())
                                .map_err(|_| PolytopeError::BadChildCount {
                                    rank: 1,
                                    child_count: endpoints.len(),
                                })?;
                            Ok([a, b])
                        })
                        .try_collect()?;

                // Now we will assemble a list of vertices in order.
                let mut verts = Vec::with_capacity(edges.len());

                // Make an adjacency list representing the graph of the polygon.
                let mut adj: HashMap<PolytopeId, SmallVec<[PolytopeId; 2]>> = HashMap::new();
                for &[a, b] in edges.iter() {
                    adj.entry(a).or_default().push(b);
                    adj.entry(b).or_default().push(a);
                }

                let [mut prev, mut current] =
                    edges.first().ok_or(PolytopeError::BadChildCount {
                        rank: 2,
                        child_count: 0,
                    })?;
                let first_vertex = prev;
                verts.push(self.get(current)?.unwrap_point()?.clone());
                while current != first_vertex {
                    let new = adj
                        .get(&current)
                        .unwrap()
                        .iter()
                        .copied()
                        .find(|&v| v != prev)
                        .ok_or(PolytopeError::BadPolygon)?;
                    prev = current;
                    current = new;
                    verts.push(self.get(current)?.unwrap_point()?.clone());
                }

                Ok(Polygon { verts })
            })
            .collect()
    }

    /// Slices the polytope by a hyperplane, removing external parts if carving.
    pub fn slice_by_plane(&mut self, plane: &Hyperplane, carving: bool) -> PolytopeResult<()> {
        for root in std::mem::take(&mut self.roots) {
            match self.slice_polytope(root, plane)? {
                SliceResult::Above => {
                    if carving {
                        self.delete_polytope(root);
                    } else {
                        self.roots.push(root);
                    }
                }
                SliceResult::Below => self.roots.push(root),
                SliceResult::Split { above, below } => {
                    if carving {
                        self.delete_polytope(above);
                    } else {
                        self.roots.push(above);
                    }
                    self.roots.push(below);
                }
                SliceResult::New { .. } => return Err(PolytopeError::BadSliceResult),
            };
        }

        self.polytopes.retain(|_idx, polytope| {
            match polytope.slice_result() {
                // Remove dead polytopes.
                Some(SliceResult::Above) if carving => false,
                Some(SliceResult::Split { .. }) => false,
                // Reset slice results.
                _ => {
                    polytope.reset_slice_result();
                    true
                }
            }
        });
        Ok(())
    }

    fn slice_polytope(&mut self, p: PolytopeId, plane: &Hyperplane) -> PolytopeResult<SliceResult> {
        let polytope = self.get(p)?;

        if let Some(ret) = polytope.slice_result() {
            return Ok(ret);
        }

        let ret = match &polytope.contents {
            PolytopeContents::Point { point, .. } => {
                if plane.distance_to(point) < EPSILON {
                    SliceResult::Below
                } else {
                    SliceResult::Above
                }
            }
            PolytopeContents::Branch { rank, children, .. } => {
                let rank = *rank;
                let old_children = children.clone();
                let mut children_above: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut children_below: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut intersection_children_above = smallvec![];
                let mut intersection_children_below = smallvec![];

                for child in old_children {
                    match self.slice_polytope(child, plane)? {
                        SliceResult::Above => children_above.push(child),
                        SliceResult::Below => children_below.push(child),
                        SliceResult::Split { above, below } => {
                            children_above.push(above);
                            children_below.push(below);
                            intersection_children_above
                                .push(self.get(above)?.intersection_child()?);
                            intersection_children_below
                                .push(self.get(below)?.intersection_child()?);
                        }
                        SliceResult::New { .. } => return Err(PolytopeError::BadSliceResult),
                    }
                }

                if rank == 1 {
                    match (children_above.as_slice(), children_below.as_slice()) {
                        // Both children are above.
                        ([_, _], []) => SliceResult::Above,
                        // Children are on opposite sides.
                        ([a], [b]) => {
                            let v1 = self.get(*a)?.unwrap_point()?;
                            let v2 = self.get(*b)?.unwrap_point()?;
                            let v1_distance = -plane.distance_to(v1);
                            let v2_distance = plane.distance_to(v2);
                            let sum = v1_distance + v2_distance;
                            let intersection = (v2 * v1_distance + v1 * v2_distance) / sum;
                            let intersection_above = self.add_point(intersection.clone(), None);
                            let intersection_below = self.add_point(intersection, None);
                            SliceResult::Split {
                                above: self.add_branch(
                                    1,
                                    smallvec![*a, intersection_above],
                                    Some(SliceResult::New {
                                        intersection: intersection_above,
                                    }),
                                )?,
                                below: self.add_branch(
                                    1,
                                    smallvec![*b, intersection_below],
                                    Some(SliceResult::New {
                                        intersection: intersection_below,
                                    }),
                                )?,
                            }
                        }
                        // Both children are below.
                        ([], [_, _]) => SliceResult::Below,
                        _ => return Err(PolytopeError::BadEdge),
                    }
                } else {
                    match (children_above.as_slice(), children_below.as_slice()) {
                        // No children
                        ([], []) => {
                            return Err(PolytopeError::BadChildCount {
                                rank,
                                child_count: 0,
                            })
                        }
                        // All children are above.
                        (_, []) => SliceResult::Above,
                        // All children are below.
                        ([], _) => SliceResult::Below,
                        // Children are on both sides.
                        (_, _) => {
                            if intersection_children_above.len() < 2
                                || intersection_children_below.len() < 2
                            {
                                return Err(PolytopeError::BadIntersection);
                            }
                            let intersection_above =
                                self.add_branch(rank - 1, intersection_children_above, None)?;
                            let intersection_below =
                                self.add_branch(rank - 1, intersection_children_below, None)?;
                            children_above.push(intersection_above);
                            children_below.push(intersection_below);
                            let above = self.add_branch(
                                rank,
                                children_above,
                                Some(SliceResult::New {
                                    intersection: intersection_above,
                                }),
                            )?;
                            let below = self.add_branch(
                                rank,
                                children_below,
                                Some(SliceResult::New {
                                    intersection: intersection_below,
                                }),
                            )?;
                            self.get_mut(above)?.set_slice_result(SliceResult::New {
                                intersection: intersection_above,
                            });
                            self.get_mut(below)?.set_slice_result(SliceResult::New {
                                intersection: intersection_below,
                            });
                            SliceResult::Split { above, below }
                        }
                    }
                }
            }
        };
        self.get_mut(p)?.set_slice_result(ret);
        Ok(ret)
    }
}

/// Index of a polytope in a polytope arena.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct PolytopeId(u32);

/// Node in the polytope tree.
#[derive(Debug, Clone, PartialEq)]
struct Polytope {
    contents: PolytopeContents,
    parents: SmallVec<[PolytopeId; 4]>,
}
impl Polytope {
    /// Returns the rank (number of dimensions) of the polytope. A point has
    /// rank 0, a line has rank 1, etc.
    fn rank(&self) -> u8 {
        self.contents.rank()
    }
    /// Returns the result of the most recent slicing operation.
    fn slice_result(&self) -> Option<SliceResult> {
        match &self.contents {
            PolytopeContents::Point { slice_result, .. }
            | PolytopeContents::Branch { slice_result, .. } => *slice_result,
        }
    }
    /// Returns the result of the current slicing operation.
    fn set_slice_result(&mut self, new_slice_result: SliceResult) {
        match &mut self.contents {
            PolytopeContents::Point { slice_result, .. }
            | PolytopeContents::Branch { slice_result, .. } => {
                *slice_result = Some(new_slice_result)
            }
        }
    }
    /// Resets the result of the current slicing operation.
    fn reset_slice_result(&mut self) {
        match &mut self.contents {
            PolytopeContents::Point { slice_result, .. }
            | PolytopeContents::Branch { slice_result, .. } => *slice_result = None,
        }
    }

    /// Returns the coordinate point if this polytope is a point, or an error if
    /// is a branch.
    fn unwrap_point(&self) -> PolytopeResult<&Vector<f32>> {
        match &self.contents {
            PolytopeContents::Point { point, .. } => Ok(point),
            _ => Err(PolytopeError::ExpectedPoint { rank: self.rank() }),
        }
    }
    /// Returns the children of the polytope if it is a branch, or an error if
    /// it is a point.
    fn children(&self) -> PolytopeResult<&SmallVec<[PolytopeId; 4]>> {
        match &self.contents {
            PolytopeContents::Point { .. } => Err(PolytopeError::ExpectedBrach),
            PolytopeContents::Branch { children, .. } => Ok(children),
        }
    }
    /// Returns the intersection between the polytope and the slicing hyperplane.
    fn intersection_child(&self) -> PolytopeResult<PolytopeId> {
        match self.slice_result() {
            Some(SliceResult::New { intersection }) => return Ok(intersection),
            _ => {
                dbg!(self);
                todo!()
            }
        }
    }
    /// Returns a mutable reference to the children of the polytope if it is a
    /// branch, or an error if it is a point.
    fn children_mut(&mut self) -> PolytopeResult<&mut SmallVec<[PolytopeId; 4]>> {
        match &mut self.contents {
            PolytopeContents::Point { .. } => Err(PolytopeError::ExpectedBrach),
            PolytopeContents::Branch { children, .. } => Ok(children),
        }
    }
}

/// Contents of a polytope, either a point or a branch.
///
/// `slice_result` is included here instead of in `Polytope` because it makes
/// the struct smaller, somehow. (Something to do with enum tag optimizations,
/// probably?)
#[derive(Debug, Clone, PartialEq)]
enum PolytopeContents {
    Point {
        point: Vector<f32>,
        slice_result: Option<SliceResult>,
    },
    Branch {
        rank: u8,
        children: SmallVec<[PolytopeId; 4]>,
        slice_result: Option<SliceResult>,
    },
}
impl PolytopeContents {
    /// Constructs a point polytope.
    fn new_point(point: Vector<f32>) -> Self {
        Self::Point {
            point,
            slice_result: None,
        }
    }
    /// Constructs a non-point polytope.
    fn new_branch(rank: u8, children: SmallVec<[PolytopeId; 4]>) -> Self {
        Self::Branch {
            rank,
            children,
            slice_result: None,
        }
    }

    /// Returns the rank (number of dimensions) of the polytope. A point has
    /// rank 0, a line has rank 1, etc.
    fn rank(&self) -> u8 {
        match self {
            Self::Point { .. } => 0,
            Self::Branch { rank, .. } => *rank,
        }
    }
}

/// Result of slicing a polytope with a hyperplane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SliceResult {
    /// The whole polytope is above the slice.
    Above,
    /// The whole polytope is below the slice.
    Below,
    /// The polytope is cut by the slice.
    Split {
        above: PolytopeId,
        below: PolytopeId,
    },
    /// The polytope was produced by the slice.
    New { intersection: PolytopeId },
}

/// Error from doing polytope math.
#[derive(Error, Debug)]
pub enum PolytopeError {
    #[error("internal error: null polytope")]
    NullPolytope,
    #[error("internal error: orphaned polytope")]
    OrphanedPolytope,
    #[error("internal error: polytope with rank {rank} cannot have {child_count} children")]
    BadChildCount { rank: u8, child_count: usize },
    #[error(
        "internal error: polytope of rank {parent_rank} cannot have child of rank {child_rank}"
    )]
    BadChildRank { parent_rank: u8, child_rank: u8 },
    #[error("internal error: bad polygon")]
    BadPolygon,
    #[error("internal error: bad slice result")]
    BadSliceResult,
    #[error("internal error: bad intersection")]
    BadIntersection,
    #[error("internal error: bad edge")]
    BadEdge,
    #[error("internal error: expected point, got branch with rank {rank}")]
    ExpectedPoint { rank: u8 },
    #[error("internal error: expected branch, got point")]
    ExpectedBrach,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub verts: Vec<Vector<f32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hyperplane {
    /// Must be normalised.
    pub normal: Vector<f32>,
    pub distance: f32,
}
impl Hyperplane {
    /// Returns the position of the point on the hyperplane nearest the origin.
    fn pole(&self) -> Vector<f32> {
        &self.normal * self.distance
    }

    fn distance_to(&self, point: impl VectorRef<f32>) -> f32 {
        -(self.pole() - point).dot(&self.normal)
    }
}

fn base_3_expansion(n: u32, digit_count: u8) -> impl Iterator<Item = u32> {
    std::iter::successors(Some(n), |x| Some(x / 3))
        .take(digit_count as _)
        .map(|x| x % 3)
}

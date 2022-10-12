use anyhow::{anyhow, bail, ensure, Context, Result};
use approx::abs_diff_eq;
use itertools::Itertools;
use slab::Slab;
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{self};
use thiserror::Error;

use crate::math::*;
use crate::puzzle::Facet;

const EPSILON: f32 = 0.00001;
const SPLIT_MARGIN: f32 = EPSILON * 5000.;

const EXTRA_ASSERTIONS: bool = cfg!(debug_assertions);

/// Arena of polytopes which can be split.
///
/// Each piece is represented as an isolated DAG (directed acyclic graph). The
/// DAG is arranged into layers corresponding to polytope ranks. For example, a
/// cube is represented as follows:
///
/// - There are 8 rank-0 polytopes (points). Each has no children.
/// - There are 12 rank-1 polytopes (edges). Each has two points as children.
/// - There are 6 rank-2 polytopes (faces). Each has four edges as children.
/// - There is one rank-3 polytope (the whole cube). It has all six faces as
///   children and is the root of the graph.
///
/// Each point contains its location, but no other nodes in the graph contain
/// any geometric information. They are all purely topological.
#[derive(Clone)]
pub struct PolytopeArena {
    /// Unordered set of polytopes.
    polytopes: Slab<Polytope>,
    /// Root polytopes.
    pub(crate) roots: BTreeSet<PolytopeId>,
    /// Number of dimensions.
    ndim: u8,
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
    /// Constructs a polytope arena containing a hypercube. This hypercube can
    /// then be carved into any convex polytope of the same number of
    /// dimensions.
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

        let mut roots = BTreeSet::new();
        // The center of the 3^NDIM cube is the root.
        roots.insert(PolytopeId(3_u32.pow(ndim as _) / 2));
        let mut this = Self {
            polytopes: Slab::new(),
            roots,
            ndim,
        };

        let powers_of_3 = || std::iter::successors(Some(1), |x| Some(x * 3));

        for i in 0..3_u32.pow(ndim as _) {
            let rank = base_3_expansion(i, ndim)
                .filter(|&digit| digit == 1)
                .count() as u8;

            if rank == 0 {
                // This is a vertex.
                let point = base_3_expansion(i, ndim)
                    .map(|digit| (digit as f32 - 1.0) * radius)
                    .collect();
                this.add(Polytope::Point { point });
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
                this.add(Polytope::Branch {
                    rank,
                    location: (rank == ndim - 1).then_some(FacetLocation::Primordial),
                    children,
                });
            };
        }

        this
    }

    /// Returns a polytope by ID.
    fn get(&self, id: PolytopeId) -> Result<&Polytope, NullPolytope> {
        self.polytopes.get(id.0 as _).ok_or(NullPolytope)
    }
    /// Returns a mutable reference to a polytope by ID.
    fn get_mut(&mut self, id: PolytopeId) -> Result<&mut Polytope, NullPolytope> {
        self.polytopes.get_mut(id.0 as _).ok_or(NullPolytope)
    }
    /// Deletes a polytope nonrecursively.
    fn remove(&mut self, id: PolytopeId) -> Result<(), NullPolytope> {
        match self.polytopes.try_remove(id.0 as _) {
            Some(_) => Ok(()),
            None => Err(NullPolytope),
        }
    }

    /// Adds a polytope to the arena.
    fn add(&mut self, polytope: Polytope) -> PolytopeId {
        let idx = self.polytopes.insert(polytope);
        PolytopeId(idx as _)
    }
    /// Adds a point to the arena.
    fn add_point(&mut self, point: Vector) -> PolytopeId {
        self.add(Polytope::Point { point })
    }
    /// Adds a non-point polytope to the arena and registers it as a parent with
    /// each of its children.
    fn add_branch(
        &mut self,
        rank: u8,
        mut location: Option<FacetLocation>,
        children: SmallVec<[PolytopeId; 4]>,
    ) -> Result<PolytopeId> {
        ensure!(
            !children.is_empty(),
            "cannot add rank {rank} polytope with no children",
        );

        if rank == 1 {
            ensure!(
                children.len() == 2,
                "cannot add edge with {} children",
                children.len(),
            );
        }

        let ret = self.add(Polytope::Branch {
            rank,
            location,
            children: children.clone(),
        });

        for child in children {
            let child = self.get_mut(child)?;
            ensure!(
                child.rank() + 1 == rank,
                "cannot add rank {rank} polytope with rank {} child",
                child.rank(),
            );
        }
        Ok(ret)
    }
    /// Deletes a root polytope and all its children.
    fn delete_root(&mut self, id: PolytopeId) -> Result<()> {
        let mut deleted_set = EXTRA_ASSERTIONS.then(HashSet::new);

        self.roots.remove(&id);
        self.delete_polytope_recursive(id, &mut deleted_set);

        if let Some(deleted) = deleted_set {
            // Make sure that no other polytopes hold a reference to any
            // children of this one.
            for (_, p) in &self.polytopes {
                if let Ok(children) = p.children() {
                    for child in children {
                        ensure!(
                            !deleted.contains(child),
                            "reference to deleted polytope remains",
                        );
                    }
                }
            }
        }
        Ok(())
    }
    fn delete_polytope_recursive(
        &mut self,
        id: PolytopeId,
        deleted_set: &mut Option<HashSet<PolytopeId>>,
    ) {
        self.polytopes.try_remove(id.0 as usize);
        if let Some(deleted) = deleted_set {
            deleted.insert(id);
        }

        if let Ok(Ok(children)) = self.get(id).map(|p| p.children().cloned()) {
            for child in children {
                self.delete_polytope_recursive(child, deleted_set);
            }
        }
    }
    pub fn remove_internal(&mut self) -> Result<()> {
        for root in self.roots.clone() {
            if self.is_internal(root)? {
                self.delete_root(root)?;
                self.roots.remove(&root);
            }
        }
        Ok(())
    }

    pub fn is_internal(&self, id: PolytopeId) -> Result<bool> {
        Ok(match self.get(id)? {
            Polytope::Branch {
                rank,
                location,
                children,
            } => {
                if *rank == self.ndim {
                    children
                        .iter()
                        .all(|&c| self.is_internal(c).unwrap_or(false))
                } else {
                    *location == Some(FacetLocation::Internal)
                }
            }
            _ => false,
        })
    }

    pub fn radius(&self) -> f32 {
        self.polytopes
            .iter()
            .filter_map(|(_, p)| p.point().ok())
            .fold(1.0, |a, b| f32::max(a, b.mag2()))
            .sqrt()
    }

    /// Returns a list of all polygons (rank-2 polytopes) in the arena.
    pub fn polygons(&self, no_internal: bool) -> Result<Vec<(PolytopeId, Vec<Polygon>)>> {
        self.roots
            .iter()
            .map(|&p| Ok((p, self.polytope_polygons(p, no_internal)?)))
            .collect()
    }

    pub fn polytope_polygons(&self, p: PolytopeId, no_internal: bool) -> Result<Vec<Polygon>> {
        let polytope = self.get(p)?;
        let internal = self.is_internal(p)?;
        if !no_internal || !internal {
            if polytope.rank() == 2 {
                let edges: Vec<[PolytopeId; 2]> = polytope
                    .children()?
                    .iter()
                    .map(|&p| -> Result<[PolytopeId; 2]> {
                        let edge = self.get(p)?;
                        let endpoints = edge.children()?;
                        // Unpack the edge into the point on either end.
                        let [a, b] = *<&[PolytopeId; 2]>::try_from(endpoints.as_slice())
                            .context("bad child count for edge")?;
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

                let [mut prev, mut current] = edges.first().context("bad child cound for edge")?;
                let first_vertex = prev;
                verts.push(self.get(current)?.point()?.clone());
                while current != first_vertex {
                    let new = adj
                        .get(&current)
                        .unwrap()
                        .iter()
                        .copied()
                        .find(|&v| v != prev)
                        .context("bad polygon")?;
                    prev = current;
                    current = new;
                    let prev_point = self.get(prev)?.point()?;
                    let current_point = self.get(current)?.point()?;
                    if !abs_diff_eq!(prev_point, current_point) {
                        verts.push(current_point.clone());
                    }
                }

                Ok(vec![Polygon { verts }])
            } else if polytope.rank() > 2 {
                polytope
                    .children()?
                    .iter()
                    .map(|&child| self.polytope_polygons(child, no_internal))
                    .flatten_ok()
                    .collect()
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    }
    pub fn polytope_facet_ids(&self, p: PolytopeId, no_internal: bool) -> Result<Vec<PolytopeId>> {
        let polytope = self.get(p)?;
        Ok(polytope
            .children()?
            .iter()
            .copied()
            .filter(|&child| !no_internal || !self.is_internal(child).expect("Bad child"))
            .collect())
    }
    pub fn polytope_location(&self, p: PolytopeId) -> Result<Facet> {
        match self.get(p)? {
            Polytope::Branch {
                location: Some(FacetLocation::Boundary(f)),
                ..
            } => Ok(*f),
            _ => bail!("polytope {} has no location", p),
        }
    }

    /// Returns an error and logs if the arena is in an invalid state.
    ///
    /// This runs an expensive check, so avoid it if possible.
    fn validate(&self) -> Result<()> {
        let result = self._validate();
        if result.is_err() {
            log::error!(
                "Invalid polytope arena! Dumping contents as DOT:\n\n{}",
                self.graph_str()
            );
        }
        result
    }
    fn _validate(&self) -> Result<()> {
        let mut seen = HashSet::new();

        // Each root should have a disjoint set of descendents.
        for &root in &self.roots {
            let mut new_seen = HashSet::new();
            self.add_descendents_to_set(root, &mut new_seen)?;
            ensure!(
                new_seen.is_disjoint(&seen),
                "root polytope {root} shares these \
                 children with other roots: {:?}",
                new_seen.union(&seen),
            );
            seen.extend(new_seen);
        }

        // And there should be no orphaned polytopes.
        ensure!(self.polytopes.len() == seen.len(), "orphaned polytopes");

        // And every polytope should have the correct number of children, and
        // the children should be of the correct rank.
        for (i, polytope) in &self.polytopes {
            if let Ok(children) = polytope.children() {
                let rank = polytope.rank();
                if rank == 1 {
                    ensure!(children.len() == 2, "edge {i} doesn't have two children");
                } else {
                    ensure!(children.len() != 0, "polytope {i} has no children");
                }
                for &child in children {
                    let child_rank = self.get(child)?.rank();
                    ensure!(
                        child_rank + 1 == rank,
                        "polytope {i} has rank {rank} but its \
                         child with ID {child} has rank {child_rank}",
                    );
                }
            }
        }

        Ok(())
    }

    /// Produces a Graphviz-compatible DOT file representing the entire polytope
    /// arena. See https://graphviz.org/ for more.
    pub fn graph_str(&self) -> String {
        let mut s = String::new();
        s += "graph {\n";
        for (i, polytope) in &self.polytopes {
            s += &format!("  {i} ");
            match polytope {
                Polytope::Point { point } => s += &format!("[label=\"{point:.03}\"]"),
                Polytope::Branch {
                    rank: _,
                    location,
                    children,
                } => {
                    let color = match location {
                        Some(FacetLocation::Primordial) => "pink",
                        Some(FacetLocation::External) => "beige",
                        Some(FacetLocation::Boundary(_)) => "lightblue",
                        Some(FacetLocation::Internal) => "gray",
                        None => "green",
                    };
                    s += &format!("[style=filled color={}]\n", color);
                    s += &format!("  {i} -- ");
                    s += "{ ";
                    for child in children {
                        s += &format!("{} ", child);
                    }
                    s += "}";
                }
            }
            s += "\n";
        }
        s += "}\n";
        s
    }

    /// Adds all descendents of a polytope to a set.
    fn add_descendents_to_set(
        &self,
        root: PolytopeId,
        descendents: &mut HashSet<PolytopeId>,
    ) -> Result<()> {
        if descendents.insert(root) {
            if let Ok(children) = self.get(root)?.children() {
                for &child in children {
                    self.add_descendents_to_set(child, descendents)?;
                }
            }
        }
        Ok(())
    }

    /// Slices the polytope by a hyperplane, removing external parts.
    pub fn carve(&mut self, plane: &Hyperplane, facet: Facet) -> Result<()> {
        self.slice_all(plane, SliceMode::Carve(facet))
    }
    /// Slices the polytope by a hyperplane.
    pub fn slice_internal(&mut self, plane: &Hyperplane) -> Result<()> {
        self.slice_all(plane, SliceMode::Internal)
    }

    /// Slices every polytope by a hyperplane, removing external parts if
    /// carving.
    fn slice_all(&mut self, plane: &Hyperplane, mode: SliceMode) -> Result<()> {
        let mut op = SliceOperation {
            plane,
            mode,
            results: HashMap::new(),
        };

        // Update roots.
        for root in std::mem::take(&mut self.roots) {
            let (above, below) = match self.slice_polytope(root, &mut op)? {
                SliceResult::Above => (Some(root), None),
                SliceResult::Below => (None, Some(root)),
                SliceResult::Split { above, below } => {
                    self.roots.remove(&root);
                    (above, below)
                }
            };

            if let Some(above) = above {
                if mode.loc_above() != FacetLocation::External {
                    self.roots.insert(above);
                }
            }
            if let Some(below) = below {
                if mode.loc_below() != FacetLocation::External {
                    self.roots.insert(below);
                }
            }
        }

        // Delete dead polytopes.
        for (polytope, result) in op.results {
            let (above, below) = match result {
                SliceResult::Above => (Some(polytope), None),
                SliceResult::Below => (None, Some(polytope)),
                SliceResult::Split { above, below } => {
                    self.remove(polytope).context("removing split polytope")?;
                    if mode.loc_above() == FacetLocation::External {
                        if let Some(above) = above {
                            if let Ok(children) = self.get(above)?.children() {
                                if let Some(&intersection) = children.last() {
                                    self.remove(intersection).context(
                                        "removing intersection child of upper polytope of split",
                                    )?;
                                }
                            }
                        }
                    }
                    if mode.loc_below() == FacetLocation::External {
                        if let Some(below) = below {
                            if let Ok(children) = self.get(below)?.children() {
                                if let Some(&intersection) = children.last() {
                                    self.remove(intersection).context(
                                        "removing intersection child of lower polytope of split",
                                    )?;
                                }
                            }
                        }
                    }
                    (above, below)
                }
            };
            if let Some(above) = above {
                if mode.loc_above() == FacetLocation::External {
                    self.remove(above)
                        .context("removing upper polytope of split")?;
                }
            }
            if let Some(below) = below {
                if mode.loc_below() == FacetLocation::External {
                    self.remove(below)
                        .context("removing lower polytope of split")?;
                }
            }
        }

        if EXTRA_ASSERTIONS {
            self.validate()?;
        }

        Ok(())
    }

    fn slice_polytope(&mut self, p: PolytopeId, op: &mut SliceOperation) -> Result<SliceResult> {
        // If `p` has already been sliced, then just return that result.
        if let Some(&result) = op.results.get(&p) {
            return Ok(result);
        }

        let result = match self.get(p).context("cannot slice missing polytope")? {
            Polytope::Point { point } => {
                let distance = op.plane.distance_to(point);
                if distance < -(SPLIT_MARGIN - EPSILON) {
                    SliceResult::Below
                } else if distance > SPLIT_MARGIN - EPSILON {
                    SliceResult::Above
                } else {
                    SliceResult::Split {
                        above: None,
                        below: None,
                    }
                }
            }
            Polytope::Branch {
                rank,
                location,
                children,
            } => {
                let rank = *rank;
                let location = *location;
                let old_children = children.clone();
                let mut children_above: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut children_below: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut intersection_children_above = smallvec![];
                let mut intersection_children_below = smallvec![];

                // Tracks if any child has been split
                let mut split_flag = false;
                for &child in &old_children {
                    match self.slice_polytope(child, op)? {
                        SliceResult::Above => children_above.push(child),
                        SliceResult::Below => children_below.push(child),
                        SliceResult::Split { above, below } => {
                            split_flag = true;
                            if let Some(above) = above {
                                children_above.push(above);
                                intersection_children_above.push(
                                    *self
                                        .get(above)?
                                        .children()?
                                        .last()
                                        .context("no intersection child")?,
                                );
                            }
                            if let Some(below) = below {
                                children_below.push(below);
                                intersection_children_below.push(
                                    *self
                                        .get(below)?
                                        .children()?
                                        .last()
                                        .context("no intersection child")?,
                                );
                            }
                        }
                    }
                }

                match (children_above.as_slice(), children_below.as_slice()) {
                    // Children got deleted.
                    ([], []) => SliceResult::Split {
                        above: None,
                        below: None,
                    },
                    // All children are above.
                    (_, []) if !split_flag => SliceResult::Above,
                    // All children are below.
                    ([], _) if !split_flag => SliceResult::Below,
                    // Children are on opposite sides.
                    _ => {
                        let mut intersection_above = None;
                        let mut intersection_below = None;

                        if rank == 1 {
                            let mut a = self.get(old_children[0])?.point()?.clone();
                            let mut b = self.get(old_children[1])?.point()?.clone();
                            let mut ah = op.plane.distance_to(&a);
                            let mut bh = op.plane.distance_to(&b);

                            // Ensure that `a` is above the plane and `b` is
                            // below the plane.
                            if ah < bh {
                                std::mem::swap(&mut a, &mut b);
                                std::mem::swap(&mut ah, &mut bh);
                            }
                            // Now `ah` is positive and `bh` is negative, so
                            // this subtraction actually gives a sum of the
                            // absolute values.
                            let sum = ah - bh;

                            // Split this edge into two edges: one above the
                            // plane and one below the plane.

                            if !children_above.is_empty() {
                                let t = (ah - SPLIT_MARGIN) / sum;
                                intersection_above = Some(self.add_point(util::mix(&a, &b, t)));
                            }

                            if !children_below.is_empty() {
                                let t = (ah + SPLIT_MARGIN) / sum;
                                intersection_below = Some(self.add_point(util::mix(&a, &b, t)));
                            }
                        } else {
                            // Split this polytope into two polytopes: one above
                            // the plane and one below the plane.

                            if intersection_children_above.len() >= 2 {
                                intersection_above = Some(self.add_branch(
                                    rank - 1,
                                    (rank == self.ndim).then_some(op.mode.loc_above()),
                                    intersection_children_above,
                                )?);
                            }

                            if intersection_children_below.len() >= 2 {
                                intersection_below = Some(self.add_branch(
                                    rank - 1,
                                    (rank == self.ndim).then_some(op.mode.loc_below()),
                                    intersection_children_below,
                                )?);
                            }
                        }
                        let above = intersection_above
                            .map(|intersection| {
                                children_above.push(intersection);
                                self.add_branch(rank, location, children_above)
                            })
                            .transpose()?;
                        let below = intersection_below
                            .map(|intersection| {
                                children_below.push(intersection);
                                self.add_branch(rank, location, children_below)
                            })
                            .transpose()?;
                        SliceResult::Split { above, below }
                    }
                }
            }
        };

        op.results.insert(p, result);
        Ok(result)
    }

    pub fn axis_spans(&self, axis: &Vector) -> Result<Vec<(PolytopeId, Span)>> {
        self.roots
            .iter()
            .map(|&p| Ok((p, self.polytope_axis_span(p, axis)?)))
            .collect()
    }
    fn polytope_axis_span(&self, p: PolytopeId, axis: &Vector) -> Result<Span> {
        match self.get(p)? {
            Polytope::Point { point } => {
                let distance = point.dot(axis);
                Ok(Span {
                    above: distance,
                    below: distance,
                })
            }
            Polytope::Branch { children, .. } => children
                .iter()
                .map(|child| self.polytope_axis_span(*child, axis))
                .reduce(|a, b| Ok(a?.union(b?)))
                .context("bad chilid count")?,
        }
    }

    pub fn transform_polytope(&mut self, root: PolytopeId, m: &Matrix) -> Result<()> {
        self.transform_recurse(root, &mut HashSet::new(), &mut |arena, id| {
            if let Polytope::Point { point, .. } = arena.get_mut(id)? {
                *point = m * &*point;
            }
            Ok(())
        })
    }

    fn transform_recurse(
        &mut self,
        p: PolytopeId,
        seen: &mut HashSet<PolytopeId>,
        closure: &mut impl FnMut(&mut PolytopeArena, PolytopeId) -> Result<()>,
    ) -> Result<()> {
        closure(self, p)?;
        seen.insert(p);
        if let Ok(children) = self.get(p)?.children() {
            for child in children.clone() {
                if !seen.contains(&child) {
                    self.transform_recurse(child, seen, closure)?;
                }
            }
        }
        Ok(())
    }
}

/// Index of a polytope in a polytope arena.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolytopeId(pub u32);
impl std::fmt::Display for PolytopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Node in the polytope DAG, either a point (leaf) or a branch.
///
/// `slice_result` is included here instead of in `Polytope` because it makes
/// the struct smaller, somehow. (Something to do with enum tag optimizations,
/// probably?)
#[derive(Debug, Clone, PartialEq)]
enum Polytope {
    Point {
        point: Vector,
    },
    Branch {
        rank: u8,
        location: Option<FacetLocation>, // Only for facets, None otherwise
        children: SmallVec<[PolytopeId; 4]>,
    },
}
impl Polytope {
    /// Returns the rank (number of dimensions) of the polytope. A point has
    /// rank 0, a line has rank 1, etc.
    fn rank(&self) -> u8 {
        match self {
            Self::Point { .. } => 0,
            Self::Branch { rank, .. } => *rank,
        }
    }
    /// Returns the coordinate point if this polytope is a point, or an error if
    /// is a branch.
    fn point(&self) -> Result<&Vector> {
        match &self {
            Self::Point { point, .. } => Ok(point),
            _ => Err(anyhow!("expected point, got rank {} polytope", self.rank())),
        }
    }
    /// Returns the children of the polytope if it is a branch, or an error if
    /// it is a point.
    fn children(&self) -> Result<&SmallVec<[PolytopeId; 4]>> {
        match &self {
            Self::Point { .. } => bail!("can't get children of point"),
            Self::Branch { children, .. } => Ok(children),
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
        /// Portion of the polytope that is above the slice. Its last child
        /// intersects the slicing plane.
        above: Option<PolytopeId>,
        /// Portion of the polytope that is below the slice. Its last child
        /// intersects the slicing plane.
        below: Option<PolytopeId>,
    },
}

/// Location of a facet, which may be external, internal, or on the boundary
/// of the polytope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FacetLocation {
    /// This facet is from the original hypercube. If any of these facets
    /// remain after slicing, then the original cube was not big enough.
    Primordial,
    /// This facet is external to the puzzle. It will be deleted.
    External,
    /// This facet is a sticker facet.
    Boundary(Facet),
    /// This facet is internal to the puzzle.
    Internal,
}

/// How to slice the polytope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SliceMode {
    /// Delete any polytopes above the slicing plane and mark the ones created
    /// by the slice as being part of this facet.
    Carve(Facet),
    /// Mark polytopes created by the slice as internal.
    Internal,
}
impl SliceMode {
    fn loc_above(self) -> FacetLocation {
        match self {
            SliceMode::Carve(_) => FacetLocation::External,
            SliceMode::Internal => FacetLocation::Internal,
        }
    }
    fn loc_below(self) -> FacetLocation {
        match self {
            SliceMode::Carve(f) => FacetLocation::Boundary(f),
            SliceMode::Internal => FacetLocation::Internal,
        }
    }
}

/// In-progress slice operation
#[derive(Debug)]
struct SliceOperation<'a> {
    plane: &'a Hyperplane,
    mode: SliceMode,
    results: HashMap<PolytopeId, SliceResult>,
}

pub struct Span {
    pub above: f32,
    pub below: f32,
}
impl Span {
    pub fn union(&self, other: Span) -> Span {
        Span {
            above: f32::max(self.above, other.above),
            below: f32::min(self.below, other.below),
        }
    }
}

#[derive(Debug)]
pub struct NullPolytope;
impl std::fmt::Display for NullPolytope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "null polytope")
    }
}
impl std::error::Error for NullPolytope {}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub verts: Vec<Vector>,
}

fn base_3_expansion(n: u32, digit_count: u8) -> impl Iterator<Item = u32> {
    std::iter::successors(Some(n), |x| Some(x / 3))
        .take(digit_count as _)
        .map(|x| x % 3)
}

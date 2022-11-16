use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;
use slab::Slab;
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::iter::Sum;
use tinyset::Set64;

use crate::math::*;
use crate::puzzle::Facet;

const EXTRA_ASSERTIONS: bool = cfg!(debug_assertions);

macro_rules! ensure {
    ($($tok:tt)*) => {
        if EXTRA_ASSERTIONS {
            anyhow::ensure!($($tok)*);
        }
    };
}

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

    /// Returns the rank of a polytope.
    pub fn get_rank(&self, id: PolytopeId) -> Result<u8> {
        Ok(self.get(id)?.rank())
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
        location: Option<FacetLocation>,
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
        } else if rank > 2 {
            ensure!(
                children.len() > rank as usize,
                "cannot add rank {rank} polytope with {} children",
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

    pub fn polytope_vertices(&self, p: PolytopeId) -> Result<Vec<Vector>> {
        match self.get(p)? {
            Polytope::Point { point } => Ok(vec![point.clone()]),
            Polytope::Branch { children, .. } => children
                .iter()
                .map(|&child| self.polytope_vertices(child))
                .flatten_ok()
                .collect(),
        }
    }

    /// Returns indexed polygons for a single polytope.
    pub fn polytope_indexed_polygons(
        &self,
        p: PolytopeId,
        no_internal: bool,
    ) -> Result<IndexedPolygons> {
        let mut vertex_map = HashMap::new();
        let mut verts = vec![];
        let polys = self
            .polytope_polygons(p, no_internal)?
            .into_iter()
            .map(|poly| {
                poly.into_iter()
                    .map(|point_id| match vertex_map.get(&point_id) {
                        Some(i) => Ok(*i),
                        None => {
                            vertex_map.insert(point_id, verts.len() as u16);
                            verts.push(self.get(point_id)?.point()?.clone());
                            Ok(*vertex_map.get(&point_id).unwrap())
                        }
                    })
                    .collect::<Result<_>>()
                    .map(IndexedPolygon)
            })
            .try_collect()?;
        Ok(IndexedPolygons { verts, polys })
    }

    // TODO: rethink API
    pub fn get_point(&self, p: PolytopeId) -> Result<&Vector> {
        self.get(p)?.point()
    }

    // pub fn most_common_points(&self, ps: &[PolytopeId]) -> Vec<PolytopeId> {
    //     let mut candidates = HashMap::new();
    //     let mut rank = 0;
    //     let mut most_in_common = 0;
    //     for &p in ps {
    //         let mut descendents = HashSet::new();
    //         self.add_descendents_to_set(p, &mut descendents);
    //         for d in descendents {
    //             candidates.entry(d).or_insert(0)
    //         }
    //     }
    //     descendents.into_iter().filter(predicate)
    // }

    pub fn polytope_polygons(
        &self,
        p: PolytopeId,
        no_internal: bool,
    ) -> Result<Vec<Vec<PolytopeId>>> {
        if no_internal && self.is_internal(p)? {
            return Ok(vec![]);
        }

        let polytope = self.get(p)?;
        match polytope.rank() {
            0..=1 => Ok(vec![]),
            2 => {
                let edges: Vec<[PolytopeId; 2]> = polytope
                    .children()?
                    .iter()
                    .map(|&p| self.get(p)?.edge_endpoints())
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
                verts.push(current);
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
                    verts.push(current);
                }

                Ok(vec![verts])
            }
            3.. => polytope
                .children()?
                .iter()
                .map(|&child| self.polytope_polygons(child, no_internal))
                .flatten_ok()
                .collect(),
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

        // Collect all descendents.
        for &root in &self.roots {
            self.add_descendents_to_set(root, &mut seen)?;
        }

        // There should be no orphaned polytopes.
        ensure!(self.polytopes.len() == seen.len(), "orphaned polytopes");

        // Every polytope should have the correct number of children, and the
        // children should be of the correct rank.
        for (i, polytope) in &self.polytopes {
            if let Ok(children) = polytope.children() {
                let rank = polytope.rank();
                for &child in children {
                    let child_rank = self.get(child)?.rank();
                    ensure!(
                        child_rank + 1 == rank,
                        "polytope {i} has rank {rank} but its \
                         child with ID {child} has rank {child_rank}",
                    );
                }
                if rank == 1 {
                    ensure!(children.len() == 2, "edge {i} doesn't have two children");
                } else {
                    ensure!(
                        children.len() > rank as usize,
                        "polytope {i} has rank {rank} but only {} children",
                        children.len(),
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
    pub fn add_descendents_to_set(
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
        log::trace!("Carving along plane {plane:?} for {facet:?}");
        self.slice_all(plane, SliceMode::Carve(facet))
    }
    /// Slices the polytope by a hyperplane.
    pub fn slice_internal(&mut self, plane: &Hyperplane) -> Result<()> {
        log::trace!("Slicing along plane {plane:?}");
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
            match self.slice_polytope(root, &mut op)? {
                SliceResult::Above { .. } if op.mode.delete_above() => (),
                SliceResult::Above { .. } => {
                    self.roots.insert(root);
                }
                SliceResult::Below { .. } => {
                    self.roots.insert(root);
                }
                SliceResult::Flush => bail!("root polytope cannot be flush"),
                SliceResult::Split { above, below, .. } => {
                    if !op.mode.delete_above() {
                        self.roots.insert(above);
                    }
                    self.roots.insert(below);
                }
            };
        }

        // Delete dead polytopes.
        for (polytope, result) in op.results {
            match result {
                SliceResult::Above { .. } => {
                    if op.mode.delete_above() {
                        self.remove(polytope)
                            .context("removing upper polytope of split")?;
                    }
                }
                SliceResult::Below { .. } => (),
                SliceResult::Flush => (),
                SliceResult::Split { above, .. } => {
                    self.remove(polytope)?;
                    if op.mode.delete_above() {
                        self.remove(above)
                            .context("removing upper polytope of split")?;
                    }
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
                match abs_diff_cmp(&distance, &0.0) {
                    std::cmp::Ordering::Less => SliceResult::Below { intersection: None },
                    std::cmp::Ordering::Equal => SliceResult::Flush,
                    std::cmp::Ordering::Greater => SliceResult::Above { intersection: None },
                }
            }
            Polytope::Branch {
                rank,
                location,
                children,
                ..
            } => {
                let rank = *rank;
                let location = *location;
                let old_children = children.clone();
                let mut children_above: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut children_below: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut flush_grandchildren: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut flush_child = None;

                for &child in &old_children {
                    match self.slice_polytope(child, op)? {
                        SliceResult::Above { intersection } => {
                            children_above.push(child);
                            flush_grandchildren.extend(intersection);
                        }
                        SliceResult::Below { intersection } => {
                            children_below.push(child);
                            flush_grandchildren.extend(intersection);
                        }
                        // TODO potential optimization: early return `Above` or
                        // `Below` after seeing a `Flush` child.
                        SliceResult::Flush => flush_child = Some(child),
                        SliceResult::Split {
                            above,
                            below,
                            intersection,
                        } => {
                            children_above.push(above);
                            children_below.push(below);
                            flush_grandchildren.push(intersection);
                        }
                    }
                }

                // Honestly there's so few grandchildren that this is probably
                // the most efficient solution.
                flush_grandchildren.sort();
                flush_grandchildren.dedup();

                if flush_child.is_none() && flush_grandchildren.len() >= 2 {
                    flush_child = Some(self.add_branch(
                        rank - 1,
                        (rank == self.ndim).then_some(op.mode.slice_location()),
                        flush_grandchildren,
                    )?);
                }

                match (children_above.as_slice(), children_below.as_slice()) {
                    // All children are flush.
                    ([], []) => SliceResult::Flush,
                    // All children are above or flush.
                    (_, []) => SliceResult::Above {
                        intersection: flush_child,
                    },
                    // All children are below or flush.
                    ([], _) => SliceResult::Below {
                        intersection: flush_child,
                    },
                    // The polytope is split.
                    (_, _) => {
                        let intersection = if rank == 1 {
                            ensure!(children_above.len() == 1);
                            ensure!(children_below.len() == 1);
                            let a = self.get(children_above[0])?.point()?.clone();
                            let b = self.get(children_below[0])?.point()?.clone();
                            let ah = op.plane.distance_to(&a);
                            let bh = op.plane.distance_to(&b);

                            // Ensure that `a` is above the plane and `b` is
                            // below the plane.
                            ensure!(ah > 0.0);
                            ensure!(bh < 0.0);

                            // `ah` is positive and `bh` is negative, so this
                            // subtraction actually gives a sum of the absolute
                            // values.
                            let sum = ah - bh;

                            // Split this edge into two edges: one above the
                            // plane and one below the plane.
                            let t = ah / sum;

                            self.add_point(util::mix(&a, &b, t))
                        } else {
                            flush_child.context("split polytope has no flush child")?
                        };

                        // Split this polytope into two polytopes: one above the
                        // plane and one below the plane.
                        children_above.push(intersection);
                        children_below.push(intersection);
                        let above = self.add_branch(rank, location, children_above)?;
                        let below = self.add_branch(rank, location, children_below)?;
                        SliceResult::Split {
                            above,
                            below,
                            intersection,
                        }
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

    /// Returns the centroid for each root polytope.
    pub fn compute_centroids(&self) -> Result<BTreeMap<PolytopeId, Vector>> {
        let mut cache = BTreeMap::new();
        let _: Result<Vec<_>> = self
            .roots
            .iter()
            .map(|&id| Ok((id, self.compute_mass(&mut cache, id)?.com)))
            .collect();
        Ok(cache.into_iter().map(|(p, mass)| (p, mass.com)).collect())
    }
    /// Returns the mass of a non-point polytope.
    fn compute_mass(&self, cache: &mut BTreeMap<PolytopeId, Mass>, p: PolytopeId) -> Result<Mass> {
        // In this function, the terms "mass" and "volume" are used pretty much
        // interchangeably to refer to hypervolume, AKA Lebasgue measure.

        if let Some(result) = cache.get(&p) {
            return Ok(result.clone());
        }
        let result = match self.get(p)? {
            Polytope::Point { point } => Mass {
                mass: Multivector::scalar(1.0),
                com: point.clone(),
            },

            edge @ Polytope::Branch { rank: 1, .. } => {
                let [a, b] = edge.edge_endpoints()?;
                let a = self.get(a)?.point()?;
                let b = self.get(b)?.point()?;

                Mass {
                    mass: (a - b).into(),
                    com: (a + b) * 0.5,
                }
            }

            Polytope::Branch { rank, children, .. } => {
                // Compute the centroid of each child.
                let child_volumes: Vec<Mass> = children
                    .iter()
                    .map(|&child| self.compute_mass(cache, child))
                    .try_collect()?;

                // Average those centroids to get an arbitrary point inside the
                // polytope. This will be the apex of a pyramid for each child.
                let apex = child_volumes.iter().map(|mass| &mass.com).sum::<Vector>()
                    / child_volumes.len() as f32;

                // For each child, construct a pyramid with that child as the
                // base.
                child_volumes
                    .iter()
                    .map(|v| {
                        // This vector adds a new dimension to the child polytope.
                        let new_vector = &apex - &v.com;

                        let parallelotope_mass =
                            (Multivector::from(new_vector) * &v.mass).grade_project(*rank);

                        // The volume of a pyramid is `1/NDIM` times the volume
                        // of a parallelotope.
                        let mass = parallelotope_mass * (*rank as f32).recip();

                        // In 2D, the center of mass of a triangle is 1/3 the
                        // way from the base to the apex. In 3D, it's 1/4 the
                        // way up. In N dimsensions, it's 1/(NDIM+1).
                        let com = util::mix(&v.com, &apex, (*rank as f32 + 1.0).recip());

                        Mass { mass, com }
                    })
                    .sum::<Result<Mass>>()?
            }
        };
        cache.insert(p, result.clone());
        Ok(result)
    }

    /// Returns, for each polytope, the set of facets it is a part of. If a
    /// point is not in the returned map, it is not a member of any facet.
    pub fn adj_facets(&self) -> Result<BTreeMap<PolytopeId, Set64<Facet>>> {
        // TODO: consider tracking the facet set as the polytope is constructed,
        // especially since Set64 is so cheap!
        let mut results: BTreeMap<PolytopeId, Set64<Facet>> = BTreeMap::new();
        for &root in &self.roots {
            for &facet_polytope in self.get(root)?.children()? {
                if let &Polytope::Branch {
                    location: Some(FacetLocation::Boundary(facet)),
                    ..
                } = self.get(facet_polytope)?
                {
                    self.visit_recursively(facet_polytope, |current| {
                        Ok(results.entry(current).or_default().insert(facet))
                    })?;
                };
            }
        }
        Ok(results)
    }

    fn visit_recursively(
        &self,
        start: PolytopeId,
        mut visit: impl FnMut(PolytopeId) -> Result<bool>,
    ) -> Result<()> {
        let mut stack = vec![start];
        while let Some(next_id) = stack.pop() {
            if visit(next_id)? {
                if let Ok(children) = self.get(next_id)?.children() {
                    stack.extend_from_slice(children);
                }
            }
        }
        Ok(())
    }

    // /// Clamps a point to within the bounds of a polytope.
    // pub fn clamp_point(&self, point: Vector, p: PolytopeId) -> Result<Vector> {
    //     // Get a list of all the points in the polytope.
    //     let mut descendents = HashSet::new();
    //     self.add_descendents_to_set(p, &mut descendents)?;
    //     // Compute *some* point in the center of the polytope.
    //     let points = descendents.iter().filter_map(|id|self.get(id).ok()?.point().ok()?);
    //     let center = points =
    //     let points = descendents
    //         .into_iter()
    //         .filter_map(|id| Some(self.get(id).ok()?.point().ok()?))
    //         .collect_vec();

    //     todo!()
    // }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mass {
    /// Lebasgue measure (https://en.wikipedia.org/wiki/Lebesgue_measure) as a
    /// blade.
    mass: Multivector,
    /// Center of mass.
    com: Vector,
}
impl Sum<Mass> for Result<Mass> {
    fn sum<I: Iterator<Item = Mass>>(iter: I) -> Self {
        // This function assumes that all the masses are in the same subspaace.

        let mut iter = iter.peekable();
        let first = iter
            .peek()
            .context("empty polytope has no center of mass")?;

        // Some of these masses may have opposite signs. We want all masses to
        // be positive, so pick some component to normalize the signs with
        // respect to.
        let component = first.mass.most_significant_component();
        let unit_mass = &first.mass
            * first
                .mass
                .get(component)
                .context("child of polytope has zero mass")?
                .recip();

        let mut total_com = Vector::EMPTY;
        let mut total_weight = 0.0;

        for it in iter {
            let weight = it.mass.get(component).unwrap_or(0.0).abs();
            total_com += it.com * weight;
            total_weight += weight;
        }

        Ok(Mass {
            mass: unit_mass * total_weight,
            com: total_com / total_weight,
        })
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
        match self {
            Self::Point { .. } => bail!("can't get children of point"),
            Self::Branch { children, .. } => Ok(children),
        }
    }
    /// Returns the endpoints if this polytope is an edge, or an error if it is
    /// not.
    fn edge_endpoints(&self) -> Result<[PolytopeId; 2]> {
        match self {
            Polytope::Branch {
                rank: 1, children, ..
            } => children
                .as_slice()
                .try_into()
                .context("bad child count for edge"),
            _ => Err(anyhow!("expected edge, got rank {} polytope", self.rank())),
        }
    }
}

/// Result of slicing a polytope with a hyperplane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SliceResult {
    /// The whole polytope is above the slice.
    Above {
        /// Child of the polytope that is the intersection of this one with the
        /// slicing plane, if any.
        intersection: Option<PolytopeId>,
    },
    /// The whole polytope is below the slice.
    Below {
        /// Child of the polytope that is the intersection of this one with the
        /// slicing plane, if any.
        intersection: Option<PolytopeId>,
    },
    /// The polytope is contained within the slicing plane.
    Flush,
    /// The polytope is cut by the slice.
    Split {
        /// Portion of the polytope that is above the slice.
        above: PolytopeId,
        /// Portion of the polytope that is below the slice.
        below: PolytopeId,

        /// Child of the polytope that is the intersection of this one with the
        /// slicing plane.
        intersection: PolytopeId,
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
    fn delete_above(self) -> bool {
        match self {
            SliceMode::Carve(_) => true,
            SliceMode::Internal => false,
        }
    }
    fn slice_location(self) -> FacetLocation {
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IndexedPolygon(pub Vec<u16>);

#[derive(Debug, Default, Clone, PartialEq)]
pub struct IndexedPolygons {
    pub verts: Vec<Vector>,
    pub polys: Vec<IndexedPolygon>,
}

fn base_3_expansion(n: u32, digit_count: u8) -> impl Iterator<Item = u32> {
    std::iter::successors(Some(n), |x| Some(x / 3))
        .take(digit_count as _)
        .map(|x| x % 3)
}

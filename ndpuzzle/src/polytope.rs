use ahash::{AHashMap, AHashSet};
use anyhow::{bail, Context, Result};
use itertools::Itertools;
use slab::Slab;
use smallvec::SmallVec;
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::Sum;
use tinyset::Set64;

use crate::math::*;
use crate::puzzle::Facet;
use crate::util::{
    set64_intersection, IterWithExactSizeExt, Set64IntersectionIterExt, Set64TryIntersectionIterExt,
};

const EXTRA_ASSERTIONS: bool = cfg!(debug_assertions);

/// Set of puzzle facets.
pub type FacetSet = Set64<Facet>;
/// List of puzzle facets (typically sorted and deduplicated).
pub type FacetList = SmallVec<[Facet; 8]>;

macro_rules! ensure {
    ($($tok:tt)*) => {
        if EXTRA_ASSERTIONS {
            debug_assert!($($tok)*);
            anyhow::ensure!($($tok)*);
        }
    };
}

/// Arena of polytopes which can be sliced.
///
/// Each piece is represented as a DAG (directed acyclic graph). The DAG is
/// arranged into layers corresponding to polytope ranks. For example, a cube is
/// represented as follows:
///
/// - There are 8 rank-0 polytopes (points). Each has no children.
/// - There are 12 rank-1 polytopes (edges). Each has two points as children.
/// - There are 6 rank-2 polytopes (faces). Each has four edges as children.
/// - There is one rank-3 polytope (the whole cube). It has all six faces as
///   children and is the root of the graph.
///
/// Each point contains its coordinates in N-dimensional space, but no other
/// nodes in the graph contain any geometric information. They are all purely
/// topological.
#[derive(Clone)]
pub struct PolytopeArena {
    /// Number of dimensions.
    ndim: u8,
    /// Radius of initial hypercube.
    initial_radius: f32,

    /// Canonical ordering of root polytopes, so that piece IDs are
    /// deterministic. All of these have rank `NDIM`.f
    roots: Vec<PolytopeId>,
    /// Unordered set of non-point polytopes at each rank from edges (rank 1) to
    /// roots (rank `NDIM`).
    non_points: Vec<Slab<NonPointData>>,
    /// Unordered set of points (rank 0).
    points: Slab<PointData>,

    /// Cache of polytope info, invalidated whenever a polytope is removed.
    cache: RefCell<PolytopeArenaCache>,
}
impl fmt::Debug for PolytopeArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ret = f.debug_struct("PolytopeArena");
        ret.field("ndim", &self.ndim);
        ret.field("roots", &self.roots);
        for (i, polytopes) in self.non_points.iter().enumerate() {
            ret.field(&format!("rank_{}", i + 1), &polytopes.iter().collect_vec());
        }
        ret.field("points", &self.points.iter().collect_vec());
        ret.finish()
    }
}
impl PolytopeArena {
    /// Constructs a polytope arena containing a hypercube. This hypercube can
    /// then be carved into any convex polytope of the same number of
    /// dimensions.
    pub fn new_cube(ndim: u8, radius: f32) -> Result<Self> {
        // Based on Andrey Astrelin's implementation of `GenCube()` in Magic
        // Puzzle Ultimate (FaceCuts.cs).

        ensure!(radius > 0.0, "cannot build polytope with negative radius");

        // There will be only one root, with ID 0.
        let initial_root = PolytopeId {
            rank: ndim,
            idx: PolytopeIdx(0),
        };
        let mut this = Self {
            ndim,
            initial_radius: radius,

            roots: vec![initial_root],
            non_points: (1..=ndim).map(|_| Slab::new()).collect(),
            points: Slab::new(),

            cache: RefCell::new(PolytopeArenaCache::default()),
        };

        // Make a 3^NDIM grid of polytopes to construct a hypercube. The corners
        // are vertices. Between those are edges, etc.
        //
        // ```
        // • - •
        // | # |
        // • - •
        // ```
        //
        // We'll store the IDs of all the polytopes in a flat array.
        let mut ids = Vec::with_capacity(3_usize.pow(ndim as _));

        // All of these polytopes are primordial; they are not a part of any
        // actual facets.
        let facet_set = FacetSet::new();

        // Create the polytopes.
        for i in 0..3_usize.pow(ndim as _) {
            // Each base-3 digit corresponds to an axis:
            // - `digit == 0`  ->  negative side
            // - `digit == 1`  ->  positive side
            // - `digit == 2`  ->  straddling
            //
            // We use this numbering scheme because it's important that the
            // child polytopes are generated first, so that we know what IDs to
            // use for their parents.
            let rank = base_3_expansion(i, ndim)
                .filter(|&digit| digit == 2)
                .count() as u8;

            ids.push(if rank == 0 {
                // This is a point.
                let point = base_3_expansion(i, ndim)
                    .map(|digit| (digit as f32 * 2.0 - 1.0) * radius)
                    .collect();
                this.add_point(facet_set.clone(), point)
            } else {
                // This is a non-point.
                let children = powers_of_3()
                    .zip(base_3_expansion(i, ndim))
                    // For each axis we are straddling ...
                    .filter(|&(_, digit)| digit == 2)
                    // ... add two children along that axis.
                    .flat_map(|(power_of_3, _)| {
                        let negative_child = ids[i - power_of_3 * 2];
                        let positive_child = ids[i - power_of_3];
                        [negative_child, positive_child]
                    });
                this.add_non_point_polytope(facet_set.clone(), children)?
            });
        }

        // The center of the 3^NDIM cube is the root, and it is the only
        // polytope of rank `NDIM`.

        Ok(this)
    }

    /// Returns the index used to get polytopes of a given rank in `non_points`.
    fn rank_index(&self, rank: u8) -> Result<usize> {
        (rank as usize)
            .checked_sub(1)
            .filter(|&i| i < self.non_points.len())
            .context("bad polytope rank")
    }
    /// Adds a point polytope to the arena.
    fn add_point(&mut self, facet_set: FacetSet, point: Vector) -> PolytopeId {
        let idx = PolytopeIdx(self.points.insert(PointData { facet_set, point }));
        PolytopeId { rank: 0, idx }
    }
    /// Constructs a non-point polytope from a set of children and adds it to
    /// the arena.
    fn add_non_point_polytope(
        &mut self,
        facet_set: FacetSet,
        children: impl IntoIterator<Item = PolytopeId>,
    ) -> Result<PolytopeId> {
        let mut children_iter = children.into_iter();
        let mut child_ids = Set64::new();

        let first = children_iter
            .next()
            .context("cannot add polytope with no children")?;
        child_ids.insert(first.idx);
        // Infer the rank of this polytope from the rank of the first child.
        let child_rank = first.rank;
        let rank = child_rank + 1;

        for child in children_iter {
            child_ids.insert(child.idx);
            // Check the rank of each child.
            ensure!(
                child.rank == child_rank,
                "cannot add polytope with children of different ranks",
            );
        }

        // Check the number of children.
        let child_count = child_ids.len();
        if rank == 1 {
            ensure!(
                child_count == 2,
                "edge has {child_count} children (needs exactly 2)",
            );
        } else {
            ensure!(
                child_count >= rank as usize + 1,
                "rank {rank} polytope has too few children (needs at least {})",
                rank + 1,
            );
        }

        let i = self.rank_index(rank)?;
        let idx = PolytopeIdx(self.non_points[i].insert(NonPointData {
            facet_set,
            child_ids,
        }));
        Ok(PolytopeId {
            rank: child_rank + 1,
            idx,
        })
    }

    /// Removes a single polytope. Does not recurse.
    fn remove_nonrecursive(&mut self, id: PolytopeId) -> Result<()> {
        if id.rank == 0 {
            anyhow::ensure!(self.points.contains(id.idx.0), "polytope does not exist");
            self.points.remove(id.idx.0);
        } else {
            let i = self.rank_index(id.rank)?;
            let list = &mut self.non_points[i];
            anyhow::ensure!(list.contains(id.idx.0), "polytope does not exist");
            list.remove(id.idx.0);
        }

        self.cache.borrow_mut().invalidate();

        Ok(())
    }

    /// Returns the maximum Euclidean distance of any point from the origin.
    pub fn radius(&self) -> f32 {
        let mut cache = self.cache.borrow_mut();

        *cache.radius.get_or_insert_with(|| {
            self.points
                .iter()
                .map(|(_, data)| data.point.mag2())
                .max_by(f32::total_cmp)
                .unwrap_or(0.0)
                .sqrt()
        })
    }

    /// Returns an iterator over all the root polytopes in canoncial order.
    pub fn roots(&self) -> impl '_ + Iterator<Item = PolytopeRef<'_>> + ExactSizeIterator {
        self.roots.iter().map(|&id| PolytopeRef { arena: self, id })
    }
    /// Returns an iterator over each rank of polytopes, excluding points.
    fn non_point_ranks(
        &self,
    ) -> impl '_ + Iterator<Item = (u8, impl '_ + Iterator<Item = PolytopeIdx>)> {
        self.non_points
            .iter()
            .enumerate()
            .map(|(rank_index, list)| {
                (
                    rank_index as u8 + 1,
                    list.iter().map(|(i, _)| PolytopeIdx(i)),
                )
            })
    }
    /// Returns an iterator over all the non-point polytopes.
    fn non_points(&self) -> impl '_ + Iterator<Item = PolytopeRef<'_>> {
        self.non_point_ranks().flat_map(move |(rank, idxs)| {
            idxs.map(move |idx| {
                let id = PolytopeId { rank, idx };
                PolytopeRef { arena: self, id }
            })
        })
    }

    /// Returns the coordinates of a point polytope.
    pub fn polytope_point(&self, id: PolytopeId) -> Result<&Vector> {
        ensure!(id.rank == 0, "cannot get point of non-point");

        let polytope = self.points.get(id.idx.0).context("bad polytope index")?;

        Ok(&polytope.point)
    }
    /// Returns the children of a non-point polytope.
    pub fn polytope_child_ids(
        &self,
        id: PolytopeId,
    ) -> Result<impl Iterator<Item = PolytopeId> + ExactSizeIterator> {
        let rank = id.rank;
        let child_rank = rank - 1;

        let polytope = self.non_points[self.rank_index(rank)?]
            .get(id.idx.0)
            .context("bad polytope index")?;

        Ok(polytope.child_ids().map(move |idx| PolytopeId {
            rank: child_rank,
            idx,
        }))
    }
    /// Returns the set of facets that a polytope is a part of.
    pub fn polytope_facet_set(&self, id: PolytopeId) -> Result<&FacetSet> {
        if id.rank == 0 {
            let data = self.points.get(id.idx.0).context("bad polytope index")?;
            Ok(&data.facet_set)
        } else {
            let i = self.rank_index(id.rank)?;
            let data = self.non_points[i]
                .get(id.idx.0)
                .context("bad polytope index")?;
            Ok(&data.facet_set)
        }
    }
    /// Updates a polytope facet set a mutable reference to the facet set of a
    /// polytope.
    fn update_polytope_facet_set(
        &mut self,
        id: PolytopeId,
        f: impl FnOnce(&FacetSet) -> FacetSet,
    ) -> Result<()> {
        let facet_set = if id.rank == 0 {
            &mut self
                .points
                .get_mut(id.idx.0)
                .context("bad polytope index")?
                .facet_set
        } else {
            let i = self.rank_index(id.rank)?;
            &mut self.non_points[i]
                .get_mut(id.idx.0)
                .context("bad polytope index")?
                .facet_set
        };

        *facet_set = f(facet_set);

        Ok(())
    }

    /// Slices the polytope by a hyperplane, removing external parts.
    pub fn carve(&mut self, plane: &Hyperplane, facet: Facet) -> Result<()> {
        log::trace!("Carving along plane {plane:?} for {facet:?}");
        self.slice_all_roots(plane, SliceMode::Carve(facet))
    }
    /// Slices the polytope by a hyperplane.
    pub fn slice_internal(&mut self, plane: &Hyperplane) -> Result<()> {
        log::trace!("Slicing along plane {plane:?}");
        self.slice_all_roots(plane, SliceMode::Internal)
    }
    /// Slices every polytope by a hyperplane, removing external parts if
    /// carving.
    fn slice_all_roots(&mut self, plane: &Hyperplane, mode: SliceMode) -> Result<()> {
        let mut op = SliceOperation {
            plane,
            mode,
            results: AHashMap::new(),
        };

        // Update roots.
        for root in std::mem::take(&mut self.roots) {
            match self.slice_polytope(root, &mut op)? {
                SliceResult::Above { .. } if op.mode.remove_above() => (),
                SliceResult::Above { .. } => {
                    self.roots.push(root);
                }
                SliceResult::Below { .. } => {
                    self.roots.push(root);
                }
                SliceResult::Flush => bail!("root polytope cannot be flush"),
                SliceResult::Split { above, below, .. } => {
                    if !op.mode.remove_above() {
                        self.roots.push(above);
                    }
                    self.roots.push(below);
                }
            };
        }

        // Remove dead polytopes.
        for (polytope, result) in op.results {
            match result {
                SliceResult::Above { .. } => {
                    if op.mode.remove_above() {
                        self.remove_nonrecursive(polytope)
                            .context("removing upper polytope of split")?;
                    }
                }
                SliceResult::Below { .. } => (),
                SliceResult::Flush => (),
                SliceResult::Split { above, .. } => {
                    self.remove_nonrecursive(polytope)?;
                    if op.mode.remove_above() {
                        self.remove_nonrecursive(above)
                            .context("removing upper polytope of split")?;
                    }
                }
            }
        }

        self.cache.borrow_mut().invalidate();

        if EXTRA_ASSERTIONS {
            self.validate()?;
        }

        Ok(())
    }
    /// Slices a polytope by a hyperplane and caches the result.
    fn slice_polytope(&mut self, p: PolytopeId, op: &mut SliceOperation) -> Result<SliceResult> {
        // If `p` has already been sliced, then just return that result.
        if let Some(&result) = op.results.get(&p) {
            return Ok(result);
        }

        let result = if p.rank == 0 {
            let point = self.polytope_point(p)?;
            let distance = op.plane.distance_to(point);
            match abs_diff_cmp(&distance, &0.0) {
                std::cmp::Ordering::Less => SliceResult::Below { intersection: None },
                std::cmp::Ordering::Equal => SliceResult::Flush,
                std::cmp::Ordering::Greater => SliceResult::Above { intersection: None },
            }
        } else {
            let rank = p.rank;
            let facet_set = self.polytope_facet_set(p)?.clone();
            let mut children_above: Set64<PolytopeId> = Set64::new();
            let mut children_below: Set64<PolytopeId> = Set64::new();
            let mut flush_grandchildren: Set64<PolytopeId> = Set64::new();
            let mut flush_child = None;

            for child in self.polytope_child_ids(p)? {
                match self.slice_polytope(child, op)? {
                    SliceResult::Above { intersection } => {
                        children_above.insert(child);
                        flush_grandchildren.extend(intersection);
                    }
                    SliceResult::Below { intersection } => {
                        children_below.insert(child);
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
                        children_above.insert(above);
                        children_below.insert(below);
                        flush_grandchildren.insert(intersection);
                    }
                }
            }

            if flush_child.is_none() && flush_grandchildren.len() >= 2 {
                flush_child = Some(self.add_non_point_polytope(
                    op.mode.intersection_facets(&facet_set),
                    flush_grandchildren,
                )?);
            }

            match (children_above.len(), children_below.len()) {
                // All children are flush.
                (0, 0) => SliceResult::Flush,
                // All children are above or flush.
                (_, 0) => SliceResult::Above {
                    intersection: flush_child,
                },
                // All children are below or flush.
                (0, _) => SliceResult::Below {
                    intersection: flush_child,
                },
                // The polytope is split.
                (_, _) => {
                    let intersection = if rank == 1 {
                        ensure!(children_above.len() == 1);
                        ensure!(children_below.len() == 1);
                        let a = self.polytope_point(children_above.iter().next().unwrap())?;
                        let b = self.polytope_point(children_below.iter().next().unwrap())?;
                        let ah = op.plane.distance_to(&a);
                        let bh = op.plane.distance_to(&b);

                        // `a` is above the plane and `b` is below the plane, so
                        // this subtraction actually gives a sum of the absolute
                        // values.
                        ensure!(ah > 0.0);
                        ensure!(bh < 0.0);
                        let sum = ah - bh;

                        // Split this edge into two edges: one above the
                        // plane and one below the plane.
                        let t = ah / sum;

                        self.add_point(op.mode.intersection_facets(&facet_set), util::mix(a, b, t))
                    } else {
                        flush_child.context("split polytope has no flush child")?
                    };

                    // Split this polytope into two polytopes: one above the
                    // plane and one below the plane.
                    children_above.insert(intersection);
                    children_below.insert(intersection);
                    let loc_above = op.mode.facets_above(&facet_set);
                    let loc_below = op.mode.facets_below(&facet_set);
                    let above = self.add_non_point_polytope(loc_above, children_above)?;
                    let below = self.add_non_point_polytope(loc_below, children_below)?;
                    SliceResult::Split {
                        above,
                        below,
                        intersection,
                    }
                }
            }
        };

        // Update this polytope's facet set.
        match result {
            SliceResult::Above { .. } => {
                self.update_polytope_facet_set(p, |l| op.mode.facets_above(l))
            }
            SliceResult::Below { .. } => {
                self.update_polytope_facet_set(p, |l| op.mode.facets_below(l))
            }
            SliceResult::Flush => {
                self.update_polytope_facet_set(p, |l| op.mode.intersection_facets(l))
            }
            SliceResult::Split { .. } => Ok(()),
        }?;

        op.results.insert(p, result);
        Ok(result)
    }

    /// Returns whether any polytopes remain from the primordial hypercube.
    pub fn has_any_primordial(&self) -> bool {
        self.points.iter().any(|(_, data)| {
            data.point
                .iter()
                .any(|coord| abs_diff_eq!(coord.abs(), self.initial_radius, epsilon = EPSILON))
        })
    }
    /// Returns an error and logs if the arena is in an invalid state.
    ///
    /// This runs an expensive check, so avoid it if possible.
    fn validate(&self) -> Result<()> {
        let result = self._validate();
        if result.is_err() {
            log::error!(
                "Invalid polytope arena! Dumping contents as DOT:\n\n{}",
                self.graph_str(),
            );
        }
        result
    }
    fn _validate(&self) -> Result<()> {
        let mut seen = AHashSet::new();

        // Collect all descendents.
        for root in self.roots() {
            root.add_descendents_to_set(&mut seen, 0)?;
        }

        // There should be no orphaned polytopes.
        let total_polytope_count =
            self.points.len() + self.non_points.iter().map(|list| list.len()).sum::<usize>();
        anyhow::ensure!(total_polytope_count == seen.len(), "orphaned polytopes");

        for p in self.non_points() {
            // The facet set of a polytope should be the intersection of the
            // facet sets of its children.
            let expected = p
                .children()?
                .map(|child| child.facet_set())
                .try_fold_intersection()?;

            anyhow::ensure!(
                p.facet_set()? == &expected,
                "polytope {p}'s facet set is not equivalent to the intersection of its children's",
            );
        }

        Ok(())
    }

    /// Produces a Graphviz-compatible DOT file representing the entire polytope
    /// arena. See https://graphviz.org/ for more.
    pub fn graph_str(&self) -> String {
        fn color_str(loc: &FacetSet) -> &'static str {
            if loc.is_empty() {
                "grey" // nothing
            } else {
                "green" // facet(s)
            }
        }

        let mut s = String::new();
        s += "graph {\n";
        for (i, PointData { facet_set, point }) in &self.points {
            let color_str = color_str(facet_set);
            s += &format!("  0.{i} [label=\"{point:.03}\" color={color_str}]\n");
        }
        for p in self.non_points() {
            let i = p.id.idx.0;
            let rank = p.rank();
            let color_str = color_str(p.facet_set().unwrap_or(&FacetSet::new()));
            s += &format!("  {rank}.{i} [style=filled color={color_str}]\n");
            s += &format!("  {rank}.{i} -- ");
            s += "{ ";
            if let Ok(children) = p.children() {
                let child_rank = rank - 1;
                for child in children {
                    let child_idx = child.id.idx.0;
                    s += &format!("{child_rank}.{child_idx} ");
                }
            } else {
                log::error!("Error getting children of polytope {rank}.{i} when assembling graph");
            }
            s += "}\n";
        }
        s += "}\n";
        s
    }
}

/// Immutable reference to a polytope in a polytope arena.
#[derive(Debug, Copy, Clone)]
pub struct PolytopeRef<'a> {
    arena: &'a PolytopeArena,
    id: PolytopeId,
}
impl fmt::Display for PolytopeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.id, f)
    }
}
impl PartialEq for PolytopeRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.arena, other.arena) && self.id == other.id
    }
}
impl Eq for PolytopeRef<'_> {}
impl Hash for PolytopeRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.arena, state);
        self.id.hash(state);
    }
}
impl<'a> PolytopeRef<'a> {
    /// Returns the arena containing the polytope.
    pub fn arena(self) -> &'a PolytopeArena {
        self.arena
    }
    /// Returns the rank of the polytope. (point=0, edge=1, polygon=2, etc.)
    pub fn rank(self) -> u8 {
        self.id.rank
    }
    /// Returns the set of facets that the polytope is a part of.
    pub fn facet_set(self) -> Result<&'a FacetSet> {
        self.arena.polytope_facet_set(self.id)
    }
    /// Returns the set of facets that the polytope is a part of, in sorted
    /// order.
    pub fn sorted_facet_set(self) -> Result<FacetList> {
        let mut ret: SmallVec<_> = self.facet_set()?.iter().collect();
        ret.sort();
        Ok(ret)
    }
    /// Returns the coordinates of the point, or an error if the polytope is not a
    /// point.
    pub fn point(self) -> Result<&'a Vector> {
        self.arena.polytope_point(self.id)
    }
    /// Returns the children of the polytope, or an error if the polytope is a
    /// point and has no children.
    pub fn children(self) -> Result<impl Iterator<Item = PolytopeRef<'a>> + ExactSizeIterator> {
        let arena = self.arena;
        Ok(self
            .arena
            .polytope_child_ids(self.id)?
            .map(|id| PolytopeRef { arena, id }))
    }
    /// Returns the endpoints of the edge, or an error if the polytope is not an
    /// edge.
    pub fn edge_endpoints(self) -> Result<[PolytopeRef<'a>; 2]> {
        let rank = self.rank();
        ensure!(rank == 1, "expected edge; got polytope with rank {rank}");

        let mut children_iter = self.children()?;
        ensure!(children_iter.len() == 2, "edge does not have two children");

        Ok([children_iter.next().unwrap(), children_iter.next().unwrap()])
    }
    /// Returns the indexes of the endpoints of the edge, or an error if the
    /// polytope is not an edge.
    pub fn edge_endpoint_idxs(self) -> Result<[PolytopeIdx; 2]> {
        Ok(self.edge_endpoints()?.map(|v| v.id.idx))
    }
    /// Returns a set containing the polytope and all its descendents.
    pub fn descendents(self) -> Result<AHashSet<PolytopeRef<'a>>> {
        self.descendents_with_rank_at_least(0)
    }
    /// Returns a set containing the polytope and all its descendents at or
    /// above `min_rank`.
    pub fn descendents_with_rank_at_least(self, min_rank: u8) -> Result<AHashSet<PolytopeRef<'a>>> {
        let mut set = AHashSet::new();
        self.add_descendents_to_set(&mut set, min_rank)?;
        Ok(set)
    }
    /// Adds the polytope and its descendents to `set`.
    fn add_descendents_to_set(
        self,
        set: &mut AHashSet<PolytopeRef<'a>>,
        min_rank: u8,
    ) -> Result<()> {
        if !set.contains(&self) {
            set.insert(self);
            if self.rank() > min_rank {
                for child in self.children()? {
                    child.add_descendents_to_set(set, min_rank)?;
                }
            }
        }
        Ok(())
    }

    /// Returns the vertices of the polygon in cyclic order (may be clockwise or
    /// counterclockwise). Returns an error if the polytope is not a polygon.
    pub fn polygon_verts(self) -> Result<impl Iterator<Item = PolytopeRef<'a>>> {
        let rank = self.rank();
        ensure!(
            self.rank() == 2,
            "expected polygon; got polytope with rank {rank}",
        );

        /// Maximum number of vertices in a single polygon before this function
        /// will need to perform a heap allocation.
        const EXPECTED_LEN: usize = 7;

        // Get a list of edges in the polygon.
        let edges: SmallVec<[[PolytopeIdx; 2]; EXPECTED_LEN]> = self
            .children()?
            .map(PolytopeRef::edge_endpoint_idxs)
            .try_collect()?;

        // Now we will assemble a list of vertices in order. Most polygons have
        // fewer than 8 vertices, and 64 bytes is a nice number.
        let mut verts: SmallVec<[PolytopeIdx; EXPECTED_LEN]> = SmallVec::with_capacity(edges.len());

        // Make an adjacency list representing the graph of the polygon.
        let mut adj: AHashMap<PolytopeIdx, SmallVec<[PolytopeIdx; 2]>> = AHashMap::new();
        for &[a, b] in edges.iter() {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }

        let [mut prev, mut current] = edges.first().context("bad child count for edge")?;
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

        Ok(verts.into_iter().map(|idx| PolytopeRef {
            arena: self.arena,
            id: PolytopeId { rank: 0, idx },
        }))
    }
    /// Returns the indexes of the points that are descendents of the polytope.
    pub fn descendent_point_idxs(self) -> Result<Set64<PolytopeIdx>> {
        if self.rank() == 0 {
            Ok([self.id.idx].into_iter().collect())
        } else if self.rank() == 1 {
            Ok(self.children()?.map(|child| child.id.idx).collect())
        } else {
            let mut cache = self.arena.cache.borrow_mut();

            if let Some(result) = cache.point_set.get(&self.id) {
                Ok(result.clone())
            } else {
                let result = self
                    .descendents()?
                    .into_iter()
                    .filter(|p| p.rank() == 0)
                    .map(|p| p.id.idx)
                    .collect::<Set64<_>>();

                cache.point_set.insert(self.id, result.clone());

                Ok(result)
            }
        }
    }
    /// Returns the points that are descendents of the polytope.
    pub fn descendent_points(self) -> Result<impl Iterator<Item = PolytopeRef<'a>>> {
        Ok(self
            .descendent_point_idxs()?
            .into_iter()
            .map(|idx| PolytopeRef {
                arena: self.arena,
                id: PolytopeId { rank: 0, idx },
            }))
    }
    /// Returns the centroid of the polytope.
    pub fn centroid(self) -> Result<Centroid> {
        if let Some(result) = self.arena.cache.borrow().centroids.get(&self.id) {
            return Ok(result.clone());
        }

        let result = if self.rank() == 0 {
            Centroid {
                blade: Multivector::scalar(1.0),
                com: self.point()?.clone(),
            }
        } else if self.rank() == 1 {
            let [a, b] = self.edge_endpoints()?;
            let a = a.point()?;
            let b = b.point()?;
            Centroid {
                // The vector from one point to the other is the blade
                // representing the measure of a line segment.
                blade: (a - b).into(),
                // The centroid of a line segment is its midpoint.
                com: (a + b) * 0.5,
            }
        } else {
            // Compute the centroid of each child.
            let child_volumes: Vec<Centroid> = self
                .children()?
                .map(|child| child.centroid())
                .try_collect()?;

            // Average those centroids to get an arbitrary point inside the
            // polytope. This will be the apex of a pyramid for each child.
            let apex = child_volumes.iter().map(|mass| &mass.com).sum::<Vector>()
                / child_volumes.len() as f32;

            // For each child, construct a pyramid with that child as the base.
            child_volumes
                .iter()
                .map(|v| {
                    // This vector adds a new dimension to the child polytope.
                    let new_vector = &apex - &v.com;

                    let parallelotope_mass =
                        (Multivector::from(new_vector) * &v.blade).grade_project(self.rank());

                    // The measure of a pyramid is `1/NDIM` times the measure of
                    // a parallelotope.
                    let blade = parallelotope_mass * (self.rank() as f32).recip();

                    // In 2D, the centroid of a triangle is 1/3 the way from the
                    // base to the apex. In 3D, it's 1/4 the way up. In N
                    // dimsensions, it's 1/(NDIM+1).
                    let com = util::mix(&v.com, &apex, (self.rank() as f32 + 1.0).recip());

                    Centroid { blade, com }
                })
                .sum::<Result<Centroid>>()?
        };

        self.arena
            .cache
            .borrow_mut()
            .centroids
            .insert(self.id, result.clone());

        Ok(result)
    }
    /// Returns the shrink vector for each `(point, sticker)` pair in a piece.
    pub fn shrink_vectors(self, strategy: ShrinkStrategy) -> Result<ShrinkVectors<'a>> {
        // Make sure we didn't mix up the order of arguments.
        ensure!(self.rank() == self.arena.ndim);

        match strategy {
            ShrinkStrategy::ToPieceCenter => {
                let centroid = self.centroid()?.com;
                let map = self
                    .descendent_points()?
                    .map(|point| Ok((point, &centroid - point.point()?)))
                    .collect::<Result<AHashMap<_, _>>>()?;

                Ok(ShrinkVectors::PerPiece(map))
            }

            ShrinkStrategy::ToStickerCenter => {
                let map = self
                    .children()?
                    .map(|sticker| {
                        let centroid = self.centroid()?.com;
                        Ok(sticker
                            .descendent_points()?
                            .map(move |point| Ok(([sticker, point], &centroid - point.point()?))))
                    })
                    .flatten_ok() // Iter<Result<Iter<Result<T>>>> -> Iter<Result<Result<T>>>
                    .map(|res: Result<_>| res?) // Result<Result<T>> -> Result<T>
                    .collect::<Result<AHashMap<_, _>>>()?;

                Ok(ShrinkVectors::PerSticker(map))
            }

            ShrinkStrategy::ToPuzzleBoundary => {
                // Throughout this algorithm, we will only consider facets that
                // border the piece.
                let mut piece_facets = FacetSet::new();
                for child in self.children()? {
                    piece_facets.extend(child.facet_set()?.iter());
                }

                // Make a mapping from facet sets to the highest-rank polytope
                // with that exact facet set. If the piece doesn't have any
                // degeneracies (such as parallel facets in the same hyperplane)
                // then these should be unique.
                let mut polytopes_by_facet_set: AHashMap<FacetList, PolytopeRef> = AHashMap::new();

                // Make a set (unordered) of maximal facet sets. No entry in
                // this list may be a subset of another entry.
                let mut maximal_facet_sets: Vec<FacetSet> = vec![];

                for p in self.descendents()? {
                    let p_facet_set = set64_intersection(p.facet_set()?, &piece_facets);
                    let p_facet_set_list = p_facet_set.iter().collect();

                    match polytopes_by_facet_set.entry(p_facet_set_list) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(p);
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            if p.rank() > entry.get().rank() {
                                entry.insert(p);
                            }
                        }
                    }

                    let new = p_facet_set;
                    // Remove subsets.
                    maximal_facet_sets.retain(|old| !crate::util::is_subset(old, &new));
                    // Add if not a subset of another element.
                    if maximal_facet_sets
                        .iter()
                        .all(|old| !crate::util::is_subset(&new, old))
                    {
                        maximal_facet_sets.push(new.clone());
                    }
                }

                let map = self
                    .descendent_points()?
                    .map(|point| {
                        let facet_set = set64_intersection(point.facet_set()?, &piece_facets);

                        // Find the intersection of the maximal supersets.
                        let maximal_facet_set: FacetSet = maximal_facet_sets
                            .iter()
                            .filter(|s| crate::util::is_subset(&facet_set, s))
                            .fold_intersection();
                        // Convert to FacetList.
                        let maximal_facet_set: FacetList = maximal_facet_set.into_iter().collect();

                        // Find the largest polytope in the piece whose facet
                        // set is that intersection.
                        let shrink_target_polytope = polytopes_by_facet_set
                            .get(&maximal_facet_set)
                            .unwrap_or_else(|| {
                                log::warn!(
                                    "Couldn't find polytope with maximal \
                                     facet set for sticker shrink target; \
                                     resorting to piece centroid"
                                );
                                &self
                            });

                        // Shrink towards the centroid of that polytope.
                        let centroid = shrink_target_polytope.centroid()?.com;
                        Ok((point, &centroid - point.point()?))
                    })
                    .collect::<Result<AHashMap<_, _>>>()?;

                Ok(ShrinkVectors::PerPiece(map))
            }
        }
    }
}

/// Index of a polytope in a polytope arena.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolytopeId {
    rank: u8,
    idx: PolytopeIdx,
}
impl fmt::Display for PolytopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}.{}", self.rank, self.idx)
    }
}
impl tinyset::Fits64 for PolytopeId {
    unsafe fn from_u64(x: u64) -> Self {
        Self {
            rank: (x & 0xFF) as _,
            idx: PolytopeIdx((x >> 8) as _),
        }
    }

    fn to_u64(self) -> u64 {
        (self.idx.0 as u64) << 8 | self.rank as u64
    }
}
impl PolytopeId {
    /// Returns the rank of the polytope. (point=0, edge=1, polygon=2, etc.)
    pub fn rank(self) -> u8 {
        self.rank
    }
}

/// Index of a polytope of a statically-known rank in a polytope arena.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolytopeIdx(usize);
impl fmt::Display for PolytopeIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl tinyset::Fits64 for PolytopeIdx {
    unsafe fn from_u64(x: u64) -> Self {
        Self(tinyset::Fits64::from_u64(x))
    }

    fn to_u64(self) -> u64 {
        self.0.to_u64()
    }
}

/// Point polytope. (rank = 0)
#[derive(Debug, Clone)]
struct PointData {
    facet_set: FacetSet,
    point: Vector,
}
/// Non-point polytope. (rank > 0)
#[derive(Debug, Clone)]
struct NonPointData {
    facet_set: FacetSet,
    child_ids: Set64<PolytopeIdx>,
}
impl NonPointData {
    /// Returns an iterator over the IDs of the polytope's children.
    fn child_ids(&self) -> impl Iterator<Item = PolytopeIdx> + ExactSizeIterator {
        // At the time of writing, `Set64` iterators don't implement
        // `ExactSizeIterator` even though they totally could.
        self.child_ids
            .clone()
            .into_iter()
            .with_exact_size(self.child_ids.len())
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

/// How to slice the polytope.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SliceMode {
    /// Remove any polytopes above the slicing plane and mark the ones created
    /// by the slice as being part of this facet.
    Carve(Facet),
    /// Mark polytopes created by the slice as internal.
    Internal,
}
impl SliceMode {
    fn remove_above(self) -> bool {
        match self {
            SliceMode::Carve(_) => true,
            SliceMode::Internal => false,
        }
    }
    fn facet(self) -> Option<Facet> {
        match self {
            SliceMode::Carve(f) => Some(f),
            SliceMode::Internal => None,
        }
    }

    fn intersection_facets(self, original_facet_set: &FacetSet) -> FacetSet {
        let mut ret = original_facet_set.clone();
        ret.extend(self.facet());
        ret
    }
    fn facets_below(self, original_facet_set: &FacetSet) -> FacetSet {
        original_facet_set.clone()
    }
    fn facets_above(self, original_facet_set: &FacetSet) -> FacetSet {
        match self {
            SliceMode::Carve(_) => FacetSet::new(),
            SliceMode::Internal => original_facet_set.clone(),
        }
    }
}

/// In-progress slice operation
#[derive(Debug)]
struct SliceOperation<'a> {
    plane: &'a Hyperplane,
    mode: SliceMode,
    results: AHashMap<PolytopeId, SliceResult>,
}

#[derive(Debug, Default, Clone)]
struct PolytopeArenaCache {
    radius: Option<f32>,
    centroids: AHashMap<PolytopeId, Centroid>,

    /// For each polytope, the set of points it contains.
    point_set: AHashMap<PolytopeId, Set64<PolytopeIdx>>,
}
impl PolytopeArenaCache {
    fn invalidate(&mut self) {
        *self = Self::default();
    }
}

/// Centroid and Lebasgue measure of a polytope. In simpler terms: the "center
/// of mass" and "N-dimensional mass" of a polytope.
#[derive(Debug, Clone, PartialEq)]
pub struct Centroid {
    /// Lebasgue measure (https://en.wikipedia.org/wiki/Lebesgue_measure) as a
    /// blade.
    pub blade: Multivector,
    /// Center of mass.
    pub com: Vector,
}
impl Sum<Centroid> for Result<Centroid> {
    fn sum<I: Iterator<Item = Centroid>>(iter: I) -> Self {
        // This function assumes that all the masses are in the same subspaace.

        let mut iter = iter.peekable();
        let first = iter
            .peek()
            .context("empty polytope has no center of mass")?;

        // Some of these masses may have opposite signs. We want all masses to
        // be positive, so pick some component to normalize the signs with
        // respect to.
        let component = first.blade.most_significant_component();
        let unit_mass = &first.blade
            * first
                .blade
                .get(component)
                .context("child of polytope has zero mass")?
                .recip();

        let mut total_com = Vector::EMPTY;
        let mut total_weight = 0.0;

        for it in iter {
            let weight = it.blade.get(component).unwrap_or(0.0).abs();
            total_com += it.com * weight;
            total_weight += weight;
        }

        Ok(Centroid {
            blade: unit_mass * total_weight,
            com: total_com / total_weight,
        })
    }
}

fn powers_of_3() -> impl Iterator<Item = usize> {
    std::iter::successors(Some(1), |x| Some(x * 3))
}
fn base_3_expansion(n: usize, digit_count: u8) -> impl Iterator<Item = usize> {
    std::iter::successors(Some(n), |x| Some(x / 3))
        .take(digit_count as _)
        .map(|x| x % 3)
}

/// Way of shrinking polytopes on a puzzle.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ShrinkStrategy {
    /// Shrink each point toward the center of mass of the piece.
    ToPieceCenter,
    /// Shrink each point toward the center of mass of the sticker.
    ToStickerCenter,
    /// Shrink each point toward the center of mass of the "most relevant"
    /// puzzle boundary. The actual algorithm is kinda complicated, but
    /// basically this produces the best results for most puzzles.
    #[default]
    ToPuzzleBoundary,
}

/// Vectors along which to shrink each point, for a given piece.
#[derive(Debug, Clone)]
pub enum ShrinkVectors<'a> {
    /// The shrink vector for a given point does not depend on the sticker.
    PerPiece(AHashMap<PolytopeRef<'a>, Vector>),
    /// The shrink vector for a given point is different for each sticker.
    PerSticker(AHashMap<[PolytopeRef<'a>; 2], Vector>),
}
impl<'a> ShrinkVectors<'a> {
    /// Returns the shrink vector for a point, given which sticker it's on.
    pub fn get(&self, sticker: PolytopeRef<'a>, point: PolytopeRef<'a>) -> Option<&Vector> {
        match self {
            ShrinkVectors::PerPiece(map) => map.get(&point),
            ShrinkVectors::PerSticker(map) => map.get(&[sticker, point]),
        }
    }
}

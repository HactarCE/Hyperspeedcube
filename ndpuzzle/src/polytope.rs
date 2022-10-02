use anyhow::{anyhow, bail, ensure, Context, Result};
use itertools::Itertools;
use slab::Slab;
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{self};
use thiserror::Error;

use crate::math::*;

const EPSILON: f32 = 0.00001;
const SPLIT_MARGIN: f32 = EPSILON * 5000.;

/// Generates a polytope from a set of generators and base facets.
pub fn generate_polytope(
    ndim: u8,
    generators: &[Matrix],
    base_facets: &[Vector],
) -> Result<(PolytopeArena, Vec<Vector>)> {
    let radius = base_facets
        .iter()
        .map(|pole| pole.mag())
        .reduce(f32::max)
        .expect("no base facets");
    let initial_radius = radius * 2.0 * ndim as f32;
    // TODO: check if radius is too small (any original point remains).
    let mut arena = PolytopeArena::new_cube(ndim, initial_radius);

    let mut facet_poles: Vec<Vector> = base_facets.to_vec();
    let mut next_unprocessed = 0;
    while next_unprocessed < facet_poles.len() {
        for gen in generators {
            let new_pole = gen * &facet_poles[next_unprocessed];
            if facet_poles
                .iter()
                .all(|pole| !pole.approx_eq(&new_pole, EPSILON))
            {
                facet_poles.push(new_pole.resize(ndim));
            }
        }
        next_unprocessed += 1;
    }
    for pole in &facet_poles {
        arena.slice_by_plane(
            &Hyperplane {
                normal: pole.normalize().expect("msg"),
                distance: pole.mag(),
            },
            true,
        )?;
        // arena.slice_by_plane(
        //     &Hyperplane {
        //         normal: pole.normalise(),
        //         distance: pole.mag() * 0.33,
        //     },
        //     false,
        // )?;
    }
    Ok((arena, facet_poles))
}

/// Arena of polytopes that can be split.
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

        let mut roots = BTreeSet::new();
        // center of the 3^NDIM cube
        roots.insert(PolytopeId(3_u32.pow(ndim as _) / 2));
        let mut ret = Self {
            polytopes: Slab::new(),
            roots,
            ndim,
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
                PolytopeContents::new_branch(rank, children, true)
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
    fn get(&self, id: PolytopeId) -> Result<&Polytope, NullPolytope> {
        self.polytopes.get(id.0 as _).ok_or(NullPolytope)
    }
    /// Returns a mutable reference to a polytope by ID.
    fn get_mut(&mut self, id: PolytopeId) -> Result<&mut Polytope, NullPolytope> {
        self.polytopes.get_mut(id.0 as _).ok_or(NullPolytope)
    }

    /// Adds a polytope to the arena.
    fn add(&mut self, polytope: Polytope) -> PolytopeId {
        let idx = self.polytopes.insert(polytope);
        PolytopeId(idx as _)
    }
    /// Adds a point to the arena.
    fn add_point(&mut self, point: Vector, slice_result: Option<SliceResult>) -> PolytopeId {
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
        internal: bool,
    ) -> Result<PolytopeId> {
        ensure!(
            !children.is_empty(),
            "Cannot add rank {rank} polytope with no children",
        );

        if rank == 1 {
            assert_eq!(children.len(), 2);
        }
        let ret = self.add(Polytope {
            parents: smallvec![],
            contents: PolytopeContents::Branch {
                rank,
                children: children.clone(),
                slice_result,
                internal,
            },
        });

        for child in children {
            let child = self.get_mut(child)?;
            assert!(
                child.rank() + 1 == rank,
                "Cannot add rank {rank} polytope with rank {} child",
                child.rank(),
            );
            child.parents.push(ret);
        }
        Ok(ret)
    }
    /// Recursively delete a polytope.
    fn delete_polytope(&mut self, id: PolytopeId) {
        if let Ok(Ok(children)) = self.get(id).map(|p| p.children().cloned()) {
            for child in children {
                self.delete_polytope(child);
            }
        }
        self.polytopes.try_remove(id.0 as usize);
    }
    pub fn remove_internal(&mut self) -> Result<()> {
        for root in self.roots.clone() {
            if self.is_piece_internal(root)? {
                self.delete_polytope(root);
                self.roots.remove(&root);
            }
        }
        Ok(())
    }

    pub fn is_internal(&self, id: PolytopeId) -> Result<bool> {
        Ok(self.get(id)?.is_internal())
    }
    pub fn is_piece_internal(&self, id: PolytopeId) -> Result<bool> {
        let p = self.get(id)?;
        Ok(p.children()?
            .iter()
            .all(|&c| self.is_internal(c).expect("Bad child")))
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
        if !no_internal || !polytope.is_internal() {
            if polytope.rank() == 2 {
                let edges: Vec<[PolytopeId; 2]> =
                    polytope
                        .children()?
                        .iter()
                        .map(|&p| -> Result<[PolytopeId; 2]> {
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

    /// Slices the polytope by a hyperplane, removing external parts if carving.
    pub fn slice_by_plane(&mut self, plane: &Hyperplane, carving: bool) -> Result<()> {
        for root in std::mem::take(&mut self.roots) {
            match self.slice_polytope(root, plane)? {
                SliceResult::Above => {
                    if carving {
                        self.delete_polytope(root);
                    } else {
                        self.roots.insert(root);
                    }
                }
                SliceResult::Below => {
                    self.roots.insert(root);
                }
                SliceResult::Split { above, below } => {
                    if let Some(above) = above {
                        if carving {
                            self.delete_polytope(above);
                        } else {
                            self.roots.insert(above);
                        }
                    }
                    if let Some(below) = below {
                        self.roots.insert(below);
                    }
                }
                SliceResult::New { .. } => bail!("Polytope did not get sliced"),
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
                    if carving {
                        polytope.set_internal(false);
                    }
                    true
                }
            }
        });
        Ok(())
    }

    fn slice_polytope(&mut self, p: PolytopeId, plane: &Hyperplane) -> Result<SliceResult> {
        let polytope = self.get(p).context("First get")?;

        if let Some(ret) = polytope.slice_result() {
            return Ok(ret);
        }

        let ret = match &polytope.contents {
            PolytopeContents::Point { point, .. } => {
                let distance = plane.distance_to(point);
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
            PolytopeContents::Branch {
                rank,
                children,
                internal,
                ..
            } => {
                let rank = *rank;
                let internal = *internal;
                let old_children = children.clone();
                let mut children_above: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut children_below: SmallVec<[PolytopeId; 4]> = smallvec![];
                let mut intersection_children_above = smallvec![];
                let mut intersection_children_below = smallvec![];

                let mut split_flag = false;
                for &child in &old_children {
                    match self.slice_polytope(child, plane)? {
                        SliceResult::Above => children_above.push(child),
                        SliceResult::Below => children_below.push(child),
                        SliceResult::Split { above, below } => {
                            split_flag = true;
                            if let Some(above) = above {
                                children_above.push(above);
                                intersection_children_above.push(
                                    self.get(above)
                                        .context("Split child above")?
                                        .intersection_child()?,
                                );
                            }
                            if let Some(below) = below {
                                children_below.push(below);
                                intersection_children_below.push(
                                    self.get(below)
                                        .context("Split child below")?
                                        .intersection_child()?,
                                );
                            }
                        }
                        SliceResult::New { .. } => bail!("Polytope did not get sliced"),
                    }
                }
                // if rank == 2 && intersection_children_below.len() == 4 {
                //     dbg!(rank);
                //     dbg!(split_flag);
                //     dbg!(&children_above);
                //     dbg!(&children_below);
                //     dbg!(&intersection_children_above);
                //     dbg!(&intersection_children_below);
                //     for &child in &children_below {
                //         for &child2 in self.get(child)?.children()? {
                //             dbg!(self.get(child2)?.unwrap_point());
                //         }
                //     }
                //     for &child in &intersection_children_below {
                //         dbg!(self.get(child)?.unwrap_point());
                //     }
                // }

                if rank == 1 {
                    match (children_above.as_slice(), children_below.as_slice()) {
                        // Both children are above.
                        ([_, _], []) => SliceResult::Above,
                        // Both children are below.
                        ([], [_, _]) => SliceResult::Below,
                        // Children got deleted.
                        ([], []) => SliceResult::Split {
                            above: None,
                            below: None,
                        },
                        // Children are on opposite sides.
                        _ => {
                            let mut a = self
                                .get(old_children[0])
                                .context("Old children 0")?
                                .unwrap_point()?
                                .clone();
                            let mut b = self
                                .get(old_children[1])
                                .context("Old children 1")?
                                .unwrap_point()?
                                .clone();
                            let mut ah = plane.distance_to(&a);
                            let mut bh = plane.distance_to(&b);
                            if ah < bh {
                                // ensure a is above and b is below the plane
                                std::mem::swap(&mut a, &mut b);
                                std::mem::swap(&mut ah, &mut bh);
                            }
                            let sum = ah - bh; // signs are opposite
                            if (1. / sum).is_finite() {
                                let above = children_above
                                    .first()
                                    .map(|&child_above| {
                                        let t = (ah - SPLIT_MARGIN) / sum;
                                        let intersection =
                                            self.add_point(&b * t + &a * (1. - t), None);
                                        self.add_branch(
                                            1,
                                            smallvec![child_above, intersection],
                                            Some(SliceResult::New { intersection }),
                                            false,
                                        )
                                    })
                                    .transpose()?;
                                let below = children_below
                                    .first()
                                    .map(|&child_below| {
                                        let t = (ah + SPLIT_MARGIN) / sum;
                                        let intersection =
                                            self.add_point(&b * t + &a * (1. - t), None);
                                        self.add_branch(
                                            1,
                                            smallvec![child_below, intersection],
                                            Some(SliceResult::New { intersection }),
                                            false,
                                        )
                                    })
                                    .transpose()?;
                                SliceResult::Split { above, below }
                            } else {
                                SliceResult::Split {
                                    above: None,
                                    below: None,
                                }
                            }
                        }
                    }
                } else {
                    match (children_above.as_slice(), children_below.as_slice()) {
                        // All children are above.
                        (_, []) if !split_flag => SliceResult::Above,
                        // All children are below.
                        ([], _) if !split_flag => SliceResult::Below,
                        // Children are on both sides.
                        _ => {
                            let above = (intersection_children_above.len() >= 2)
                                .then(|| {
                                    let intersection_above = self.add_branch(
                                        rank - 1,
                                        intersection_children_above,
                                        None,
                                        rank == self.ndim || internal,
                                    )?;
                                    children_above.push(intersection_above);
                                    self.add_branch(
                                        rank,
                                        children_above,
                                        Some(SliceResult::New {
                                            intersection: intersection_above,
                                        }),
                                        internal,
                                    )
                                })
                                .transpose()?;
                            let below = (intersection_children_below.len() >= 2)
                                .then(|| {
                                    let intersection_below = self.add_branch(
                                        rank - 1,
                                        intersection_children_below,
                                        None,
                                        rank == self.ndim || internal,
                                    )?;
                                    children_below.push(intersection_below);
                                    self.add_branch(
                                        rank,
                                        children_below,
                                        Some(SliceResult::New {
                                            intersection: intersection_below,
                                        }),
                                        internal,
                                    )
                                })
                                .transpose()?;
                            SliceResult::Split { above, below }
                        }
                    }
                }
            }
        };
        self.get_mut(p)
            .context("Final slice result of self")?
            .set_slice_result(ret);
        Ok(ret)
    }

    pub fn above_plane(&self, plane: &Hyperplane) -> Result<(bool, Vec<PolytopeId>)> {
        let mut blocked = false;
        let mut res: Vec<PolytopeId> = vec![];
        for &root in &self.roots {
            match self.split_polytope(root, plane)? {
                SplitResult::Above => res.push(root),
                SplitResult::Below => {}
                SplitResult::Blocking => blocked = true,
            }
        }
        Ok((blocked, res))
    }

    fn split_polytope(&self, p: PolytopeId, plane: &Hyperplane) -> Result<SplitResult> {
        let polytope = self.get(p)?;

        let ret = match &polytope.contents {
            PolytopeContents::Point { point, .. } => {
                if plane.distance_to(point) < 0. {
                    SplitResult::Below
                } else {
                    SplitResult::Above
                }
            }
            PolytopeContents::Branch {
                rank: _, children, ..
            } => {
                let mut children_above = false;
                let mut children_below = false;

                for &child in children {
                    match self.split_polytope(child, plane)? {
                        SplitResult::Above => children_above = true,
                        SplitResult::Below => children_below = true,
                        SplitResult::Blocking => {}
                    }
                }
                match (children_above, children_below) {
                    // All children are above.
                    (true, false) => SplitResult::Above,
                    // Children are on both sides.
                    (true, true) => SplitResult::Blocking,
                    // All children are below.
                    (false, true) => SplitResult::Below,
                    _ => bail!("No children found"),
                }
            }
        };
        Ok(ret)
    }

    pub fn axis_spans(&self, axis: &Vector) -> Result<Vec<(PolytopeId, Span)>> {
        self.roots
            .iter()
            .map(|&p| Ok((p, self.polytope_axis_span(p, axis)?)))
            .collect()
    }
    fn polytope_axis_span(&self, p: PolytopeId, axis: &Vector) -> Result<Span> {
        let polytope = self.get(p)?;

        match &polytope.contents {
            PolytopeContents::Point { point, .. } => {
                let distance = point.dot(axis);
                Ok(Span {
                    above: distance,
                    below: distance,
                })
            }
            PolytopeContents::Branch { children, .. } => children
                .iter()
                .map(|child| self.polytope_axis_span(*child, axis))
                .reduce(|a, b| Ok(a?.union(b?)))
                .unwrap_or(Err(anyhow!("Bad child count"))),
        }
    }

    pub fn transform_polytope(&mut self, root: PolytopeId, m: &Matrix) -> Result<()> {
        self.transform_recurse(root, &mut HashSet::new(), &mut |arena, id| {
            let polytope = arena.get_mut(id)?;
            if let PolytopeContents::Point { point, .. } = &mut polytope.contents {
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
    /// Sets whether this polytope is external.
    fn set_internal(&mut self, new_internal: bool) {
        if let PolytopeContents::Branch { internal, .. } = &mut self.contents {
            *internal = new_internal;
        }
    }
    /// Returns whether this polytope is external.
    fn is_internal(&self) -> bool {
        match self.contents {
            PolytopeContents::Point { .. } => true,
            PolytopeContents::Branch { internal, .. } => internal,
        }
    }
    /// Returns the coordinate point if this polytope is a point, or an error if
    /// is a branch.
    fn unwrap_point(&self) -> Result<&Vector> {
        match &self.contents {
            PolytopeContents::Point { point, .. } => Ok(point),
            _ => Err(anyhow!("Expected point, got rank {} polytope", self.rank())),
        }
    }

    /// Returns the children of the polytope if it is a branch, or an error if
    /// it is a point.
    fn children(&self) -> Result<&SmallVec<[PolytopeId; 4]>> {
        match &self.contents {
            PolytopeContents::Point { .. } => bail!("Can't get children of point"),
            PolytopeContents::Branch { children, .. } => Ok(children),
        }
    }
    /// Returns the intersection between the polytope and the slicing hyperplane.
    fn intersection_child(&self) -> Result<PolytopeId> {
        match self.slice_result() {
            Some(SliceResult::New { intersection }) => return Ok(intersection),
            _ => {
                dbg!(self);
                todo!()
            }
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
        point: Vector,
        slice_result: Option<SliceResult>,
    },
    Branch {
        rank: u8,
        children: SmallVec<[PolytopeId; 4]>,
        slice_result: Option<SliceResult>,
        internal: bool, //todo: consider enum of states
    },
}
impl PolytopeContents {
    /// Constructs a point polytope.
    fn new_point(point: Vector) -> Self {
        Self::Point {
            point,
            slice_result: None,
        }
    }
    /// Constructs a non-point polytope.
    fn new_branch(rank: u8, children: SmallVec<[PolytopeId; 4]>, internal: bool) -> Self {
        Self::Branch {
            rank,
            children,
            slice_result: None,
            internal,
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
        above: Option<PolytopeId>,
        below: Option<PolytopeId>,
    },
    /// The polytope was produced by the slice.
    New { intersection: PolytopeId },
}

/// Result of slicing a polytope with a hyperplane.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum SplitResult {
    /// The whole polytope is above the slice.
    Above,
    /// The whole polytope is below the slice.
    Below,
    /// The polytope is cut by the slice.
    Blocking,
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
    #[error("internal error: bad matrix")]
    BadMatrix,
    #[error("internal error: expected point, got branch with rank {rank}")]
    ExpectedPoint { rank: u8 },
    #[error("internal error: expected branch, got point")]
    ExpectedBranch,
}

#[derive(Debug)]
pub struct NullPolytope;
impl std::fmt::Display for NullPolytope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Null Polytope")
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

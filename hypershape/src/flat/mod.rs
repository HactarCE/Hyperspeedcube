//! Infinite Euclidean space in which flat polytopes can be constructed.

use std::collections::{hash_map, HashMap};
use std::fmt;
use std::ops::Index;

use eyre::{bail, ensure, eyre, OptionExt, Result};
use float_ord::FloatOrd;
use hypermath::collections::generic_vec::IndexOverflow;
use hypermath::collections::{ApproxHashMap, GenericVec};
use hypermath::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use tinyset::Set64;

mod cut;
mod cut_output;
mod map;
mod polytope;

pub use cut::{Cut, CutParams, PolytopeFate};
pub use cut_output::PolytopeCutOutput;
pub use map::{SpaceMap, SpaceMapFor};
pub use polytope::{PolytopeData, PolytopeFlags};

/// Set of vertices in a [`Space`].
pub type VertexSet = Set64<VertexId>;
/// Set of polytopes in a [`Space`].
pub type PolytopeSet = Set64<PolytopeId>;

hypermath::idx_struct! {
    /// ID for a memoized vertex in a [`Space`].
    pub struct VertexId(pub u32);
    /// ID for a memoized polytope in a [`Space`].
    pub struct PolytopeId(pub u32);
}

/// List containing a value per vertex.
pub type PerVertex<T> = GenericVec<VertexId, T>;
/// List containing a value per polytope.
pub type PerPolytope<T> = GenericVec<PolytopeId, T>;

/// Patch of Euclidean (i.e., flat) space in which polytopes can be constructed.
pub struct Space {
    /// Number of dimensions of the space.
    ndim: u8,

    vertices: PerVertex<Vector>,
    vertex_data_to_id: ApproxHashMap<Vector, VertexId>,

    polytopes: PerPolytope<PolytopeData>,
    polytope_data_to_id: HashMap<PolytopeData, PolytopeId>,

    cached_subspaces: Mutex<HashMap<PolytopeId, (Vec<Vector>, pga::Blade)>>,
    cached_vertex_set: Mutex<HashMap<PolytopeId, VertexSet>>,

    cached_which_side_has_polytope: ApproxHashMap<Hyperplane, HashMap<PolytopeId, WhichSide>>,
}

impl fmt::Debug for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Space")
            .field("ndim", &self.ndim())
            .finish_non_exhaustive()
    }
}

impl Index<VertexId> for Space {
    type Output = Vector;

    fn index(&self, index: VertexId) -> &Self::Output {
        &self.vertices[index]
    }
}
impl Index<PolytopeId> for Space {
    type Output = PolytopeData;

    fn index(&self, index: PolytopeId) -> &Self::Output {
        &self.polytopes[index]
    }
}

impl Space {
    /// Constructs a new space containing no polytopes.
    ///
    /// # Panics
    ///
    /// Panics if `ndim > 7`.
    pub fn new(ndim: u8) -> Self {
        assert!(ndim >= 1, "ndim={ndim} is below min value of 1");
        assert!(ndim <= 7, "ndim={ndim} exceeds max value of 7");
        Self {
            ndim,

            vertices: PerVertex::new(),
            vertex_data_to_id: ApproxHashMap::new(),

            polytopes: PerPolytope::new(),
            polytope_data_to_id: HashMap::new(),

            cached_subspaces: Mutex::new(HashMap::new()),
            cached_vertex_set: Mutex::new(HashMap::new()),

            cached_which_side_has_polytope: ApproxHashMap::new(),
        }
    }

    /// Returns the number of dimensions of the space.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns the polytope ID for a vertex.
    pub fn vertex_to_polytope(&self, v: VertexId) -> PolytopeId {
        self.polytope_data_to_id[&PolytopeData::Vertex(v)]
    }

    /// Memoizes a vertex.
    pub fn add_vertex(&mut self, v: Vector) -> Result<VertexId, IndexOverflow> {
        match self.vertex_data_to_id.entry(v.clone()) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                let vertex_id = *e.insert(self.vertices.push(v)?);
                // Ensure that the vertex has a polytope ID as well.
                self.add_polytope(vertex_id.into())?;
                Ok(vertex_id)
            }
        }
    }
    /// Memoizes a line.
    pub fn add_line(
        &mut self,
        points: [VertexId; 2],
        flags: PolytopeFlags,
    ) -> Result<PolytopeId, IndexOverflow> {
        let [a, b] = points;
        let a = self.add_polytope(a.into())?;
        let b = self.add_polytope(b.into())?;
        self.add_polytope(PolytopeData::Polytope {
            rank: 1,
            boundary: PolytopeSet::from_iter([a, b]),
            flags,
        })
    }
    /// Memoizes a polytope.
    pub fn add_polytope(&mut self, mut p: PolytopeData) -> Result<PolytopeId, IndexOverflow> {
        // Validate the boundary of the polytope.
        #[cfg(debug_assertions)]
        match &mut p {
            PolytopeData::Vertex(_) => (),
            PolytopeData::Polytope { rank, boundary, .. } => {
                for b in boundary.iter() {
                    assert_eq!(self[b].rank() + 1, *rank, "bad boundary ranks of polytope");
                }
                if *rank == 1 {
                    assert_eq!(boundary.len(), 2, "line must have two endpoints");
                    assert!(
                        !boundary.iter().all_equal(),
                        "line endpoints must be distinct",
                    );
                }
                if *rank == 2 {
                    let mut multiplicity = HashMap::<VertexId, usize>::new();
                    for b in boundary.iter() {
                        for v in self.line_endpoints(b).expect("expected line") {
                            *multiplicity.entry(v).or_default() += 1;
                        }
                    }
                    for &m in multiplicity.values() {
                        assert_eq!(m, 2, "bad polygon structure");
                    }
                }
            }
        }

        match self.polytope_data_to_id.entry(p.clone()) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => Ok(*e.insert(self.polytopes.push(p)?)),
        }
    }
    fn add_polytope_if_non_degenerate(
        &mut self,
        p: PolytopeData,
    ) -> Result<Option<PolytopeId>, IndexOverflow> {
        if let PolytopeData::Polytope { rank, boundary, .. } = &p {
            if boundary.len() <= *rank as usize {
                return Ok(None);
            }
        }
        self.add_polytope(p).map(Some)
    }

    /// Returns the endpoints of a line, or an error if `line` is not a line.
    pub fn line_endpoints(&self, line: PolytopeId) -> Option<[VertexId; 2]> {
        let mut points = self[line]
            .boundary()
            .ok()?
            .iter()
            .map(|p| self[p].to_vertex());
        Some([points.next()??, points.next()??])
    }

    /// Adds a primordial cube to the space. When converting a shape to
    /// simplexes, any polytope flush with a facet of the primordial cube will
    /// produce an error.
    pub fn add_primordial_cube(&mut self, size: Float) -> Result<PolytopeId, IndexOverflow> {
        // Construct a 3^d array of polytope elements. Along each axis X, the
        // polytopes at X=0 and X=1 are on the boundary of X=2.
        let mut elements = Vec::<PolytopeId>::with_capacity(3_usize.pow(self.ndim as _));
        let mut boundary_indexes = Vec::<usize>::with_capacity(1 << self.ndim);
        let mut position = vec![0_u8; self.ndim as usize];
        'outer: loop {
            let zero_axes = position.iter().positions(|&x| x == 2).collect_vec();
            let element_rank = zero_axes.len() as u8;
            let polytope_data = if element_rank == 0 {
                self.add_vertex(
                    position
                        .iter()
                        .map(|&x| size * (x as Float * 2.0 - 1.0))
                        .collect(),
                )?
                .into()
            } else {
                let stride = |i| 3_usize.pow(i as _);
                boundary_indexes.clear();
                let base: usize = position
                    .iter()
                    .enumerate()
                    .map(|(i, &x)| stride(i) * x as usize)
                    .sum();
                let boundary_indexes = position
                    .iter()
                    .positions(|&x| x == 2)
                    .flat_map(|i| [base - stride(i), base - stride(i) * 2])
                    .collect_vec();

                let boundary = boundary_indexes.iter().map(|&i| elements[i]).collect();
                PolytopeData::Polytope {
                    rank: element_rank,
                    boundary,
                    flags: PolytopeFlags {
                        is_primordial: element_rank < self.ndim,
                    },
                }
            };

            let new_id = self.add_polytope(polytope_data)?;
            if element_rank == self.ndim {
                // We've constructed the whole cube!
                return Ok(new_id);
            }
            elements.push(new_id);

            // Move to the next element position.
            for k in 0..self.ndim as usize {
                position[k] += 1;
                if position[k] > 2 {
                    position[k] = 0;
                } else {
                    continue 'outer;
                }
            }
        }
    }

    /// Cuts each polytope in a set.
    pub fn cut_polytope_set(
        &mut self,
        polytopes: PolytopeSet,
        cut: &mut Cut,
    ) -> Result<PolytopeSet> {
        polytopes
            .iter()
            .map(|polytope| Ok(self.cut_polytope(polytope, cut)?.iter_inside_and_outside()))
            .flatten_ok()
            .collect()
    }
    /// Cut a polytope.
    pub fn cut_polytope(
        &mut self,
        polytope: PolytopeId,
        cut: &mut Cut,
    ) -> Result<PolytopeCutOutput> {
        if let Some(&result) = cut.polytope_cut_output_cache.get(&polytope) {
            return Ok(result);
        }

        let div = &cut.params.divider;

        let result = match self[polytope].clone() {
            PolytopeData::Vertex(p) => match div.location_of_point(&self[p]) {
                PointWhichSide::On => PolytopeCutOutput::Flush,
                PointWhichSide::Inside => PolytopeCutOutput::all_inside(polytope, None),
                PointWhichSide::Outside => PolytopeCutOutput::all_outside(polytope, None),
            },
            PolytopeData::Polytope {
                rank,
                boundary,
                flags,
            } => {
                let mut inside_boundary = PolytopeSet::new();
                let mut outside_boundary = PolytopeSet::new();
                let mut flush_polytopes = vec![];
                let mut flush_polytope_boundary = PolytopeSet::new();

                if let Some(line @ [a, b]) = self.line_endpoints(polytope) {
                    let HyperplaneLineIntersection {
                        a_loc,
                        b_loc,
                        intersection,
                    } = div.intersection_with_line_segment(line.map(|i| &self[i]));
                    for (v, v_loc) in [(a, a_loc), (b, b_loc)] {
                        let v = self.add_polytope(v.into())?;
                        match v_loc {
                            PointWhichSide::On => flush_polytopes.push(v),
                            PointWhichSide::Inside => {
                                inside_boundary.insert(v);
                            }
                            PointWhichSide::Outside => {
                                outside_boundary.insert(v);
                            }
                        }
                    }
                    if flush_polytopes.is_empty() {
                        if let Some(intersection_point) = intersection {
                            let v = self.add_vertex(intersection_point)?.into();
                            flush_polytopes.push(self.add_polytope(v)?);
                        }
                    }
                } else {
                    for b in boundary.iter() {
                        match self.cut_polytope(b, cut)? {
                            PolytopeCutOutput::Flush => flush_polytopes.push(b),
                            PolytopeCutOutput::NonFlush {
                                inside,
                                outside,
                                intersection,
                            } => {
                                inside_boundary.extend(inside);
                                outside_boundary.extend(outside);
                                flush_polytope_boundary.extend(intersection);
                            }
                        }
                    }
                }

                if flush_polytopes.len() > 1 {
                    PolytopeCutOutput::Flush
                } else {
                    let intersection = match flush_polytopes.first() {
                        Some(&p) => Some(p),
                        None => self.add_polytope_if_non_degenerate(PolytopeData::Polytope {
                            rank: rank - 1,
                            boundary: flush_polytope_boundary,
                            flags: PolytopeFlags::default(),
                        })?,
                    };

                    let inside = match cut.params.inside {
                        PolytopeFate::Keep => {
                            inside_boundary.extend(intersection);
                            self.add_polytope_if_non_degenerate(PolytopeData::Polytope {
                                rank,
                                boundary: inside_boundary,
                                flags,
                            })?
                        }
                        PolytopeFate::Remove => None,
                    };

                    let outside = match cut.params.outside {
                        PolytopeFate::Keep => {
                            outside_boundary.extend(intersection);
                            self.add_polytope_if_non_degenerate(PolytopeData::Polytope {
                                rank,
                                boundary: outside_boundary,
                                flags,
                            })?
                        }
                        PolytopeFate::Remove => None,
                    };

                    PolytopeCutOutput::NonFlush {
                        inside,
                        outside,
                        intersection,
                    }
                }
            }
        };

        cut.polytope_cut_output_cache.insert(polytope, result);
        Ok(result)
    }

    /// Returns the set of vertices of the polytope. This is exactly the vertex
    /// set of the convex hull.
    pub fn vertex_set(&self, polytope: PolytopeId) -> VertexSet {
        if let Some(result) = self.cached_vertex_set.lock().get(&polytope) {
            return result.clone();
        }

        let result = match &self[polytope] {
            PolytopeData::Vertex(p) => VertexSet::from_iter([*p]),
            PolytopeData::Polytope { boundary, .. } => {
                let boundary = boundary.clone();
                boundary.iter().flat_map(|b| self.vertex_set(b)).collect()
            }
        };

        self.cached_vertex_set
            .lock()
            .insert(polytope, result.clone());
        result
    }

    /// Returns an arbitrary vertex on the polytope.
    pub fn arbitrary_vertex(&self, polytope: PolytopeId) -> VertexId {
        self.vertex_set(polytope)
            .iter()
            .next()
            .expect("degenerate polytope")
    }

    /// Returns the hyperplane in which a facet lives. Returns an error if the
    /// polytope is not a facet (rank `ndim-1`).
    pub fn hyperplane_of_facet(&self, facet: PolytopeId) -> Result<Hyperplane> {
        let expected_rank = self.ndim - 1;
        let actual_rank = self[facet].rank();
        if expected_rank != actual_rank {
            bail!("expected polytope with rank {expected_rank}; got {actual_rank}");
        }
        self.blade_for_subspace_of_polytope(facet)?
            .to_hyperplane()
            .ok_or_eyre("error converting polytope subspace to hyperplane")
    }

    /// Returns a basis for the smallest subspace in which the polytope lives.
    /// The result is cached.
    pub fn basis_for_subspace_of_polytope(&self, polytope: PolytopeId) -> Result<Vec<Vector>> {
        let (basis, _blade) = self.basis_and_blade_for_subspace_of_polytope(polytope)?;
        Ok(basis)
    }
    /// Returns a PGA blade representing the smallest subspace in which a
    /// polytope lives. The result is cached.
    pub fn blade_for_subspace_of_polytope(&self, polytope: PolytopeId) -> Result<pga::Blade> {
        let (_basis, blade) = self.basis_and_blade_for_subspace_of_polytope(polytope)?;
        Ok(blade)
    }
    // TODO: this seems broken; remove it
    fn basis_and_blade_for_subspace_of_polytope(
        &self,
        polytope: PolytopeId,
    ) -> Result<(Vec<Vector>, pga::Blade)> {
        let ndim = self.ndim;

        if let Some(result) = self.cached_subspaces.lock().get(&polytope) {
            return Ok(result.clone());
        }

        let result = match &self[polytope] {
            PolytopeData::Vertex(p) => (vec![], pga::Blade::from_point(ndim, &self[*p])),
            PolytopeData::Polytope { boundary, .. } => {
                let boundary = boundary.clone();
                // Select the blade with the largest magnitude -- this indicates
                // a good confidence in its value.
                let (mut basis, old_blade) = boundary
                    .iter()
                    .map(|b| self.basis_and_blade_for_subspace_of_polytope(b))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .max_by_key(|(_basis, blade)| FloatOrd(blade.mag2()))
                    .ok_or_eyre("degenerate polytope")?;
                // Try to wedge with every vertex and take the one with the
                // largest magnitude.
                let (new_vertex, new_point, new_blade) = self
                    .vertex_set(polytope)
                    .iter()
                    .filter_map(|v| {
                        let new_point = pga::Blade::from_point(ndim, &self[v]);
                        let new_blade = pga::Blade::wedge(&old_blade, &new_point)?;
                        Some((v, new_point, new_blade))
                    })
                    .max_by_key(|(_, _, b)| FloatOrd(b.mag2()))
                    .ok_or_eyre("degenerate polytope")?;
                let new_point_projected = new_point
                    .orthogonal_projection_to(&old_blade)
                    .and_then(|b| b.to_vector())
                    .ok_or_eyre("degenerate polytope")?;
                let new_vector = &self[new_vertex] - new_point_projected;
                basis.push(new_vector.normalize().ok_or_eyre("degenerate polytope")?);
                (basis, new_blade)
            }
        };

        self.cached_subspaces
            .lock()
            .insert(polytope, result.clone());
        Ok(result)
    }

    /// Returns a set of the elements of a polytope, of all ranks except points.
    pub fn elements_of(&self, root: PolytopeId) -> PolytopeSet {
        let mut ret = PolytopeSet::new();
        let mut queue = vec![root];
        while let Some(p) = queue.pop() {
            if ret.insert(p) {
                if let Ok(boundary) = self[p].boundary() {
                    queue.extend(boundary.iter());
                }
            }
        }
        ret
    }

    /// Returns the set of all subelements of `root` with rank `rank`.
    pub fn subelements_with_rank(&self, root: PolytopeId, rank: u8) -> PolytopeSet {
        let mut ret = PolytopeSet::from_iter([root]);
        while ret.iter().next().is_some_and(|p| self[p].rank() > rank) {
            ret = ret
                .iter()
                // TODO: handle lines better?
                .filter_map(|p| self[p].boundary().ok())
                .flat_map(|p| p.iter())
                .collect();
        }
        ret
    }
    /// Returns an iterator over all edges of `root`.
    pub fn edges_of(&self, root: PolytopeId) -> impl '_ + Iterator<Item = [VertexId; 2]> {
        self.subelements_with_rank(root, 1)
            .into_iter()
            .flat_map(|edge| self.line_endpoints(edge))
    }

    /// Returns the set of greatest-rank subelements of a set of same-rank
    /// polytopes, if they have such a common set. This is a generalization of
    /// the notion of the infimum on a poset. If there is no such element, then
    /// the empty set is returned. Returns an error if the input polytopes do
    /// not have the same rank.
    pub fn greatest_common_subelements(&mut self, polytopes: PolytopeSet) -> Result<PolytopeSet> {
        let mut rank = polytopes
            .iter()
            .map(|p| self[p].rank())
            .all_equal_value()
            .map_err(|_| eyre!("polytopes have different ranks"))?;

        let mut subelement_sets = polytopes
            .iter()
            .map(|p| PolytopeSet::from_iter([p]))
            .collect_vec();
        while rank > 0 {
            if subelement_sets.is_empty() {
                return Ok(PolytopeSet::new());
            }

            let intersection = intersect_list_of_sets(subelement_sets.clone());
            if !intersection.is_empty() {
                return Ok(intersection);
            }

            subelement_sets = subelement_sets
                .iter()
                .map(|set| {
                    set.iter()
                        .filter_map(|e| Some(self[e].boundary().ok()?.iter()))
                        .flatten()
                        .collect()
                })
                .collect();

            rank -= 1;
        }

        // no intersection
        Ok(PolytopeSet::new())
    }

    /// Returns which side of `divider` contains `polytope`. The result is
    /// cached.
    pub fn which_side_has_polytope(
        &mut self,
        divider: &Hyperplane,
        polytope: PolytopeId,
    ) -> WhichSide {
        let vertex_set = self.vertex_set(polytope);
        *self
            .cached_which_side_has_polytope
            .entry(divider.clone())
            .or_default()
            .entry(polytope)
            .or_insert_with(|| {
                WhichSide::from_points(
                    vertex_set
                        .iter()
                        .map(|v| divider.location_of_point(&self.vertices[v])),
                )
            })
    }

    /// Returns a human-readable string representation of a polytope.
    pub fn dump_to_string(&self, root: PolytopeId) -> String {
        let max_rank = self[root].rank();
        let mut s = String::new();
        let mut stack = vec![root];
        while let Some(p) = stack.pop() {
            for _ in self[p].rank()..max_rank {
                s += "  ";
            }

            if let Some([a, b]) = self.line_endpoints(p) {
                s += &format!("{p}: line {} .. {}", self[a], self[b])
            } else {
                match &self[p] {
                    PolytopeData::Vertex(v) => s += &format!("{p}: point {v}"),
                    PolytopeData::Polytope {
                        rank,
                        boundary,
                        flags,
                    } => {
                        s += &format!("{p}: {rank}D polytope");
                        if flags.is_primordial {
                            s += " (primordial)";
                        }
                        stack.extend(boundary.iter());
                    }
                }
            }
            s.push('\n');
        }
        s
    }
}

fn intersect_list_of_sets<T: Fits64>(sets: impl IntoIterator<Item = Set64<T>>) -> Set64<T> {
    sets.into_iter().reduce(intersect_sets).unwrap_or_default()
}
fn intersect_sets<T: Fits64>(a: Set64<T>, b: Set64<T>) -> Set64<T> {
    if a.len() > b.len() {
        intersect_sets(b, a)
    } else {
        a.iter().filter(|e| b.contains(e)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube() {
        let mut space = Space::new(2);
        let root = space.add_primordial_cube(10.0).unwrap();
        println!("{}", space.dump_to_string(root));
        let result = space
            .cut_polytope(
                root,
                &mut Cut::carve(Hyperplane::from_pole(vector![1.0]).unwrap()),
            )
            .unwrap();
        match result {
            PolytopeCutOutput::Flush => println!("flush"),
            PolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(p) = inside {
                    println!("inside = {}", space.dump_to_string(p));
                    println!();
                }
                if let Some(p) = outside {
                    println!("outside = {}", space.dump_to_string(p));
                    println!();
                }
                if let Some(p) = intersection {
                    println!("intersection = {}", space.dump_to_string(p));
                    println!();
                }
            }
        }
    }
}

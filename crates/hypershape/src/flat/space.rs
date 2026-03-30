use std::sync::atomic::AtomicU64;

use super::*;

/// Global monotonic ID for [`Space`].
static GLOBAL_SPACE_ID: AtomicU64 = AtomicU64::new(0);

/// Infinite Euclidean (i.e., flat) space in which polytopes can be constructed.
pub struct Space {
    /// Unique ID.
    pub(super) id: u64,

    /// Number of dimensions of the space.
    ndim: u8,

    /// Primordial cube.
    pub(super) primordial_cube: PolytopeId,

    /// Coordinates for each vertex.
    ///
    /// Every vertex also has an entry in [`Self::polytopes`].
    pub(super) vertex_coordinates: FlatTiVec<VertexId, Float>,
    pub(super) polytopes: TiVec<ElementId, PolytopeData>,
    pub(super) hyperplanes: TiVec<HyperplaneId, Hyperplane>,

    pub(super) vertex_data_to_id: ApproxHashMap<Point, VertexId>,
    pub(super) polytope_data_to_id: HashMap<PolytopeData, ElementId>,

    /// Cached results for [`Self::vertex_set()`].
    pub(super) cached_vertex_set: Mutex<HashMap<ElementId, Set64<VertexId>>>,
    /// Cached results for [`Self::simplices()`].
    pub(super) cached_simplices: Mutex<HashMap<ElementId, SimplexBlob>>,
    /// Cached results for [`Self::centroid()`].
    pub(super) cached_centroids: Mutex<HashMap<ElementId, Centroid>>,
}

impl fmt::Debug for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Space")
            .field("ndim", &self.ndim())
            .finish_non_exhaustive()
    }
}

impl Space {
    /// Minimum number of dimensions.
    pub const MIN_NDIM: u8 = 1;
    /// Maximum number of dimensions.
    pub const MAX_NDIM: u8 = 7;

    /// Constructs a new space with a primordial cube of radius
    /// [`crate::PRIMORDIAL_CUBE_RADIUS`].
    ///
    /// Returns an error if `ndim` is not in the range
    /// [`Self::MIN_NDIM`]`..=`[`Self::MAX_NDIM`].
    pub fn new(ndim: u8) -> Result<Self> {
        Self::with_primordial_cube_radius(ndim, crate::PRIMORDIAL_CUBE_RADIUS)
    }

    /// Constructs a new space with a primordial cube of radius `radius`.
    ///
    /// Returns an error if `ndim` is not in the range
    /// [`Self::MIN_NDIM`]`..=`[`Self::MAX_NDIM`].
    pub fn with_primordial_cube_radius(ndim: u8, radius: Float) -> Result<Self> {
        let id = GLOBAL_SPACE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let (min, max) = (Self::MIN_NDIM, Self::MAX_NDIM);
        ensure!(ndim >= min, "ndim={ndim} is below min value of {min}");
        ensure!(ndim <= max, "ndim={ndim} exceeds max value of {max}");

        let mut vertex_coordinates: FlatTiVec<VertexId, f64> = FlatTiVec::new(ndim as usize);
        let mut polytopes: TiVec<ElementId, PolytopeData> = TiVec::new();
        let mut hyperplanes: TiVec<HyperplaneId, Hyperplane> = TiVec::new();

        // Construct a 3^d array of polytope elements. Along each axis X, the
        // polytopes at X=0 and X=1 are on the boundary of X=2.
        let mut elements = Vec::<ElementId>::with_capacity(3_usize.pow(ndim as _));
        let mut position = vec![0; ndim as usize];
        let primordial_cube = 'outer: loop {
            let zero_axes = position.iter().positions(|&x| x == 2).collect_vec();
            let element_centroid =
                Point::from_iter(position.iter().map(|&x| [-radius, radius, 0.0][x]));
            let element_rank = zero_axes.len() as u8;
            let polytope_data = if element_rank == 0 {
                let vertex_id = vertex_coordinates.push_row(element_centroid.as_vector().iter())?;
                PolytopeData::Vertex(vertex_id)
            } else {
                let stride = |i| 3_usize.pow(i as _);
                let base: usize = position
                    .iter()
                    .enumerate()
                    .map(|(i, &x)| stride(i) * x)
                    .sum();
                let boundary_indexes = position
                    .iter()
                    .positions(|&x| x == 2)
                    .flat_map(|i| [base - stride(i), base - stride(i) * 2])
                    .collect_vec();

                let boundary = boundary_indexes.iter().map(|&i| elements[i]).collect();
                let hyperplane = if element_rank + 1 == ndim {
                    Some(
                        hyperplanes.push(
                            Hyperplane::from_pole(element_centroid.as_vector())
                                .ok_or_eyre("error constructing primordial hyperplane")?,
                        )?,
                    )
                } else {
                    None
                };
                PolytopeData::Polytope {
                    rank: element_rank,
                    boundary,
                    hyperplane,
                    is_primordial: element_rank + 1 == ndim,
                }
            };

            let new_id = polytopes.push(polytope_data)?;
            if element_rank == ndim {
                // We've constructed the whole cube!
                break PolytopeId(new_id.0);
            }
            elements.push(new_id);

            // Move to the next element position.
            for component in &mut position {
                *component += 1;
                if *component > 2 {
                    *component = 0;
                } else {
                    continue 'outer;
                }
            }
        };

        let vertex_data_to_id = ApproxHashMap::from_iter(
            APPROX,
            vertex_coordinates
                .iter()
                .map(|(id, data)| (Point::from(data), id)),
        );
        let polytope_data_to_id =
            HashMap::from_iter(polytopes.iter().map(|(id, data)| (data.clone(), id)));

        Ok(Self {
            id,

            ndim,

            primordial_cube,

            vertex_coordinates,
            polytopes,
            hyperplanes,

            vertex_data_to_id,
            polytope_data_to_id,

            cached_vertex_set: Mutex::new(HashMap::new()),
            cached_simplices: Mutex::new(HashMap::new()),
            cached_centroids: Mutex::new(HashMap::new()),
        })
    }

    /// Returns the primordial cube.
    pub fn primordial_cube(&self) -> PolytopeId {
        self.primordial_cube
    }

    /// Returns an error if `self` and `other` are different spaces.
    pub fn ensure_same_as(&self, other: &Self) -> Result<()> {
        if self.id != other.id {
            bail!("cannot operate between different spaces");
        }
        Ok(())
    }

    /// Returns the number of dimensions of the space.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns an element of the space by ID.
    pub fn get<I: Fits64>(&self, id: I) -> SpaceRef<'_, I> {
        SpaceRef { space: self, id }
    }

    /// Returns the position of a vertex.
    pub fn vertex_pos(&self, v: VertexId) -> Point {
        Point::from(&self.vertex_coordinates[v])
    }

    /// Returns the polytope ID for a vertex.
    ///
    /// Returns an error if the vertex is not added as a polytope.
    pub fn vertex_to_polytope(&self, v: VertexId) -> ElementId {
        // This should never panic because vertices are always added as
        // polytopes.
        self.polytope_data_to_id[&PolytopeData::Vertex(v)]
    }

    /// Memoizes a vertex.
    pub fn add_vertex(&mut self, p: Point) -> Result<(VertexId, ElementId), IndexOverflow> {
        let vertex_id = match self.vertex_data_to_id.entry(p) {
            approx_collections::hash_map::Entry::Occupied(e) => *e.get(),
            approx_collections::hash_map::Entry::Vacant(e) => {
                let coordinates = e.key().as_vector().iter();
                let vertex_id = self.vertex_coordinates.push_row(coordinates)?;
                let vertex_id = *e.insert(vertex_id);
                // Ensure that the vertex has a polytope ID as well.
                self.add_polytope(vertex_id.into())?;
                vertex_id
            }
        };
        let element_id = self.add_polytope(PolytopeData::Vertex(vertex_id))?;
        Ok((vertex_id, element_id))
    }
    /// Memoizes a polytope.
    pub fn add_polytope(&mut self, p: PolytopeData) -> Result<ElementId, IndexOverflow> {
        // Validate the boundary of the polytope.
        #[cfg(debug_assertions)]
        match &p {
            PolytopeData::Vertex(_) => (),
            PolytopeData::Polytope { rank, boundary, .. } => {
                for b in boundary.iter() {
                    let boundary_rank = self.get(b).rank();
                    assert_eq!(boundary_rank + 1, *rank, "bad boundary ranks of polytope");
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
    pub(super) fn add_polytope_if_non_degenerate(
        &mut self,
        p: PolytopeData,
    ) -> Result<Option<ElementId>, IndexOverflow> {
        if let PolytopeData::Polytope { rank, boundary, .. } = &p
            && boundary.len() <= *rank as usize
        {
            return Ok(None);
        }
        self.add_polytope(p).map(Some)
    }
    pub(super) fn add_subpolytope_if_non_degenerate(
        &mut self,
        original: ElementId,
        new_boundary: Set64<ElementId>,
    ) -> Result<Option<ElementId>> {
        let p = match &self.polytopes[original] {
            PolytopeData::Vertex(_) => bail!("expected non-vertex polytope; got vertex"),
            PolytopeData::Polytope {
                rank,
                boundary: _,
                hyperplane,
                is_primordial,
            } => PolytopeData::Polytope {
                rank: *rank,
                boundary: new_boundary,
                hyperplane: *hyperplane,
                is_primordial: *is_primordial,
            },
        };
        Ok(self.add_polytope_if_non_degenerate(p)?)
    }

    /// Returns the endpoints of a line, or an error if `line` is not a line.
    pub fn line_endpoints(&self, line: ElementId) -> Option<[VertexId; 2]> {
        let mut points = self.polytopes[line]
            .boundary()
            .ok()?
            .iter()
            .map(|p| self.polytopes[p].to_vertex());
        Some([points.next()??, points.next()??])
    }

    /// Returns the set of vertices of the polytope. This is exactly the vertex
    /// set of the convex hull.
    pub(super) fn vertex_set(&self, polytope: ElementId) -> Set64<VertexId> {
        if let Some(result) = self.cached_vertex_set.lock().get(&polytope) {
            return result.clone();
        }

        let result = match &self.polytopes[polytope] {
            PolytopeData::Vertex(p) => Set64::<VertexId>::from_iter([*p]),
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

    /// Returns a set of subelements of a polytope, of all ranks except points.
    pub(super) fn subelements_of(&self, root: ElementId) -> Set64<ElementId> {
        let mut ret = Set64::<ElementId>::new();
        let mut queue = vec![root];
        while let Some(p) = queue.pop() {
            if ret.insert(p)
                && let Ok(boundary) = self.polytopes[p].boundary()
            {
                queue.extend(boundary.iter());
            }
        }
        ret
    }

    /// Returns the set of all subelements of `root` with rank `rank`.
    pub(super) fn subelements_with_rank(&self, root: ElementId, rank: u8) -> Set64<ElementId> {
        if rank > self.polytopes[root].rank() {
            return Set64::new();
        }
        let mut ret = Set64::<ElementId>::from_iter([root]);
        while ret
            .iter()
            .next()
            .is_some_and(|p| self.polytopes[p].rank() > rank)
        {
            ret = ret
                .iter()
                // TODO: handle lines better?
                .filter_map(|p| self.polytopes[p].boundary().ok())
                .flat_map(|p| p.iter())
                .collect();
        }
        ret
    }

    /// Returns the set of greatest-rank subelements of a set of same-rank
    /// polytopes, if they have such a common set. This is a generalization of
    /// the notion of the infimum on a poset. If there is no such element, then
    /// the empty set is returned. Returns an error if the input polytopes do
    /// not have the same rank.
    pub fn greatest_common_subelements(
        &self,
        elements: Set64<ElementId>,
    ) -> Result<Set64<ElementId>> {
        let mut rank = elements
            .iter()
            .map(|p| self.polytopes[p].rank())
            .all_equal_value()
            .map_err(|_| eyre!("polytopes have different ranks"))?;

        let mut subelement_sets = elements
            .iter()
            .map(|p| Set64::<ElementId>::from_iter([p]))
            .collect_vec();
        while rank > 0 {
            if subelement_sets.is_empty() {
                return Ok(Set64::<ElementId>::new());
            }

            let intersection = intersect_list_of_sets(subelement_sets.clone());
            if !intersection.is_empty() {
                return Ok(intersection);
            }

            subelement_sets = subelement_sets
                .iter()
                .map(|set| {
                    set.iter()
                        .filter_map(|e| Some(self.polytopes[e].boundary().ok()?.iter()))
                        .flatten()
                        .collect()
                })
                .collect();

            rank -= 1;
        }

        // no intersection
        Ok(Set64::<ElementId>::new())
    }

    /// Returns a human-readable string representation of a polytope element.
    pub fn dump_to_string(&self, root: ElementId) -> String {
        let polytopes = &self.polytopes;
        let vertices = &self.vertex_coordinates;

        let max_rank = polytopes[root].rank();
        let mut s = String::new();
        let mut stack = vec![root];
        while let Some(p) = stack.pop() {
            for _ in polytopes[p].rank()..max_rank {
                s += "  ";
            }

            if polytopes[p].rank() == 1 {
                // IIFE to mimic try_block
                let edge_verts = (|| {
                    let mut points = polytopes[p]
                        .boundary()
                        .ok()?
                        .iter()
                        .map(|p| polytopes[p].to_vertex());
                    Some([points.next()??, points.next()??])
                })();
                s += &match edge_verts {
                    Some([a, b]) => format!("{p}: line {:?} .. {:?}", &vertices[a], &vertices[b]),
                    None => format!("{p}: invalid edge"),
                }
            } else {
                match &polytopes[p] {
                    PolytopeData::Vertex(v) => s += &format!("{p}: point {v}"),
                    PolytopeData::Polytope {
                        rank,
                        boundary,
                        hyperplane,
                        is_primordial,
                    } => {
                        s += &format!("{p}: {rank}D polytope");
                        if *is_primordial {
                            s += " (primordial)";
                        }
                        if let Some(h) = hyperplane {
                            s += &format!(" (hyperplane {h})");
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

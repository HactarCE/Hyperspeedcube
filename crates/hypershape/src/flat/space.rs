use super::*;

/// Patch of Euclidean (i.e., flat) space in which polytopes can be constructed.
pub struct Space {
    /// Number of dimensions of the space.
    ndim: u8,

    /// Reference-counted pointer to this struct.
    this: Weak<Self>,

    /// Primordial cube.
    pub(super) primordial_cube: Mutex<Option<PolytopeId>>,

    // TODO: consider using `scc`
    pub(super) vertices: Mutex<PerVertex<Vector>>,
    pub(super) vertex_data_to_id: Mutex<ApproxHashMap<Vector, VertexId>>,

    pub(super) polytopes: Mutex<PerElement<PolytopeData>>,
    pub(super) polytope_data_to_id: Mutex<HashMap<PolytopeData, ElementId>>,

    pub(super) cached_hyperplane_of_facet: Mutex<HashMap<ElementId, Hyperplane>>,
    pub(super) cached_vertex_set: Mutex<HashMap<ElementId, Set64<VertexId>>>,

    pub(super) cached_which_side_has_polytope:
        Mutex<ApproxHashMap<Hyperplane, HashMap<ElementId, WhichSide>>>,

    /// Decomposition of each polytope element into simplices.
    pub(super) cached_simplices: Mutex<HashMap<ElementId, SimplexBlob>>,
    /// Centroid of each polytope element.
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
    /// Constructs a new space containing no polytopes.
    ///
    /// # Panics
    ///
    /// Panics if `ndim > 7`.
    pub fn new(ndim: u8) -> Arc<Self> {
        assert!(ndim >= 1, "ndim={ndim} is below min value of 1");
        assert!(ndim <= 7, "ndim={ndim} exceeds max value of 7");
        Arc::new_cyclic(|this| Self {
            ndim,
            this: this.clone(),

            primordial_cube: Mutex::new(None),

            vertices: Mutex::new(PerVertex::new()),
            vertex_data_to_id: Mutex::new(ApproxHashMap::new()),

            polytopes: Mutex::new(PerElement::new()),
            polytope_data_to_id: Mutex::new(HashMap::new()),

            cached_hyperplane_of_facet: Mutex::new(HashMap::new()),
            cached_vertex_set: Mutex::new(HashMap::new()),

            cached_which_side_has_polytope: Mutex::new(ApproxHashMap::new()),

            cached_simplices: Mutex::new(HashMap::new()),
            cached_centroids: Mutex::new(HashMap::new()),
        })
    }

    /// Returns an `Arc` reference to the space.
    pub fn arc(&self) -> Arc<Self> {
        self.this.upgrade().expect("`Space` removed from `Arc`")
    }
    /// Returns an error if `self` and `other` are different spaces.
    pub fn ensure_same_as(&self, other: &Self) -> Result<()> {
        if !Weak::ptr_eq(&self.this, &other.this) {
            bail!("cannot operate between different spaces");
        }
        Ok(())
    }
    /// Returns an error if `self` and `other` are the same space.
    pub fn ensure_not_same_as(&self, other: &Self) -> Result<()> {
        if Weak::ptr_eq(&self.this, &other.this) {
            bail!("expected different spaces but got the same space");
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

    /// Returns the polytope ID for a vertex.
    pub fn vertex_to_polytope(&self, v: VertexId) -> ElementId {
        self.add_polytope(v.into())
            .expect("TODO: handle vertex_to_polytope() error better")
    }

    /// Memoizes a vertex.
    pub fn add_vertex(&self, v: Vector) -> Result<VertexId, IndexOverflow> {
        let cache = &mut self.vertex_data_to_id.lock();
        match cache.entry(v.clone()) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                let vertex_id = *e.insert(self.vertices.lock().push(v)?);
                // Ensure that the vertex has a polytope ID as well.
                self.add_polytope(vertex_id.into())?;
                Ok(vertex_id)
            }
        }
    }
    /// Memoizes a line.
    pub fn add_line(&self, points: [VertexId; 2]) -> Result<ElementId, IndexOverflow> {
        let [a, b] = points;
        let a = self.add_polytope(a.into())?;
        let b = self.add_polytope(b.into())?;
        self.add_polytope(PolytopeData::Polytope {
            rank: 1,
            boundary: Set64::from_iter([a, b]),

            is_primordial: false,
            seam: None,

            patch: None,
        })
    }
    /// Memoizes a polytope.
    pub fn add_polytope(&self, p: PolytopeData) -> Result<ElementId, IndexOverflow> {
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

        match self.polytope_data_to_id.lock().entry(p.clone()) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => Ok(*e.insert(self.polytopes.lock().push(p)?)),
        }
    }
    pub(super) fn add_polytope_if_non_degenerate(
        &self,
        p: PolytopeData,
    ) -> Result<Option<ElementId>, IndexOverflow> {
        if let PolytopeData::Polytope { rank, boundary, .. } = &p {
            if boundary.len() <= *rank as usize {
                return Ok(None);
            }
        }
        self.add_polytope(p).map(Some)
    }
    pub(super) fn add_subpolytope_if_non_degenerate(
        &self,
        original: ElementId,
        new_boundary: Set64<ElementId>,
    ) -> Result<Option<ElementId>> {
        let p = match &self.polytopes.lock()[original] {
            PolytopeData::Vertex(_) => bail!("expected polytope; got vertex"),
            PolytopeData::Polytope {
                rank,
                boundary: _,
                is_primordial,
                seam,
                patch,
            } => PolytopeData::Polytope {
                rank: *rank,
                boundary: new_boundary,
                is_primordial: *is_primordial,
                seam: *seam,
                patch: *patch,
            },
        };
        let new_id = self.add_polytope_if_non_degenerate(p)?;

        if let Some(new) = new_id {
            if self.polytopes.lock()[new].rank() + 1 == self.ndim {
                // Intersection is a new facet! Copy the facet hyperplane.
                let mut cache = self.cached_hyperplane_of_facet.lock();
                if let Some(plane) = cache.get(&original).cloned() {
                    cache.insert(new, plane);
                }
            }
        }

        Ok(new_id)
    }

    /// Returns the endpoints of a line, or an error if `line` is not a line.
    pub fn line_endpoints(&self, line: ElementId) -> Option<[VertexId; 2]> {
        let polytopes = self.polytopes.lock();
        let mut points = polytopes[line]
            .boundary()
            .ok()?
            .iter()
            .map(|p| polytopes[p].to_vertex());
        Some([points.next()??, points.next()??])
    }

    /// Returns the set of vertices of the polytope. This is exactly the vertex
    /// set of the convex hull.
    pub(super) fn vertex_set(&self, polytope: ElementId) -> Set64<VertexId> {
        if let Some(result) = self.cached_vertex_set.lock().get(&polytope) {
            return result.clone();
        }

        let polytopes = self.polytopes.lock();
        let result = match polytopes[polytope].clone() {
            PolytopeData::Vertex(p) => Set64::<VertexId>::from_iter([p]),
            PolytopeData::Polytope { boundary, .. } => {
                drop(polytopes);
                let boundary = boundary.clone();
                boundary.iter().flat_map(|b| self.vertex_set(b)).collect()
            }
        };

        self.cached_vertex_set
            .lock()
            .insert(polytope, result.clone());
        result
    }

    /// Returns the hyperplane in which a facet lives.
    pub fn hyperplane_of_facet(&self, facet: FacetId) -> Result<Hyperplane> {
        self.cached_hyperplane_of_facet
            .lock()
            .get(&ElementId(facet.0))
            .cloned()
            .ok_or_eyre("missing hyperplane for facet")
    }

    /// Returns a set of subelements of a polytope, of all ranks except points.
    pub(super) fn subelements_of(&self, root: ElementId) -> Set64<ElementId> {
        let mut ret = Set64::<ElementId>::new();
        let mut queue = vec![root];
        while let Some(p) = queue.pop() {
            if ret.insert(p) {
                if let Ok(boundary) = self.polytopes.lock()[p].boundary() {
                    queue.extend(boundary.iter());
                }
            }
        }
        ret
    }

    /// Returns the set of all subelements of `root` with rank `rank`.
    pub(super) fn subelements_with_rank(&self, root: ElementId, rank: u8) -> Set64<ElementId> {
        let polytopes = self.polytopes.lock();
        if rank > polytopes[root].rank() {
            return Set64::new();
        }
        let mut ret = Set64::<ElementId>::from_iter([root]);
        while ret
            .iter()
            .next()
            .is_some_and(|p| polytopes[p].rank() > rank)
        {
            ret = ret
                .iter()
                // TODO: handle lines better?
                .filter_map(|p| polytopes[p].boundary().ok())
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
        let polytopes = self.polytopes.lock();

        let mut rank = elements
            .iter()
            .map(|p| polytopes[p].rank())
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
                        .filter_map(|e| Some(polytopes[e].boundary().ok()?.iter()))
                        .flatten()
                        .collect()
                })
                .collect();

            rank -= 1;
        }

        // no intersection
        Ok(Set64::<ElementId>::new())
    }

    /// Returns which side of `divider` contains `element`. The result is
    /// cached.
    pub fn which_side_has_polytope(&self, divider: &Hyperplane, element: ElementId) -> WhichSide {
        let vertex_set = self.vertex_set(element);
        *self
            .cached_which_side_has_polytope
            .lock()
            .entry(divider.clone())
            .or_default()
            .entry(element)
            .or_insert_with(|| {
                WhichSide::from_points(
                    vertex_set
                        .iter()
                        .map(|v| divider.location_of_point(&self.vertices.lock()[v])),
                )
            })
    }

    /// Returns a human-readable string representation of a polytope element.
    pub fn dump_to_string(&self, root: ElementId) -> String {
        let polytopes = self.polytopes.lock();
        let vertices = self.vertices.lock();

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
                    Some([a, b]) => format!("{p}: line {} .. {}", vertices[a], vertices[b]),
                    None => format!("{p}: invalid edge"),
                }
            } else {
                match &polytopes[p] {
                    PolytopeData::Vertex(v) => s += &format!("{p}: point {v}"),
                    PolytopeData::Polytope {
                        rank,
                        boundary,

                        is_primordial,

                        seam,
                        patch,
                    } => {
                        s += &format!("{p}: {rank}D polytope");
                        if *is_primordial {
                            s += " (primordial)";
                        }
                        if let Some(seam) = seam {
                            s += &format!(" (seam {seam})");
                        }
                        if let Some(patch) = patch {
                            s += &format!(" (patch {patch})");
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

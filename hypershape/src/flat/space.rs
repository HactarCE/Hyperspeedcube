use super::*;

/// Patch of Euclidean (i.e., flat) space in which polytopes can be constructed.
pub struct Space {
    /// Number of dimensions of the space.
    ndim: u8,

    /// Reference-counted pointer to this struct.
    this: Weak<Self>,

    // TODO: consider using `boxcar` for vectors and `dashmap` for hashmaps
    pub(super) vertices: Mutex<PerVertex<Vector>>,
    pub(super) vertex_data_to_id: Mutex<ApproxHashMap<Vector, VertexId>>,

    pub(super) polytopes: Mutex<PerElement<PolytopeData>>,
    pub(super) polytope_data_to_id: Mutex<HashMap<PolytopeData, ElementId>>,

    pub(super) cached_subspaces: Mutex<HashMap<ElementId, (Vec<Vector>, pga::Blade)>>,
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

            vertices: Mutex::new(PerVertex::new()),
            vertex_data_to_id: Mutex::new(ApproxHashMap::new()),

            polytopes: Mutex::new(PerElement::new()),
            polytope_data_to_id: Mutex::new(HashMap::new()),

            cached_subspaces: Mutex::new(HashMap::new()),
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
    pub fn add_polytope(&self, mut p: PolytopeData) -> Result<ElementId, IndexOverflow> {
        // Validate the boundary of the polytope.
        #[cfg(debug_assertions)]
        match &mut p {
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
        Ok(self.add_polytope_if_non_degenerate(p)?)
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

    /// Returns the hyperplane in which a facet lives. Returns an error if the
    /// polytope is not a facet (rank `ndim-1`).
    pub fn hyperplane_of_facet(&self, facet: ElementId) -> Result<Hyperplane> {
        let expected_rank = self.ndim - 1;
        let actual_rank = self.polytopes.lock()[facet].rank();
        if expected_rank != actual_rank {
            bail!("expected polytope with rank {expected_rank}; got {actual_rank}");
        }
        self.blade_for_subspace_of_polytope(facet)?
            .to_hyperplane()
            .ok_or_eyre("error converting polytope subspace to hyperplane")
    }

    /// Returns a basis for the smallest subspace in which the polytope lives.
    /// The result is cached.
    pub fn basis_for_subspace_of_polytope(&self, polytope: ElementId) -> Result<Vec<Vector>> {
        let (basis, _blade) = self.basis_and_blade_for_subspace_of_polytope(polytope)?;
        Ok(basis)
    }
    /// Returns a PGA blade representing the smallest subspace in which a
    /// polytope lives. The result is cached.
    pub fn blade_for_subspace_of_polytope(&self, polytope: ElementId) -> Result<pga::Blade> {
        let (_basis, blade) = self.basis_and_blade_for_subspace_of_polytope(polytope)?;
        Ok(blade)
    }
    // TODO: this seems broken; remove it
    fn basis_and_blade_for_subspace_of_polytope(
        &self,
        polytope: ElementId,
    ) -> Result<(Vec<Vector>, pga::Blade)> {
        let ndim = self.ndim;

        if let Some(result) = self.cached_subspaces.lock().get(&polytope) {
            return Ok(result.clone());
        }

        let polytopes = self.polytopes.lock();
        let result = match polytopes[polytope].clone() {
            PolytopeData::Vertex(p) => (
                vec![],
                pga::Blade::from_point(ndim, &self.vertices.lock()[p]),
            ),
            PolytopeData::Polytope { boundary, .. } => {
                drop(polytopes);
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
                        let new_point = pga::Blade::from_point(ndim, &self.vertices.lock()[v]);
                        let new_blade = pga::Blade::wedge(&old_blade, &new_point)?;
                        Some((v, new_point, new_blade))
                    })
                    .max_by_key(|(_, _, b)| FloatOrd(b.mag2()))
                    .ok_or_eyre("degenerate polytope")?;
                let new_point_projected = new_point
                    .orthogonal_projection_to(&old_blade)
                    .and_then(|b| b.to_vector())
                    .ok_or_eyre("degenerate polytope")?;
                let new_vector = &self.vertices.lock()[new_vertex] - new_point_projected;
                basis.push(new_vector.normalize().ok_or_eyre("degenerate polytope")?);
                (basis, new_blade)
            }
        };

        self.cached_subspaces
            .lock()
            .insert(polytope, result.clone());
        Ok(result)
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
        let max_rank = self.polytopes.lock()[root].rank();
        let mut s = String::new();
        let mut stack = vec![root];
        while let Some(p) = stack.pop() {
            for _ in self.polytopes.lock()[p].rank()..max_rank {
                s += "  ";
            }

            if let Some([a, b]) = self.line_endpoints(p) {
                s += &format!(
                    "{p}: line {} .. {}",
                    self.vertices.lock()[a],
                    self.vertices.lock()[b]
                )
            } else {
                match &self.polytopes.lock()[p] {
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
                            s += &format!(" (seam {seam})")
                        }
                        if let Some(patch) = patch {
                            s += &format!(" (patch {patch})")
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

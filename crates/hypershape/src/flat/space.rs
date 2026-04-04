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
    pub(super) portals: TiVec<PortalId, PortalData>,

    pub(super) vertex_data_to_id: ApproxHashMap<Point, VertexId>,
    pub(super) polytope_data_to_id: HashMap<PolytopeData, ElementId>,
    pub(super) hyperplane_data_to_id: ApproxHashMap<Hyperplane, HyperplaneId>,

    /// Cached results for [`Self::vertex_set()`].
    pub(super) cached_vertex_set: Mutex<HashMap<ElementId, Set64<VertexId>>>,
    /// Cached results for [`Self::simplices()`].
    pub(super) cached_simplices: Mutex<HashMap<ElementId, SimplexBlob>>,
    /// Cached results for [`Self::centroid()`].
    pub(super) cached_centroids: Mutex<HashMap<ElementId, Centroid>>,
    /// Cached results for [`Self::unfold()`].
    cached_unfolded: HashMap<ElementId, ElementId>,
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
                PolytopeData::from(vertex_id)
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
                    boundary_portals: BoundaryPortals::EMPTY,
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
            portals: TiVec::new(),

            vertex_data_to_id,
            polytope_data_to_id,
            hyperplane_data_to_id: ApproxHashMap::new(APPROX),

            cached_vertex_set: Mutex::new(HashMap::new()),
            cached_simplices: Mutex::new(HashMap::new()),
            cached_centroids: Mutex::new(HashMap::new()),
            cached_unfolded: HashMap::new(),
        })
    }

    /// Returns the primordial cube.
    pub fn primordial_cube(&self) -> PolytopeId {
        self.primordial_cube
    }

    /// Adds a folded shape with the given mirror planes and carve planes.
    ///
    /// The shape can be sliced as normal using [`Cut`] and can be unfolded
    /// using [`Self::unfold()`].
    pub fn add_folded_shape(
        &mut self,
        mirror_planes: impl IntoIterator<Item = Hyperplane>,
        carve_planes: impl IntoIterator<Item = Hyperplane>,
    ) -> Result<PolytopeId> {
        let mut ret: ElementId = self.primordial_cube().into();

        let mirror_planes = mirror_planes.into_iter().collect_vec();
        let carve_planes = carve_planes
            .into_iter()
            .flat_map(|init| {
                let mut seen = ApproxHashMap::new(APPROX);
                seen.insert(init.clone(), ());
                let mut orbit = vec![init];
                let mut next_unprocessed_index = 0;
                while next_unprocessed_index < orbit.len() {
                    for m in &mirror_planes {
                        let new_plane = m.reflect_hyperplane(&orbit[next_unprocessed_index]);
                        if seen.insert(new_plane.clone(), ()).is_none() {
                            orbit.push(new_plane);
                        }
                    }
                    next_unprocessed_index += 1;
                }
                orbit
            })
            .collect_vec();

        // IIFE to mimic try_block
        for mirror_plane in mirror_planes {
            ret = Cut::carve_portal(mirror_plane)
                .cut(self, ret)?
                .outside()
                .ok_or_eyre("fundamental region is empty")?;
        }

        for carve_plane in carve_planes {
            ret = Cut::carve(carve_plane)
                .cut(self, ret)?
                .inside()
                .ok_or_eyre("geometry is empty")?;
        }

        let ret = self.get(ret).as_polytope()?;

        if ret.has_primordial_facet() {
            bail!("primordial cube exists in final shape");
        }

        Ok(ret.id())
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
    ///
    /// Panics if the polytope is not a vertex.
    pub fn vertex_pos(&self, v: impl ElementIdConvert) -> Point {
        Point::from(&self.vertex_coordinates[v.to_vertex_id(self).expect("not a vertex")])
    }

    /// Returns the polytope ID for a vertex.
    ///
    /// Returns an error if the vertex is not added as a polytope.
    pub fn vertex_to_polytope(&self, v: VertexId) -> ElementId {
        // This should never panic because vertices are always added as
        // polytopes.
        self.polytope_data_to_id[&PolytopeData::from(v)]
    }

    /// Memoizes a hyperplane.
    pub(super) fn add_hyperplane(&mut self, h: Hyperplane) -> Result<HyperplaneId> {
        // We could canonicalize the hyperplane's orientation to avoid storing
        // duplicates but it doesn't really matter.
        match self.hyperplane_data_to_id.entry(h) {
            approx_collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
            approx_collections::hash_map::Entry::Vacant(e) => {
                let id = self.hyperplanes.push(e.key().clone())?;
                Ok(*e.insert(id))
            }
        }
    }
    /// Memoizes a vertex.
    pub(super) fn add_vertex(&mut self, p: Point) -> Result<(VertexId, ElementId)> {
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
        let element_id = self.add_polytope(PolytopeData::from(vertex_id))?;
        Ok((vertex_id, element_id))
    }
    /// Memoizes a polytope.
    pub(super) fn add_polytope(&mut self, p: PolytopeData) -> Result<ElementId> {
        // Validate the boundary of the polytope.
        #[cfg(debug_assertions)]
        match &p {
            PolytopeData::Vertex(_) => (),
            PolytopeData::Polytope { rank, boundary, .. } => {
                for b in boundary.iter() {
                    let boundary_rank = self.get(b).rank();
                    ensure!(boundary_rank + 1 == *rank, "bad boundary ranks of polytope");
                }
                if *rank == 1 {
                    ensure!(boundary.len() == 2, "line must have two endpoints");
                    ensure!(
                        !boundary.iter().all_equal(),
                        "line endpoints must be distinct",
                    );
                }
                if *rank == 2 {
                    let mut multiplicity = HashMap::<VertexId, usize>::new();
                    for b in boundary.iter() {
                        for v in self.line_endpoints(b).ok_or_eyre("expected line")? {
                            *multiplicity.entry(v).or_default() += 1;
                        }
                    }
                    for &m in multiplicity.values() {
                        ensure!(m == 2, "bad polygon structure");
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
    ) -> Result<Option<ElementId>> {
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
        new_boundary: impl IntoIterator<Item = ElementId>,
        new_boundary_portals: impl IntoIterator<Item = (PortalId, ElementId)>,
    ) -> Result<Option<ElementId>> {
        let p = match &self.polytopes[original] {
            PolytopeData::Vertex(_) => bail!("expected non-vertex polytope; got vertex"),
            PolytopeData::Polytope {
                rank,
                boundary: _,
                boundary_portals: _,
                hyperplane,
                is_primordial,
            } => PolytopeData::Polytope {
                rank: *rank,
                boundary_portals: BoundaryPortals::new(new_boundary_portals),
                boundary: new_boundary.into_iter().collect(),
                hyperplane: *hyperplane,
                is_primordial: *is_primordial,
            },
        };
        self.add_polytope_if_non_degenerate(p)
    }

    /// Applies a portal's transfomr to a polytope.
    fn send_polytope_through_portal(
        &mut self,
        polytope: ElementId,
        portal: PortalId,
    ) -> Result<ElementId> {
        let portal_plane = &self.hyperplanes[self.portals[portal].hyperplane];
        match self.polytopes[polytope].clone() {
            PolytopeData::Vertex(v) => {
                let (_vertex_id, element_id) =
                    self.add_vertex(portal_plane.reflect_point(&self.vertex_pos(v)))?;
                Ok(element_id)
            }

            PolytopeData::Polytope {
                rank,
                boundary,
                boundary_portals,
                hyperplane,
                is_primordial,
            } => {
                ensure!(
                    boundary_portals.is_empty(),
                    "cannot send polytope through portal when polytope has nonempty boundary portals",
                );
                let portal_plane = portal_plane.clone();
                let new_polytope_data = PolytopeData::Polytope {
                    rank,
                    boundary: boundary
                        .iter()
                        .map(|b| self.send_polytope_through_portal(b, portal))
                        .try_collect()?,
                    boundary_portals: BoundaryPortals::EMPTY,
                    hyperplane: match hyperplane {
                        Some(h) => Some(self.add_hyperplane(
                            portal_plane.reflect_hyperplane(&self.hyperplanes[h]),
                        )?),
                        None => None,
                    },
                    is_primordial,
                };
                Ok(self.add_polytope(new_polytope_data)?)
            }
        }
    }

    /// Returns the endpoints of a line, or an error if `line` is not a line.
    pub(super) fn line_endpoints(&self, line: ElementId) -> Option<[VertexId; 2]> {
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
            PolytopeData::Vertex(v) => Set64::<VertexId>::from_iter([*v]),
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

    /// Unfolds a polytope across portals, and returns the set of portals that
    /// the polytope crosses through.
    ///
    /// Returns `None` if the polytope only exists because of portals. This
    /// should never happen for a max-rank polytope.
    pub fn unfold(&mut self, polytope: ElementId) -> Result<ElementId> {
        if let Some(cached_result) = self.cached_unfolded.get(&polytope) {
            return Ok(*cached_result);
        }

        match self.polytopes[polytope].clone() {
            PolytopeData::Vertex { .. } => Ok(polytope),

            PolytopeData::Polytope {
                rank,
                boundary,
                boundary_portals,
                hyperplane,
                is_primordial,
            } => {
                if boundary
                    .iter()
                    .any(|b| self.get(b).as_facet().is_ok_and(|f| f.is_primordial()))
                {
                    bail!(
                        "polytope is too big/infinite (cannot unfold shape with primordial facets)",
                    )
                }

                let mut unprocessed_boundary_elements: VecDeque<ElementId> = boundary
                    .iter()
                    .filter(|b| !boundary_portals.contains_element(*b))
                    .map(|b| self.unfold(b))
                    .try_collect()?;

                // Orbit the non-portal boundary elements using the bounding portals
                // elements as the generating set.

                let generator_portals = boundary_portals.iter_portals().collect_vec();
                let mut new_boundary_elements_set: Set64<ElementId> =
                    unprocessed_boundary_elements.iter().copied().collect();
                while let Some(elem) = unprocessed_boundary_elements.pop_front() {
                    for &g in &generator_portals {
                        let new_elem = self.send_polytope_through_portal(elem, g)?;
                        if new_boundary_elements_set.insert(new_elem) {
                            unprocessed_boundary_elements.push_back(new_elem);
                        }
                    }
                }

                let unfolded_polytope = self.add_polytope(PolytopeData::Polytope {
                    rank,
                    boundary: new_boundary_elements_set,
                    boundary_portals: BoundaryPortals::EMPTY,
                    hyperplane,
                    is_primordial,
                })?;
                self.cached_unfolded.insert(polytope, unfolded_polytope);
                Ok(unfolded_polytope)
            }
        }
    }

    /// Returns the transform to apply when passing through a portal.
    pub fn portal_transform(&self, portal_id: PortalId) -> Result<pga::Motor> {
        let h = self.portals.get(portal_id)?.hyperplane;
        pga::Motor::plane_reflection(self.ndim, &self.hyperplanes[h]).ok_or_eyre("bad hyperplane")
    }

    /// Returns a human-readable string representation of a polytope element.
    pub fn dump_to_string(&self, root: ElementId) -> String {
        let polytopes = &self.polytopes;
        let vertices = &self.vertex_coordinates;

        let max_rank = polytopes[root].rank();
        let mut s = String::new();
        let mut stack = vec![(root, String::new())];
        while let Some((p, p_portals_str)) = stack.pop() {
            for _ in polytopes[p].rank()..max_rank {
                s += "  ";
            }
            match &polytopes[p] {
                PolytopeData::Vertex(v) => {
                    s += &format!("{p}: point {v} {:.4?}", self.vertex_coordinates.get(*v));
                }
                PolytopeData::Polytope {
                    rank,
                    boundary,
                    boundary_portals,
                    hyperplane,
                    is_primordial,
                } => {
                    if *rank == 1 {
                        if let Some([a, b]) = boundary.iter().collect_array()
                            && let Some(va) = polytopes[a].to_vertex()
                            && let Some(vb) = polytopes[b].to_vertex()
                        {
                            s += &format!(
                                "{p}: line{p_portals_str} <{:.4?}{}> .. <{:.4?}{}>",
                                &vertices[va],
                                format_portal_list(boundary_portals.portals_for_element(a)),
                                &vertices[vb],
                                format_portal_list(boundary_portals.portals_for_element(b)),
                            );
                        } else {
                            s += &format!("{p}: invalid edge");
                        }
                    } else {
                        s += &format!("{p}: {rank}D polytope{p_portals_str}");
                        if *is_primordial {
                            s += " (primordial)";
                        }
                        if let Some(h) = hyperplane {
                            s += &format!(" (hyperplane {h})");
                        }
                        stack.extend(boundary.iter().map(|b| {
                            let portals_str =
                                format_portal_list(boundary_portals.portals_for_element(b));
                            (b, portals_str)
                        }));
                    }
                }
            }
            s.push('\n');
        }
        s
    }
}

fn format_portal_list(portals: impl Iterator<Item = PortalId>) -> String {
    let portals = portals.collect_vec();
    if portals.is_empty() {
        String::new()
    } else {
        format!(" (portals [{}])", portals.iter().join(", "))
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

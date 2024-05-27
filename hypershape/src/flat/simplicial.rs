use super::*;

impl Space {
    /// Returns the combined centroid of a set of polytopes, or `None` if the
    /// combined weight is zero. This is only meaningful if all polytopes have
    /// the same rank.
    pub fn combined_centroid(
        &self,
        polytopes: impl IntoIterator<Item = ElementId>,
    ) -> Result<Option<Centroid>> {
        let mut ret = Centroid::ZERO;
        for p in polytopes {
            ret += self.centroid(p)?;
        }
        Ok((!ret.is_zero()).then_some(ret))
    }

    /// Returns the centroid of a polytope element.
    pub(super) fn centroid(&self, element: ElementId) -> Result<Centroid> {
        if let Some(result) = self.cached_centroids.lock().get(&element) {
            return Ok(result.clone());
        }

        let mut sum = Centroid::ZERO;
        for simplex in self.simplices(element)? {
            // The center of a simplex is the average of its vertices.
            let verts = simplex.vertices(self);
            let center = verts.iter().map(|v| v.pos()).sum::<Vector>() / verts.len() as Float;

            // Orthogonalize the vectors spanning the simplex.
            let mut verts_iter = verts.iter().map(|v| v.pos());
            let initial_vertex = verts_iter.next().ok_or_eyre("simplex is empty")?;
            let mut vectors = verts_iter.map(|v| v - &initial_vertex).collect_vec();
            for i in 1..vectors.len() {
                for j in 0..i {
                    let Some(rejected) = vectors[i].rejected_from(&vectors[j]) else {
                        return Ok(Centroid::ZERO);
                    };
                    vectors[i] = rejected;
                }
            }
            // This is scaled by some factor depending on the number of
            // dimensions but that's fine.
            let weight = vectors
                .into_iter()
                .map(|v| v.mag2())
                .product::<Float>()
                .sqrt();

            sum += Centroid::new(&center, weight);
        }
        Ok(sum)
    }

    /// Returns a simplicial complex representing a polytope element.
    pub(super) fn simplices<'a>(&self, element: ElementId) -> Result<SimplexBlob> {
        if let Some(result) = self.cached_simplices.lock().get(&element) {
            return Ok(result.clone());
        }

        let element = self.get(element);

        let result = if let Ok(v) = element.as_vertex() {
            Simplex::new([v.id]).into()
        } else {
            if element.as_facet().is_ok_and(|f| f.is_primordial()) {
                bail!(
                    "primordial cube is present in final shape! \
                     your shape may be infinite",
                );
            }
            SimplexBlob::from_convex_hull(
                &element
                    .boundary()
                    .map(|b| self.simplices(b.id))
                    .collect::<Result<Vec<_>>>()?,
            )?
        };

        self.cached_simplices
            .lock()
            .insert(element.id, result.clone());
        Ok(result)
    }

    /// Returns a triangulation of all the polygons in `element`.
    pub(super) fn triangles(&self, element: ElementId) -> Result<Vec<[VertexId; 3]>> {
        let polygons = self.subelements_with_rank(element, 2);
        let simplexes: Vec<SimplexBlob> = polygons
            .iter()
            .map(|polygon| self.simplices(polygon))
            .try_collect()?;
        Ok(simplexes
            .into_iter()
            .flatten()
            .filter_map(|simplex| simplex.try_into_array())
            .collect())
    }
}

/// Simplexes comprising a convex polytope.
#[derive(Debug, Default, Clone)]
pub struct SimplexBlob(SmallVec<[Simplex; 2]>);
impl fmt::Display for SimplexBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blob[{}]", self.0.iter().join(", "))
    }
}
impl From<Simplex> for SimplexBlob {
    fn from(s: Simplex) -> Self {
        SimplexBlob::new([s])
    }
}
impl IntoIterator for SimplexBlob {
    type Item = Simplex;

    type IntoIter = smallvec::IntoIter<[Simplex; 2]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl SimplexBlob {
    /// Empty simplex blob.
    pub const EMPTY: Self = SimplexBlob(SmallVec::new_const());

    /// Constructs a `SimplexBlob` from a list of simplices.
    pub fn new(simplices: impl IntoIterator<Item = Simplex>) -> Self {
        SimplexBlob(simplices.into_iter().collect())
    }

    /// Constructs a higher-dimensional simplex blob from a list of facets. No
    /// validation is done on the facets.
    pub fn from_convex_hull(facets: &[SimplexBlob]) -> Result<Self> {
        let Some(arbitrary_facet) = facets.iter().find_map(|f| f.0.first()) else {
            return Ok(SimplexBlob::EMPTY);
        };
        let facet_ndim = arbitrary_facet.ndim()?;

        ensure!(
            facets
                .iter()
                .flat_map(|f| &f.0)
                .all(|s| s.ndim().ok() == Some(facet_ndim)),
            "cannot construct simplex blob from \
             dimension-mismatched convex hull",
        );

        let facet_simplices = facets.iter().flat_map(|f| &f.0);
        let vertex_set: Set64<VertexId> = facet_simplices.flat_map(|s| s.0.iter()).collect();

        // Optimization: if the number of simplices equals the facet dimension
        // plus 2 equals the nubmer of vertices, then the result is a single
        // simplex.
        let number_of_simplices = facets.iter().map(|f| f.0.len()).sum::<usize>();
        let is_single_simplex = number_of_simplices == facet_ndim as usize + 2
            && number_of_simplices == vertex_set.len();
        if is_single_simplex {
            // Construct the single simplex.
            Ok(SimplexBlob::new([Simplex(vertex_set)]))
        } else {
            // Pick a vertex to start from.
            let initial_vertex = arbitrary_facet.arbitrary_vertex()?;
            Ok(SimplexBlob::from_convex_hull_and_initial_vertex(
                facets,
                initial_vertex,
            ))
        }
    }

    fn from_convex_hull_and_initial_vertex(
        facets: &[SimplexBlob],
        initial_vertex: VertexId,
    ) -> Self {
        let mut ret = smallvec![];

        // For every facet that does not contain that vertex ...
        for facet in facets {
            if facet.0.iter().all(|s| !s.0.contains(initial_vertex)) {
                // ... for every simplex in that facet ...
                for simplex in &facet.0 {
                    // ... construct a new simplex that will be in the result.
                    let mut simplex = simplex.clone();
                    simplex.0.insert(initial_vertex);
                    // And add that simplex, if it's not a duplicate.
                    if !ret.contains(&simplex) {
                        ret.push(simplex);
                    }
                }
            }
        }

        SimplexBlob(ret)
    }

    /// Merges another blob of simplexes into this one.
    pub fn extend(&mut self, other: SimplexBlob) {
        self.0.extend(other.0);
    }

    /// Iterates over simplices in the blob.
    pub fn iter(&self) -> impl '_ + Iterator<Item = &Simplex> {
        self.0.iter()
    }
}

/// Simplex represented as a set of vertices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simplex(pub Set64<VertexId>);
impl fmt::Display for Simplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Simplex({})", self.0.iter().join(", "))
    }
}
impl Simplex {
    /// Constructs a simplex from vertices.
    pub fn new(verts: impl IntoIterator<Item = VertexId>) -> Self {
        Simplex(verts.into_iter().collect())
    }

    /// Returns the number of dimensions of the simplex, which is inferred from
    /// the number of vertices.
    pub fn ndim(&self) -> Result<u8> {
        (self.0.len() as u8)
            .checked_sub(1)
            .ok_or_eyre("simplex cannot be empty")
    }
    /// Returns the vertex set of the simplex.
    pub fn vertices<'a>(&'a self, space: &'a Space) -> VertexSet<'a> {
        VertexSet::new(space, self.0.clone())
    }

    /// Converts the simplex into an array of `N` vertices, or returns `None` if
    /// the simplex has the wrong number of vertices.
    pub fn try_into_array<const N: usize>(&self) -> Option<[VertexId; N]> {
        self.0.iter().collect_vec().try_into().ok()
    }

    /// Returns a vertex from the simplex, or an error if the simplex is empty.
    pub(super) fn arbitrary_vertex(&self) -> Result<VertexId> {
        self.0.iter().next().ok_or_eyre("simplex is empty")
    }

    /// Returns all 1-dimensional elements of the simplex.
    pub fn edges(&self) -> impl '_ + Iterator<Item = [VertexId; 2]> {
        let verts: SmallVec<[VertexId; 8]> = self.0.iter().collect();
        verts
            .into_iter()
            .tuple_combinations()
            .map(|(v1, v2)| [v1, v2])
    }
    /// Returns all (N-1)-dimensional elements of the simplex.
    pub fn facets(&self) -> Result<impl '_ + Iterator<Item = Simplex>> {
        let ndim = self.ndim()?;
        let facet_ndim = ndim.checked_sub(1).ok_or_eyre("0D simplex has no facets")?;
        Ok(self.elements(facet_ndim))
    }
    /// Returns all elements of the simplex with a given number of dimensions.
    pub fn elements(&self, ndim: u8) -> impl '_ + Iterator<Item = Simplex> {
        self.0
            .iter()
            .combinations(ndim as usize + 1)
            .map(|verts| Simplex(Set64::from_iter(verts)))
    }
}

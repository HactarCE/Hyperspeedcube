use super::*;

/// Trait for [`SpaceMap`] to map different types.
pub trait SpaceMapFor<T: Copy> {
    /// Applies the map to a `thing` and returns the result.
    #[must_use]
    fn map(&mut self, thing: T) -> Result<T>;
}

/// Lazy map from one [`Space`] to another.
#[derive(Debug)]
pub struct SpaceMap<'a> {
    source: &'a Space,
    destination: &'a mut Space,
    vertices: HashMap<VertexId, VertexId>,
    polytopes: HashMap<PolytopeId, PolytopeId>,
}
impl<'a> SpaceMap<'a> {
    /// Constructs a map from `old_space` to `new_space`.
    pub fn new(source: &'a Space, destination: &'a mut Space) -> Result<Self> {
        ensure!(
            source.ndim() == destination.ndim(),
            "cannot map between spaces of different dimensions",
        );
        Ok(Self {
            source,
            destination,
            vertices: HashMap::new(),
            polytopes: HashMap::new(),
        })
    }
}
impl SpaceMapFor<VertexId> for SpaceMap<'_> {
    fn map(&mut self, thing: VertexId) -> Result<VertexId> {
        match self.vertices.entry(thing) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                Ok(*e.insert(self.destination.add_vertex(self.source[thing].clone())?))
            }
        }
    }
}
impl SpaceMapFor<PolytopeId> for SpaceMap<'_> {
    fn map(&mut self, thing: PolytopeId) -> Result<PolytopeId> {
        if let Some(&p) = self.polytopes.get(&thing) {
            return Ok(p);
        }

        let polytope_data = match &self.source[thing] {
            PolytopeData::Vertex(p) => PolytopeData::Vertex(self.map(*p)?),
            PolytopeData::Polytope {
                rank,
                boundary,
                flags,
            } => PolytopeData::Polytope {
                rank: *rank,
                boundary: boundary.iter().map(|b| self.map(b)).try_collect()?,
                flags: *flags,
            },
        };
        let new_id = self.destination.add_polytope(polytope_data)?;

        self.polytopes.insert(thing, new_id);
        Ok(new_id)
    }
}

use super::*;

/// Trait for [`SpaceMap`] to map different types.
pub trait SpaceMapFor<T: Copy> {
    /// Applies the map to a `thing` and returns the result.
    fn map(&mut self, thing: T) -> Result<T>;
}

/// Lazy map from one [`Space`] to another.
#[derive(Debug)]
pub struct SpaceMap<'a> {
    source: &'a Space,
    destination: &'a Space,
    vertices: HashMap<VertexId, VertexId>,
    polytopes: HashMap<ElementId, ElementId>,
    hyperplanes: HashMap<HyperplaneId, HyperplaneId>,
}
impl<'a> SpaceMap<'a> {
    /// Constructs a map from `old_space` to `new_space`.
    pub fn new(source: &'a Space, destination: &'a Space) -> Result<Self> {
        ensure!(
            source.ndim() == destination.ndim(),
            "cannot map between spaces of different dimensions",
        );
        source.ensure_not_same_as(destination)?;
        Ok(Self {
            source,
            destination,
            vertices: HashMap::new(),
            polytopes: HashMap::new(),
            hyperplanes: HashMap::new(),
        })
    }
}
impl SpaceMapFor<VertexId> for SpaceMap<'_> {
    fn map(&mut self, thing: VertexId) -> Result<VertexId> {
        match self.vertices.entry(thing) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                let vertex_pos = self.source.vertices.lock()[thing].clone();
                Ok(*e.insert(self.destination.add_vertex(vertex_pos)?))
            }
        }
    }
}
impl SpaceMapFor<ElementId> for SpaceMap<'_> {
    fn map(&mut self, thing: ElementId) -> Result<ElementId> {
        if let Some(&p) = self.polytopes.get(&thing) {
            return Ok(p);
        }

        let polytopes = self.source.polytopes.lock();
        let polytope_data = match polytopes[thing].clone() {
            PolytopeData::Vertex(p) => {
                drop(polytopes);
                PolytopeData::Vertex(self.map(p)?)
            }
            PolytopeData::Polytope {
                rank,
                boundary,
                hyperplane,

                is_primordial,

                seam,
                patch,
            } => {
                drop(polytopes);
                PolytopeData::Polytope {
                    rank,
                    boundary: boundary.iter().map(|b| self.map(b)).try_collect()?,
                    hyperplane,

                    is_primordial,

                    seam,
                    patch,
                }
            }
        };
        let new_id = self.destination.add_polytope(polytope_data)?;

        self.polytopes.insert(thing, new_id);
        Ok(new_id)
    }
}
impl SpaceMapFor<HyperplaneId> for SpaceMap<'_> {
    fn map(&mut self, thing: HyperplaneId) -> Result<HyperplaneId> {
        match self.hyperplanes.entry(thing) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                let hyperplane = self.source.hyperplanes.lock()[thing].clone();
                Ok(*e.insert(self.destination.hyperplanes.lock().push(hyperplane)?))
            }
        }
    }
}

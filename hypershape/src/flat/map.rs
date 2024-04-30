use super::*;

/// Trait for [`SpaceMap`] to map different types.
pub trait SpaceMapFor<T: Copy> {
    /// Applies the map to a `thing` and returns the result.
    #[must_use]
    fn map(&mut self, thing: T) -> T;

    /// Applies the map to a set of `things` and returns the result.
    #[must_use]
    fn map_set(&mut self, things: &Set64<T>) -> Set64<T>
    where
        T: Fits64,
    {
        self.map_iter(things.iter()).collect()
    }

    /// Applies the map to each element of an iterator and returns the result.
    #[must_use]
    fn map_iter<'a>(
        &'a mut self,
        things: impl 'a + Iterator<Item = T>,
    ) -> impl 'a + Iterator<Item = T> {
        things.map(move |thing| self.map(thing))
    }

    /// Applies the map to a `thing` in-place.
    fn map_mut(&mut self, thing: &mut T) {
        *thing = self.map(*thing);
    }

    /// Applies the map to a set of `things` in-place.
    fn map_set_mut(&mut self, things: &mut Set64<T>)
    where
        T: Fits64,
    {
        *things = self.map_set(things);
    }
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
    fn map(&mut self, thing: VertexId) -> VertexId {
        *self.vertices.entry(thing).or_insert_with(|| {
            self.destination
                .add_vertex(self.source[thing].clone())
                .expect("error adding vertex to space")
        })
    }
}
impl SpaceMapFor<PolytopeId> for SpaceMap<'_> {
    fn map(&mut self, thing: PolytopeId) -> PolytopeId {
        if let Some(&p) = self.polytopes.get(&thing) {
            return p;
        }

        let polytope_data = match &self.source[thing] {
            PolytopeData::Vertex(p) => PolytopeData::Vertex(self.map(*p)),
            PolytopeData::Polytope {
                rank,
                boundary,
                flags,
            } => PolytopeData::Polytope {
                rank: *rank,
                boundary: boundary.iter().map(|b| self.map(b)).collect(),
                flags: *flags,
            },
        };
        let new_id = self
            .destination
            .add_polytope(polytope_data)
            .expect("error adding polytope to space");

        self.polytopes.insert(thing, new_id);
        new_id
    }
}

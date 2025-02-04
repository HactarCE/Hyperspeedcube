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
    manifolds: HashMap<ManifoldRef, ManifoldRef>,
    polytopes: HashMap<AtomicPolytopeRef, AtomicPolytopeRef>,
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
            manifolds: HashMap::new(),
            polytopes: HashMap::new(),
        })
    }
}
impl SpaceMapFor<ManifoldRef> for SpaceMap<'_> {
    fn map(&mut self, thing: ManifoldRef) -> ManifoldRef {
        *self.manifolds.entry(thing).or_insert_with(|| {
            let blade = self.source.blade_of(thing);
            self.destination
                .add_manifold(blade)
                .expect("error adding blade to space")
        })
    }
}
impl SpaceMapFor<AtomicPolytopeRef> for SpaceMap<'_> {
    fn map(&mut self, thing: AtomicPolytopeRef) -> AtomicPolytopeRef {
        if let Some(&p) = self.polytopes.get(&thing) {
            return p;
        }

        let new_manifold = self.map(self.source.manifold_of(thing));
        let new_boundary = self.map_iter(self.source.boundary_of(thing)).collect();
        let new_ref = self
            .destination
            .add_atomic_polytope(new_manifold, new_boundary)
            .expect("error adding polytope to space");
        self.polytopes.insert(thing, new_ref);
        new_ref
    }
}

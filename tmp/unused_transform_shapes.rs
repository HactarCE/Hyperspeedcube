
/// Transforms a set of shapes.
pub fn transform_shapes(
    &mut self,
    shapes: &[Shape],
    isometry: Isometry,
    new_patch: PatchId,
) -> Result<Vec<Shape>> {
    let mut manifold_cache = HashMap::new();
    let mut shape_cache = HashMap::new();

    let mut transformed_shapes = vec![];
    for shape in shapes {
        let mut transformed_convex_components = AtomicShapeSet::new();
        for convex_component in &shape.0 {
            transformed_convex_components.insert(self.transform_convex_shape_cached(
                convex_component,
                isometry,
                new_patch,
                &mut manifold_cache,
                &mut shape_cache,
            )?);
        }
        transformed_shapes.push(Shape(transformed_convex_components));
    }
    Ok(transformed_shapes)
}
fn transform_convex_shape_cached(
    &mut self,
    shape: AtomicPolytopeRef,
    isometry: Isometry,
    new_patch: PatchId,
    manifold_cache: &mut HashMap<ManifoldId, ManifoldRef>,
    shape_cache: &mut HashMap<AtomicPolytopeId, AtomicPolytopeRef>,
) -> Result<AtomicPolytopeRef> {
    if let Some(&cached_result) = shape_cache.get(&shape.id) {
        Ok(cached_result * shape.sign)
    } else {
        let transformed_manifold = match manifold_cache.entry(self.manifold_of(shape).id) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                *e.insert(self.add_manifold(isometry.transform_blade(&self[*e.key()].blade))?)
            }
        };
        let mut shape_data = self[shape.id].clone();
        match &mut shape_data {
            AtomicPolytope::PointPair {
                patch,
                portal_sets: _,
                manifold,
            } => {
                *patch = new_patch;
                *manifold = transformed_manifold.id;
            }
            AtomicPolytope::NonPointPair {
                patch,
                portal_set: _,
                manifold,
                boundary,
            } => {
                *patch = new_patch;
                *manifold = transformed_manifold.id;
                *boundary = boundary
                    .iter()
                    .map(|boundary_elem| {
                        self.transform_convex_shape_cached(
                            boundary_elem,
                            isometry,
                            new_patch,
                            manifold_cache,
                            shape_cache,
                        )
                    })
                    .try_collect()?
            }
        }
        let result = AtomicPolytopeRef {
            id: self.get_or_insert_polytope_data(shape_data)?,
            sign: transformed_manifold.sign,
        };
        shape_cache.insert(shape.id, result);
        Ok(result * shape.sign)
    }
}
fn unportal(
    &mut self,
    shape: AtomicPolytopeRef,
    portal_set_to_remove: u32,
    cache: &mut HashMap<AtomicPolytopeId, AtomicPolytopeRef>,
) -> Result<AtomicPolytopeRef> {
    if let Some(&cached_result) = cache.get(&shape.id) {
        return Ok(cached_result * shape.sign);
    }

    let mut shape_data = self[shape.id].clone();
    match &mut shape_data {
        AtomicPolytope::PointPair { portal_sets, .. } => {
            portal_sets[0] &= !portal_set_to_remove;
            portal_sets[1] &= !portal_set_to_remove;
        }
        AtomicPolytope::NonPointPair {
            portal_set,
            boundary,
            ..
        } => {
            *portal_set &= !portal_set_to_remove;
            *boundary = boundary
                .iter()
                .map(|boundary| self.unportal(shape, portal_set_to_remove, cache))
                .try_collect()?;
        }
    }
    let result = AtomicPolytopeRef::from(self.get_or_insert_polytope_data(shape_data)?);
    cache.insert(shape.id, result);
    Ok(result)
}

pub fn expand_cut_divider_thru_portals(
    &mut self,
    divider: ManifoldRef,
    initial_patch: PatchId,
) -> Result<Vec<(ManifoldRef, PatchId)>> {
    let mut results = vec![(divider, initial_patch)];
    let mut seen: HashSet<(ManifoldRef, PatchId)> = results.iter().copied().collect();
    let mut next_unprocessed = 0;
    while let Some((manifold, patch)) = results.get(next_unprocessed) {
        for (portal, portal_data) in &self[initial_patch].portals {
            let untransformed_blade = &self[manifold.id].blade;
            let transformed_blade = portal_data.isometry.transform_blade(untransformed_blade);
            let mut new_manifold = self.add_manifold(transformed_blade)?;
            let mut new_patch = portal_data.new_patch;
            results.push((new_manifold, new_patch));
            seen.insert((new_manifold, new_patch));
        }
        next_unprocessed += 1;
        if results.len() > MAX_PORTAL_EXPANSION {
            bail!("too much symmetry expansion (exceeded maximum of {MAX_PORTAL_EXPANSION})");
        }
    }
    Ok(results)
}

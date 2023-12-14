/// Cut operation, which caches intermediate results.
#[derive(Debug)]
pub struct CutInProgress<'a> {
    pub(super) space: &'a mut Space,
    pub(super) op: Cut,
}
impl CutInProgress<'_> {
    /// Cuts a shape.
    pub fn cut(&mut self, shape: AtomicPolytopeRef) -> Result<ShapeCutResult> {
        self.space
            .cut_atomic_polytope(shape, &mut self.op)
            .map(|result| match result {
                AtomicPolytopeCutOutput::Flush => ShapeCutResult {
                    inside: Some(shape),
                    outside: Some(shape),
                    flush_facet: None,
                },

                AtomicPolytopeCutOutput::ManifoldInside => ShapeCutResult {
                    inside: Some(shape),
                    outside: None,
                    flush_facet: None,
                },

                AtomicPolytopeCutOutput::ManifoldOutside => ShapeCutResult {
                    inside: None,
                    outside: Some(shape),
                    flush_facet: None,
                },

                AtomicPolytopeCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection_shape,
                } => ShapeCutResult {
                    inside,
                    outside,
                    flush_facet: intersection_shape,
                },
            })
    }
    /// Cuts multiple shapes and returns the set resulting from it.
    pub fn cut_set(&mut self, shapes: AtomicShapeSet) -> Result<AtomicShapeSet> {
        shapes
            .into_iter()
            .map(|shape| {
                let result = self.cut(shape)?;
                Ok([result.inside, result.outside])
            })
            .flatten_ok()
            .flatten_ok()
            .collect()
    }
}

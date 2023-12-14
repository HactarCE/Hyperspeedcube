
/// Cuts a set of polytopes by a manifold.
#[tracing::instrument(skip_all)]
pub fn cut(
    &mut self,
    shapes: &AtomicShapeSet,
    params: CutParams,
) -> Result<(AtomicShapeSet, AtomicShapeSet)> {
    if params.inside == ShapeFate::Remove && params.outside == ShapeFate::Remove {
        // Why would you do this? You're just removing everything.
        return Ok((AtomicShapeSet::new(), AtomicShapeSet::new()));
    }

    let mut op = CutOp::new(params);
    let mut ret_inside = AtomicShapeSet::new();
    let mut ret_outside = AtomicShapeSet::new();
    for shape in shapes {
        match self.cut_convex_shape(shape, &mut op)? {
            AtomicPolytopeCutOutput::Flush => (), // Neither inside nor outside.
            AtomicPolytopeCutOutput::ManifoldInside => {
                ret_inside.insert(shape);
            }
            AtomicPolytopeCutOutput::ManifoldOutside => {
                ret_outside.insert(shape);
            }
            AtomicPolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection_shape: _,
            } => {
                ret_inside.extend(inside);
                ret_outside.extend(outside);
            }
        }
    }
    Ok((ret_inside, ret_outside))
}

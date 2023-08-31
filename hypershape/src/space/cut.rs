use super::*;

#[derive(Debug)]
pub struct CutInProgress<'a> {
    pub(super) space: &'a mut Space,
    pub(super) op: CutOp,
}
impl CutInProgress<'_> {
    #[must_use]
    pub fn cut(&mut self, shape: ShapeRef) -> Result<ShapeCutResult> {
        self.space
            .cut_shape(shape, &mut self.op)
            .map(|result| match result {
                ShapeSplitResult::Flush => ShapeCutResult {
                    inside: Some(shape),
                    outside: Some(shape),
                    flush_facet: None,
                },

                ShapeSplitResult::ManifoldInside => ShapeCutResult {
                    inside: Some(shape),
                    outside: None,
                    flush_facet: None,
                },

                ShapeSplitResult::ManifoldOutside => ShapeCutResult {
                    inside: None,
                    outside: Some(shape),
                    flush_facet: None,
                },

                ShapeSplitResult::NonFlush {
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
    #[must_use]
    pub fn cut_set(&mut self, shapes: ShapeSet) -> Result<ShapeSet> {
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

pub struct ShapeCutResult {
    pub inside: Option<ShapeRef>,
    pub outside: Option<ShapeRef>,
    pub flush_facet: Option<ShapeRef>,
}

/// Parameters for cutting a bunch of shapes.
#[derive(Debug, Clone)]
pub struct CutParams {
    /// Manifold that divides the inside of the cut from the outside of the cut.
    pub divider: ManifoldRef,
    /// What to do with the shapes on the inside of the cut.
    pub inside: ShapeFate,
    /// What to do with the shapes on the outside of the cut.
    pub outside: ShapeFate,
}
impl fmt::Display for CutParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ divider: {}, inside: {}, outside: {} }}",
            self.divider, self.inside, self.outside,
        )
    }
}

/// What to do with a shape resulting from a cutting operation.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ShapeFate {
    /// The shape should be removed.
    #[default]
    Remove,
    /// The shape should remain.
    Keep,
}
impl fmt::Display for ShapeFate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShapeFate::Remove => write!(f, "REMOVE"),
            ShapeFate::Keep => write!(f, "KEEP"),
        }
    }
}

/// In-progress slicing operation.
#[derive(Debug)]
pub(super) struct CutOp {
    /// Cut parameters.
    pub cut: CutParams,

    /// Cache of the result of splitting each shape.
    pub shape_split_results_cache: HashMap<ShapeId, ShapeSplitResult>,
    /// Cache of which side(s) of the cut contains each manifold.
    manifold_which_side_cache: HashMap<ManifoldId, ManifoldWhichSide>,
    /// Cache of the intersection of the cut with each manifold.
    manifold_intersections_cache: HashMap<ManifoldId, ManifoldRef>,
}
impl fmt::Display for CutOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CutOp")
            .field("cut", &self.cut)
            .finish_non_exhaustive()
    }
}
impl CutOp {
    pub fn new(cut: CutParams) -> Self {
        Self {
            cut,

            shape_split_results_cache: HashMap::new(),
            manifold_which_side_cache: HashMap::new(),
            manifold_intersections_cache: HashMap::new(),
        }
    }

    #[tracing::instrument(level = "trace", skip_all, fields(manifold = %manifold), ret(Debug), err(Debug))]
    pub fn cached_which_side_of_cut_contains_manifold(
        &mut self,
        space: &mut Space,
        manifold: ManifoldId,
    ) -> Result<ManifoldWhichSide> {
        Ok(match self.manifold_which_side_cache.entry(manifold) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                *e.insert(space.which_side(space.manifold(), self.cut.divider, manifold)?)
            }
        })
    }
    pub fn cached_intersection_of_manifold_and_cut(
        &mut self,
        space: &mut Space,
        manifold: ManifoldRef,
    ) -> Result<ManifoldRef> {
        Ok(match self.manifold_intersections_cache.entry(manifold.id) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => *e.insert(space.intersect(
                space.manifold(),
                self.cut.divider,
                manifold.id.into(),
            )?),
        } * manifold.sign)
    }
}

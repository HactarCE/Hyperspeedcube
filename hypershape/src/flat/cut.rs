use super::*;

/// Parameters for cutting shapes.
#[derive(Clone)]
pub struct CutParams {
    /// Plane that divides the inside of the cut from the outside of the cut.
    pub divider: Hyperplane,
    /// What to do with the shapes on the inside of the cut.
    pub inside: PolytopeFate,
    /// What to do with the shapes on the outside of the cut.
    pub outside: PolytopeFate,
}
impl fmt::Debug for CutParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ divider: {:?}, inside: {}, outside: {} }}",
            self.divider, self.inside, self.outside,
        )
    }
}

/// What to do with a shape resulting from a cutting operation.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PolytopeFate {
    /// The shape should be removed.
    #[default]
    Remove,
    /// The shape should remain.
    Keep,
}
impl fmt::Display for PolytopeFate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolytopeFate::Remove => write!(f, "REMOVE"),
            PolytopeFate::Keep => write!(f, "KEEP"),
        }
    }
}

/// In-progress cut operation, which caches intermediate results.
#[derive(Debug)]
pub struct Cut {
    space: Arc<Space>,
    /// Cut parameters.
    params: CutParams,
    /// Cache of the result of splitting each shape.
    output_cache: HashMap<ElementId, ElementCutOutput>,
}
impl fmt::Display for Cut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cut")
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}
impl Cut {
    /// Constructs a cutting operation that deletes polytopes on the outside of
    /// the cut and keeps only those on the inside.
    pub fn carve(space: &Space, divider: Hyperplane) -> Self {
        let params = CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Remove,
        };
        Self::new(space, params)
    }
    /// Constructs a cutting operation that keeps all resulting polytopes.
    pub fn slice(space: &Space, divider: Hyperplane) -> Self {
        let params = CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Keep,
        };
        Self::new(space, params)
    }

    /// Constructs a cutting operation.
    pub fn new(space: &Space, params: CutParams) -> Self {
        Self {
            space: space.arc(),
            params,
            output_cache: HashMap::new(),
        }
    }

    /// Returns the parameters used to create the cut.
    pub fn params(&self) -> &CutParams {
        &self.params
    }

    /// Cuts an element.
    pub fn cut(&mut self, element: impl ToElementId) -> Result<ElementCutOutput> {
        let cut = &mut *self;
        let space = cut.space.arc(); // TODO(perf): is this bad for perf?
        let element = element.to_element_id(&space);

        if let Some(&result) = cut.output_cache.get(&element) {
            return Ok(result);
        }

        let div = &cut.params.divider;

        let polytopes = space.polytopes.lock();
        let result = match polytopes[element].clone() {
            PolytopeData::Vertex(p) => match div.location_of_point(&space.vertices.lock()[p]) {
                PointWhichSide::On => ElementCutOutput::Flush,
                PointWhichSide::Inside => ElementCutOutput::all_inside(element, None),
                PointWhichSide::Outside => ElementCutOutput::all_outside(element, None),
            },
            PolytopeData::Polytope { rank, boundary, .. } => {
                let mut inside_boundary = Set64::<ElementId>::new();
                let mut outside_boundary = Set64::<ElementId>::new();
                let mut flush_polytopes = vec![];
                let mut flush_polytope_boundary = Set64::<ElementId>::new();

                drop(polytopes);

                if let Some(line @ [a, b]) = space.line_endpoints(element) {
                    let vertices = space.vertices.lock();
                    let HyperplaneLineIntersection {
                        a_loc,
                        b_loc,
                        intersection,
                    } = div.intersection_with_line_segment(line.map(|i| &vertices[i]));
                    drop(vertices);
                    for (v, v_loc) in [(a, a_loc), (b, b_loc)] {
                        let v = space.add_polytope(v.into())?;
                        match v_loc {
                            PointWhichSide::On => flush_polytopes.push(v),
                            PointWhichSide::Inside => {
                                inside_boundary.insert(v);
                            }
                            PointWhichSide::Outside => {
                                outside_boundary.insert(v);
                            }
                        }
                    }
                    if flush_polytopes.is_empty() {
                        if let Some(intersection_point) = intersection {
                            let v = space.add_vertex(intersection_point)?.into();
                            flush_polytopes.push(space.add_polytope(v)?);
                        }
                    }
                } else {
                    for b in boundary.iter() {
                        match cut.cut(b)? {
                            ElementCutOutput::Flush => flush_polytopes.push(b),
                            ElementCutOutput::NonFlush {
                                inside,
                                outside,
                                intersection,
                            } => {
                                inside_boundary.extend(inside);
                                outside_boundary.extend(outside);
                                flush_polytope_boundary.extend(intersection);
                            }
                        }
                    }
                }

                if flush_polytopes.len() > 1 {
                    ElementCutOutput::Flush
                } else {
                    let intersection = match flush_polytopes.first() {
                        Some(&p) => Some(p),
                        None => {
                            let new_id =
                                space.add_polytope_if_non_degenerate(PolytopeData::Polytope {
                                    rank: rank - 1,
                                    boundary: flush_polytope_boundary,

                                    is_primordial: false,

                                    seam: None,
                                    patch: None,
                                })?;

                            if let Some(new) = new_id {
                                // New facet! Cache the hyperplane.
                                let plane = cut.params.divider.clone();
                                space.cached_hyperplane_of_facet.lock().insert(new, plane);
                            }

                            new_id
                        }
                    };

                    let inside = match cut.params.inside {
                        PolytopeFate::Keep => {
                            inside_boundary.extend(intersection);
                            space.add_subpolytope_if_non_degenerate(element, inside_boundary)?
                        }
                        PolytopeFate::Remove => None,
                    };

                    let outside = match cut.params.outside {
                        PolytopeFate::Keep => {
                            outside_boundary.extend(intersection);
                            space.add_subpolytope_if_non_degenerate(element, outside_boundary)?
                        }
                        PolytopeFate::Remove => None,
                    };

                    ElementCutOutput::NonFlush {
                        inside,
                        outside,
                        intersection,
                    }
                }
            }
        };

        cut.output_cache.insert(element, result);
        Ok(result)
    }
}

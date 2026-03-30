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
impl CutParams {
    /// Constructs a cutting operation that deletes polytopes on the outside of
    /// the cut and keeps only those on the inside.
    pub fn carve(divider: Hyperplane) -> Self {
        Self {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Remove,
        }
    }
    /// Constructs a cutting operation that keeps all resulting polytopes.
    pub fn slice(divider: Hyperplane) -> Self {
        Self {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Keep,
        }
    }
}

#[derive(Debug)]
struct CutCache {
    /// Cached space ID.
    space_id: u64,
    /// Cached ID of the new hyperplane.
    hyperplane_id: HyperplaneId,
    /// Cache of the result of splitting each shape.
    outputs: HashMap<ElementId, ElementCutOutput>,
}

/// In-progress cut operation, which caches intermediate results.
#[derive(Debug)]
pub struct Cut {
    /// Cut parameters.
    params: CutParams,
    /// Cache, which is initialized when the first polytope is cut.
    cache: Option<CutCache>,
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
    ///
    /// This is equivalent to `Cut::new(CutParams::carve(divider))`.
    pub fn carve(divider: Hyperplane) -> Self {
        Self::new(CutParams::carve(divider))
    }
    /// Constructs a cutting operation that keeps all resulting polytopes.
    ///
    /// This is equivalent to `Cut::new(CutParams::slice(divider))`.
    pub fn slice(divider: Hyperplane) -> Self {
        Self::new(CutParams::slice(divider))
    }

    /// Constructs a cutting operation.
    pub fn new(params: CutParams) -> Self {
        Self {
            params,
            cache: None,
        }
    }

    /// Returns the parameters used to create the cut.
    pub fn params(&self) -> &CutParams {
        &self.params
    }

    /// Cuts an element.
    #[expect(clippy::too_many_lines)] // it's a complicated algorithm!
    pub fn cut(
        &mut self,
        space: &mut Space,
        element: impl ToElementId,
    ) -> Result<ElementCutOutput> {
        let cache = match &mut self.cache {
            Some(c) => c,
            None => {
                let c = CutCache {
                    space_id: space.id,
                    hyperplane_id: space.hyperplanes.push(self.params.divider.clone())?,
                    outputs: HashMap::new(),
                };
                self.cache.insert(c)
            }
        };

        if cache.space_id != space.id {
            bail!("cut constructed for a different space");
        };
        let hyperplane_id = cache.hyperplane_id;

        let element = element.to_element_id(space);

        let distance = self.params.divider.distance();
        if distance.is_infinite() {
            let element = element.to_element_id(space);
            return Ok(ElementCutOutput::NonFlush {
                inside: (distance == Float::INFINITY).then_some(element),
                outside: (distance == -Float::INFINITY).then_some(element),
                intersection: None,
            });
        }

        if let Some(&result) = cache.outputs.get(&element) {
            return Ok(result);
        }

        let div = &self.params.divider;

        let result = match space.polytopes[element].clone() {
            PolytopeData::Vertex(v) => match div.location_of_point(&space.vertex_pos(v)) {
                PointWhichSide::On => ElementCutOutput::Flush,
                PointWhichSide::Inside => ElementCutOutput::all_inside(element, None),
                PointWhichSide::Outside => ElementCutOutput::all_outside(element, None),
            },
            PolytopeData::Polytope { rank, boundary, .. } => {
                let mut inside_boundary = Set64::<ElementId>::new();
                let mut outside_boundary = Set64::<ElementId>::new();
                let mut flush_polytopes = vec![];
                let mut flush_polytope_boundary = Set64::<ElementId>::new();

                if let Some([a, b]) = space.line_endpoints(element) {
                    let HyperplaneLineIntersection {
                        a_loc,
                        b_loc,
                        intersection,
                    } = div.intersection_with_line_segment([
                        &space.vertex_pos(a),
                        &space.vertex_pos(b),
                    ]);
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
                    if flush_polytopes.is_empty()
                        && let Some(intersection_point) = intersection
                    {
                        let (_vertex_id, element_id) = space.add_vertex(intersection_point)?;
                        flush_polytopes.push(element_id);
                    }
                } else {
                    for b in boundary.iter() {
                        match self.cut(space, b)? {
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
                        None => space.add_polytope_if_non_degenerate(PolytopeData::Polytope {
                            rank: rank - 1,
                            boundary: flush_polytope_boundary,
                            hyperplane: (rank == space.ndim()).then_some(hyperplane_id),
                            is_primordial: false,
                        })?,
                    };

                    let inside = match self.params.inside {
                        PolytopeFate::Keep => {
                            inside_boundary.extend(intersection);
                            space.add_subpolytope_if_non_degenerate(element, inside_boundary)?
                        }
                        PolytopeFate::Remove => None,
                    };

                    let outside = match self.params.outside {
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

        let cache = self.cache.as_mut().expect("missing cut cache");
        cache.outputs.insert(element, result);
        Ok(result)
    }
}

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
    /// Whether the cut creates a portal.
    pub portal: bool,
}
impl fmt::Debug for CutParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ divider: {:?}, inside: {}, outside: {}, portal: {} }}",
            self.divider, self.inside, self.outside, self.portal,
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

#[derive(Debug)]
struct CutCache {
    /// Cached space ID.
    space_id: u64,
    /// Cached ID of the new hyperplane.
    hyperplane_id: HyperplaneId,
    /// ID of the portal created by the cut.
    portal_id: Option<PortalId>,
    /// For each existing portal, whether it is perpendicular to the new cut.
    perpendicular_portals: TiVec<PortalId, Option<bool>>,
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
    pub fn carve(divider: Hyperplane) -> Self {
        Self::new(CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Remove,
            portal: false,
        })
    }
    /// Constructs a cutting operation that keeps all resulting polytopes.
    pub fn slice(divider: Hyperplane) -> Self {
        Self::new(CutParams {
            divider,
            inside: PolytopeFate::Keep,
            outside: PolytopeFate::Keep,
            portal: false,
        })
    }

    /// Constructs a cutting operation that creates a portal at `divider`.
    ///
    /// Only the **outside** of the divider is kept.
    pub fn carve_portal(divider: Hyperplane) -> Self {
        Self::new(CutParams {
            divider,
            inside: PolytopeFate::Remove,
            outside: PolytopeFate::Keep,
            portal: true,
        })
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

    fn is_perpendicular_to_portal(&mut self, space: &Space, portal: PortalId) -> bool {
        let cache = self.cache.as_mut().expect("missing cut cache");
        *cache.perpendicular_portals[portal].get_or_insert_with(|| {
            let portal_hyperplane = &space.hyperplanes[space.portals[portal].hyperplane];
            APPROX.eq_zero(portal_hyperplane.normal().dot(self.params.divider.normal()))
        })
    }

    /// Cuts an element.
    #[expect(clippy::too_many_lines)] // it's a complicated algorithm!
    pub fn cut(
        &mut self,
        space: &mut Space,
        element: impl ElementIdConvert,
    ) -> Result<ElementCutOutput> {
        let element = element.to_element_id(space);

        let distance = self.params.divider.distance();
        if distance.is_infinite() {
            return Ok(ElementCutOutput::NonFlush {
                inside: (distance == Float::INFINITY).then_some(element),
                outside: (distance == -Float::INFINITY).then_some(element),
                intersection: None,
            });
        }

        let cache = get_or_insert_cache(&mut self.cache, &self.params, space)?;

        if cache.space_id != space.id {
            bail!("cut constructed for a different space");
        };
        let hyperplane_id = cache.hyperplane_id;
        let new_portal_id = cache.portal_id;

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
            PolytopeData::Polytope {
                rank,
                boundary,
                boundary_portals,
                ..
            } => {
                let mut inside_boundary = Set64::<ElementId>::new();
                let mut outside_boundary = Set64::<ElementId>::new();
                let mut flush_polytopes = vec![];
                let mut flush_polytope_boundary = Set64::<ElementId>::new();
                let mut flush_polytope_boundary_portals =
                    SmallVec::<[(PortalId, ElementId); 8]>::new();
                let mut inside_boundary_portals = SmallVec::<[(PortalId, ElementId); 8]>::new();
                let mut outside_boundary_portals = SmallVec::<[(PortalId, ElementId); 8]>::new();
                let mut flush_polytope_portals = SmallVec::<[PortalId; 8]>::new();
                flush_polytope_portals.extend(new_portal_id);

                if rank == 1 {
                    let [a, b] = boundary
                        .iter()
                        .collect_array()
                        .ok_or_eyre("bad line segment")?;
                    let HyperplaneLineIntersection {
                        a_loc,
                        b_loc,
                        intersection,
                    } = div.intersection_with_line_segment([
                        &space.vertex_pos(a),
                        &space.vertex_pos(b),
                    ]);
                    for (v, v_loc) in [(a, a_loc), (b, b_loc)] {
                        let boundary_portals_for_element = boundary_portals.pairs_for_element(v);
                        match v_loc {
                            PointWhichSide::On => {
                                flush_polytopes.push(v);
                                if new_portal_id.is_some() {
                                    flush_polytope_portals
                                        .extend(boundary_portals_for_element.map(|(p, _)| p));
                                }
                            }
                            PointWhichSide::Inside => {
                                inside_boundary.insert(v);
                                inside_boundary_portals.extend(boundary_portals_for_element);
                            }
                            PointWhichSide::Outside => {
                                outside_boundary.insert(v);
                                outside_boundary_portals.extend(boundary_portals_for_element);
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
                            ElementCutOutput::Flush => {
                                flush_polytopes.push(b);
                                flush_polytope_portals
                                    .extend(boundary_portals.portals_for_element(b));
                            }
                            ElementCutOutput::NonFlush {
                                inside,
                                outside,
                                intersection,
                            } => {
                                if let Some(inside) = inside {
                                    inside_boundary.insert(inside);
                                    inside_boundary_portals.extend(
                                        boundary_portals
                                            .portals_for_element(b)
                                            .map(|p| (p, inside)),
                                    );
                                }
                                if let Some(outside) = outside {
                                    outside_boundary.insert(outside);
                                    outside_boundary_portals.extend(
                                        boundary_portals
                                            .portals_for_element(b)
                                            .map(|p| (p, outside)),
                                    );
                                }
                                if let Some(intersection) = intersection {
                                    flush_polytope_boundary.insert(intersection);
                                    flush_polytope_boundary_portals.extend(
                                        boundary_portals
                                            .portals_for_element(b)
                                            .map(|p| (p, intersection)),
                                    );
                                }
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
                            boundary_portals: BoundaryPortals::new(
                                flush_polytope_boundary_portals
                                    .into_iter()
                                    .filter(|&(p, _)| self.is_perpendicular_to_portal(space, p)),
                            ),
                            hyperplane: (rank == space.ndim()).then_some(hyperplane_id),
                            is_primordial: false,
                        })?,
                    };
                    let intersection_portals: SmallVec<[_; 8]> = match intersection {
                        Some(i) => flush_polytope_portals.iter().map(|&p| (p, i)).collect(),
                        None => smallvec![],
                    };

                    let inside = match self.params.inside {
                        PolytopeFate::Keep => {
                            inside_boundary.extend(intersection);
                            inside_boundary_portals.extend(intersection_portals.clone());
                            space.add_subpolytope_if_non_degenerate(
                                element,
                                inside_boundary,
                                inside_boundary_portals,
                            )?
                        }
                        PolytopeFate::Remove => None,
                    };

                    let outside = match self.params.outside {
                        PolytopeFate::Keep => {
                            outside_boundary.extend(intersection);
                            outside_boundary_portals.extend(intersection_portals);
                            space.add_subpolytope_if_non_degenerate(
                                element,
                                outside_boundary,
                                outside_boundary_portals,
                            )?
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

fn get_or_insert_cache<'a>(
    opt_cache: &'a mut Option<CutCache>,
    params: &CutParams,
    space: &mut Space,
) -> Result<&'a mut CutCache> {
    if opt_cache.is_none() {
        let hyperplane_id = space.add_hyperplane(params.divider.clone())?;
        let c = CutCache {
            space_id: space.id,
            hyperplane_id,
            portal_id: if params.portal {
                Some(space.portals.push(PortalData {
                    hyperplane: hyperplane_id,
                })?)
            } else {
                None
            },
            perpendicular_portals: TiVec::new_with_len(space.portals.len()),
            outputs: HashMap::new(),
        };
        *opt_cache = Some(c);
    }
    Ok(opt_cache.as_mut().expect("missing cache"))
}

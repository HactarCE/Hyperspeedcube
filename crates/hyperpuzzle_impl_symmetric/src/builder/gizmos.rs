use eyre::{OptionExt, Result, eyre};
use hypergroup::{ConstraintSet, GroupElementId};
use hypermath::pga::Motor;
use hypermath::{APPROX, ApproxHashMap, Hyperplane, Point, Vector, VectorRef, approx_collections};
use hyperpuzzle_core::{Axis, Mesh, PerAxis, PerGizmoFace, TiMask};
use hyperpuzzle_impl_nd_euclid::NdEuclidAxisVectors;
use hypershape::{Cut, Space};
use hypuz_notation::{Move, Transform};
use itertools::Itertools;

use super::TwistSystemProduct;
use crate::{NamedPointSet, StabilizerFamily, SymmetricTwistSystemComponent};

pub fn build_3d_gizmo(
    mesh: &mut Mesh,
    gizmo_twists: &mut PerGizmoFace<Move>,
    axis_vectors: &PerAxis<Vector>,
    twists: &SymmetricTwistSystemComponent,
) -> Result<()> {
    let axis_from_vector =
        ApproxHashMap::from_iter(APPROX, axis_vectors.iter().map(|(ax, v)| (v.clone(), ax)));

    let mut space = Space::new(3)?;
    let mut seen_axes = TiMask::new_empty(twists.axes.len());
    for facet_id in gizmo_facets(&mut space, axis_vectors, twists)? {
        let init_axis = *axis_from_vector
            .get(space.get(facet_id).hyperplane_pole()?.into_vector())
            .ok_or_eyre("unknown axis vector")?;

        if seen_axes.contains(init_axis) {
            continue; // already handled!
        }
        seen_axes.insert(init_axis);

        let unfolded_face_id = space.unfold(facet_id.into())?;
        let unfolded_face = space.get(unfolded_face_id).as_face()?;

        let vertex_positions = unfolded_face
            .vertices_in_order()?
            .map(|v| v.pos())
            .collect_vec();

        // Generate mesh for each face
        for (axis, _, m) in orbit_axes_with_representatives(init_axis, twists, &mut seen_axes) {
            let transformed_vertex_positions = vertex_positions.iter().map(|p| m.transform(p));
            let surface_id = mesh.add_gizmo_surface(&axis_vectors[axis])?;
            let range = mesh.add_gizmo_polygon(transformed_vertex_positions, surface_id)?;
            mesh.add_gizmo_face(range)?;
            gizmo_twists.push(Transform::new(&twists.axes.names[axis], None).into())?;
        }
    }

    Ok(())
}

pub fn build_4d_gizmo(
    mesh: &mut Mesh,
    gizmo_twists: &mut PerGizmoFace<Move>,
    axis_vectors: &PerAxis<Vector>,
    twists: &SymmetricTwistSystemComponent,
    mut warn_fn: impl FnMut(eyre::Report),
) -> Result<()> {
    let axis_from_vector =
        ApproxHashMap::from_iter(APPROX, axis_vectors.iter().map(|(ax, v)| (v.clone(), ax)));

    let mut space = Space::new(4)?;
    let mut seen_axes = TiMask::new_empty(twists.axes.len());
    'facet: for facet_id in gizmo_facets(&mut space, axis_vectors, twists)? {
        let init_axis = *axis_from_vector
            .get(space.get(facet_id).hyperplane_pole()?.into_vector())
            .ok_or_eyre("unknown axis vector")?;

        if seen_axes.contains(init_axis) {
            continue; // already handled!
        }
        seen_axes.insert(init_axis);

        let unfolded_cell_id = space.unfold(facet_id.into())?;

        let mut vector_to_twist_family = ApproxHashMap::new(APPROX);

        let (undeorbiter, orbit_index) = twists.axis_undeorbiters[init_axis];
        let axis_orbit = &twists.axis_orbits[orbit_index];
        for (secondary, _unit_twist, gizmo_pole_distance) in &axis_orbit.stabilizer_twists {
            // Transform `secondary` to be based on `init_axis`.
            let init_secondary =
                secondary.transform_by_group_element(&twists.named_point_action, undeorbiter);

            // IIFE to mimic try_block
            let init_vector = (|| {
                init_secondary
                    .vector(&twists.named_point_vectors)
                    .rejected_from(&axis_vectors[init_axis])?
                    .normalize_to(*gizmo_pole_distance)
            })()
            .ok_or_eyre("gizmo pole distance cannot be zero")?;
            vector_to_twist_family.insert(
                init_vector.clone(),
                (init_secondary.clone(), *gizmo_pole_distance),
            );
            // Generate the stabilizer subgroup of the axis. The coset returned
            // from `solve()` must contain the identity, therefore the coset is
            // equivalent to its subgroup.
            let subgroup_generators = axis_orbit
                .subgroup_solver
                .lock()
                .solve(&ConstraintSet::EMPTY)
                .ok_or_eyre("no axis stabilizer")?
                .subgroup
                .generators
                .into_iter()
                .map(|g| (g, twists.group.motor(g)))
                .collect_vec();
            hypergroup::orbit(
                (init_vector, init_secondary),
                &subgroup_generators,
                |(vector, secondary), (g, m)| {
                    let mut new_vector = m.transform(vector);
                    if let approx_collections::hash_map::Entry::Vacant(entry) =
                        vector_to_twist_family.entry_with_mut_key(&mut new_vector)
                    {
                        let new_secondary =
                            secondary.transform_by_group_element(&twists.named_point_action, *g);
                        entry.insert((new_secondary.clone(), *gizmo_pole_distance));
                        Some((new_vector, new_secondary))
                    } else {
                        None
                    }
                },
            );
        }

        // Carve gizmo faces
        let mut cell = unfolded_cell_id;
        let mut faces = vec![];
        for (v, (secondary, gizmo_pole_distance)) in vector_to_twist_family {
            let cut_plane = Hyperplane::from_pole(v).ok_or_eyre("bad gizmo pole")?;
            let mut cut = Cut::carve(cut_plane);
            let cut_result = cut.cut(&mut space, cell)?;
            if let Some(cut_cell) = cut_result.inside() {
                cell = cut_cell;
            } else {
                warn_fn(eyre!(
                    "twist gizmo for axis {:?} is empty due to {} with distance {}",
                    &twists.axes.names[init_axis],
                    StabilizerFamily {
                        primary: init_axis,
                        secondary
                    }
                    .name(&twists.axes.names, &twists.named_point_names),
                    gizmo_pole_distance,
                ));
                continue 'facet;
            };

            for (face, _, _) in &mut faces {
                if let Some(f) = face {
                    *face = cut.cut(&mut space, *f)?.inside();
                }
            }

            faces.push((cut_result.intersection(), secondary, gizmo_pole_distance));
        }

        // Generate vertex positions for each face
        let faces: Vec<(Vec<Point>, NamedPointSet)> = faces
            .into_iter()
            .filter_map(|(face, secondary, gizmo_pole_distance)| match face {
                Some(f) => Some((f, secondary)),
                None => {
                    warn_fn(eyre!(
                        "gizmo pole distance of {} is too far for {}",
                        gizmo_pole_distance,
                        StabilizerFamily {
                            primary: init_axis,
                            secondary
                        }
                        .name(&twists.axes.names, &twists.named_point_names)
                    ));
                    None
                }
            })
            .map(|(face, secondary)| {
                let vertex_positions = space
                    .get(face)
                    .as_face()?
                    .vertices_in_order()?
                    .map(|v| v.pos())
                    .collect_vec();
                eyre::Ok((vertex_positions, secondary))
            })
            .try_collect()?;

        // Generate mesh for each cell/axis
        for (axis, e, m) in orbit_axes_with_representatives(init_axis, twists, &mut seen_axes) {
            // Generate mesh for each face
            for (vertex_positions, secondary) in &faces {
                let transformed_vertex_positions = vertex_positions.iter().map(|p| m.transform(p));
                let transformed_secondary =
                    secondary.transform_by_group_element(&twists.named_point_action, e);
                let surface_id = mesh.add_gizmo_surface(&axis_vectors[axis])?;
                let range = mesh.add_gizmo_polygon(transformed_vertex_positions, surface_id)?;
                mesh.add_gizmo_face(range)?;
                let family_str = StabilizerFamily {
                    primary: axis,
                    secondary: transformed_secondary,
                }
                .name(&twists.axes.names, &twists.named_point_names);
                gizmo_twists.push(Transform::new(family_str, None).into())?;
            }
        }
    }

    Ok(())
}

fn gizmo_facets(
    space: &mut Space,
    axis_vectors: &PerAxis<Vector>,
    twists: &SymmetricTwistSystemComponent,
) -> Result<Vec<hypershape::FacetId>> {
    let mirror_planes = twists
        .coxeter_mirrors
        .iter()
        .filter_map(|mirror_vector| Hyperplane::new(mirror_vector, 0.0));
    let carve_planes = twists
        .axis_orbits
        .iter()
        .filter_map(|orbit| Hyperplane::from_pole(&axis_vectors[orbit.first]));

    let gizmo_polyhedron = space.add_folded_shape(mirror_planes, carve_planes)?;
    let gizmo_polyhedron = space.get(gizmo_polyhedron);
    Ok(gizmo_polyhedron
        .facets()
        .filter(|&f| {
            !gizmo_polyhedron
                .boundary_portals()
                .contains_element(f.as_element().id())
        })
        .map(|f| f.id())
        .collect())
}

fn orbit_axes_with_representatives(
    init: Axis,
    twists: &SymmetricTwistSystemComponent,
    seen: &mut TiMask<Axis>,
) -> Vec<(Axis, GroupElementId, Motor)> {
    hypergroup::orbit_collect(
        (init, GroupElementId::IDENTITY, Motor::ident(twists.ndim())),
        twists.group.generators(),
        |_, (ax, e, m), &g| {
            let new_axis = twists.axis_action.act(g, *ax);
            (!seen.contains(new_axis)).then(|| {
                seen.insert(new_axis);
                let new_elem = twists.group.compose(g, *e);
                let new_motor = twists.group.motor(g) * m;
                (new_axis, new_elem, new_motor)
            })
        },
    )
}

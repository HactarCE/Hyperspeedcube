use eyre::{OptionExt, Result, eyre};
use hypergroup::{ConstraintSet, ExceededOrbitLimit, GroupElementId};
use hypermath::pga::Motor;
use hypermath::{
    APPROX, ApproxHashMap, Hyperplane, Matrix, Point, Vector, VectorRef, approx_collections,
};
use hyperpuzzle_core::{Axis, Mesh, PerAxis, PerGizmoFace, TiMask};
use hypershape::{Cut, Space};
use hypuz_notation::{Move, Transform};
use itertools::Itertools;

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

        let mut vertex_positions = unfolded_face
            .vertices_in_order()?
            .map(|v| v.pos())
            .collect_vec();
        // Enforce consistent winding order for gizmo faces.
        if Matrix::from_cols((0..3).map(|i| vertex_positions[i].as_vector())).determinant() > 0.0 {
            vertex_positions.reverse();
        }

        // Generate mesh for each face
        for (axis, _, m) in orbit_axes_with_representatives(init_axis, twists, &mut seen_axes)? {
            let mut transformed_vertex_positions = vertex_positions
                .iter()
                .map(|p| m.transform(p))
                .collect_vec();
            if m.is_reflection() {
                transformed_vertex_positions.reverse();
            }
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
        let init_axis_vector = space.get(facet_id).hyperplane_pole()?.into_vector();
        let init_axis = *axis_from_vector
            .get(init_axis_vector.clone())
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
            hypergroup::orbit_with_limit(
                hypergroup::ORBIT_LIMIT,
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
            )?;
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
                let mut vertex_positions = space
                    .get(face)
                    .as_face()?
                    .vertices_in_order()?
                    .map(|v| v.pos())
                    .collect_vec();
                // Enforce consistent winding order for gizmo faces.
                if Matrix::from_cols([
                    &init_axis_vector,
                    vertex_positions[0].as_vector(),
                    vertex_positions[1].as_vector(),
                    vertex_positions[2].as_vector(),
                ])
                .determinant()
                    > 0.0
                {
                    vertex_positions.reverse();
                }
                eyre::Ok((vertex_positions, secondary))
            })
            .try_collect()?;

        // Generate mesh for each cell/axis
        for (axis, e, m) in orbit_axes_with_representatives(init_axis, twists, &mut seen_axes)? {
            // Generate mesh for each face
            for (vertex_positions, secondary) in &faces {
                let mut transformed_vertex_positions = vertex_positions
                    .iter()
                    .map(|p| m.transform(p))
                    .collect_vec();
                if m.is_reflection() {
                    transformed_vertex_positions.reverse();
                }
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

/// Returns a list of facets to use for constructing gizmos. Each facet
/// corresponds to one axis.
///
/// Note that "facet" is the same as "face" in 3D, but in 4D "facet" is the same
/// as "cell."
fn gizmo_facets(
    space: &mut Space,
    axis_vectors: &PerAxis<Vector>,
    twists: &SymmetricTwistSystemComponent,
) -> Result<Vec<hypershape::FacetId>> {
    let mirror_planes = twists
        .fundamental_region_mirrors
        .iter()
        .filter_map(|v| Hyperplane::new(v, 0.0));
    let carve_planes = twists
        .axis_vectors
        .vectors_by_id
        .iter_values()
        .filter_map(|v| Hyperplane::from_pole(v));

    let gizmo_polytope = space.add_folded_shape(mirror_planes, carve_planes)?;
    let gizmo_polytope = space.get(gizmo_polytope);
    Ok(gizmo_polytope
        .facets()
        .filter(|&facet| {
            !gizmo_polytope
                .boundary_portals()
                .contains_element(facet.as_element().id())
        })
        .map(|f| f.id())
        .collect())
}

fn orbit_axes_with_representatives(
    init: Axis,
    twists: &SymmetricTwistSystemComponent,
    seen: &mut TiMask<Axis>,
) -> Result<Vec<(Axis, GroupElementId, Motor)>, ExceededOrbitLimit> {
    hypergroup::orbit_collect_with_limit(
        hypergroup::ORBIT_LIMIT,
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

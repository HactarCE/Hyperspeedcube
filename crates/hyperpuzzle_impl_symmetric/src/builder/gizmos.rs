use std::collections::HashMap;

use eyre::{OptionExt, Result, eyre};
use hypergroup::{
    ConstraintSet, ConstraintSolver, CoxeterMatrix, GroupAction, GroupElementId, IsometryGroup,
};
use hypermath::{
    APPROX, ApproxHashMap, Float, Hyperplane, Point, Vector, VectorRef, approx_collections,
    pga::Motor,
};
use hyperpuzzle_core::{Axis, Mesh, NameSpecBiMap, PerAxis, PerGizmoFace, TiMask};
use hyperpuzzle_impl_nd_euclid::GizmoTwist;
use hypershape::{Cut, ElementCutOutput, ElementId, ElementIdConvert, FaceId, FacetId, Space};
use hypuz_notation::{Multiplier, Transform};
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;

use crate::{StabilizableAxisSet, StabilizerFamily, builder::ProductPuzzleAxes};

pub fn build_3d_gizmo<'a>(
    mesh: &mut Mesh,
    gizmo_twists: &mut PerGizmoFace<GizmoTwist>,
    axes: &ProductPuzzleAxes,
    axis_names: &NameSpecBiMap<Axis>,
) -> Result<()> {
    let axis_from_vector =
        ApproxHashMap::from_iter(APPROX, axes.vectors.iter().map(|(ax, v)| (v.clone(), ax)));

    let mut space = Space::new(3)?;
    let mut seen_axes = TiMask::new_empty(axes.len());
    for facet_id in gizmo_facets(&mut space, axes)? {
        let init_axis = *axis_from_vector
            .get(space.get(facet_id).hyperplane()?.pole().into_vector())
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
        for (axis, _, m) in orbit_axes_with_representatives(init_axis, axes, &mut seen_axes) {
            let transformed_vertex_positions = vertex_positions.iter().map(|p| m.transform(p));
            let surface_id = mesh.add_gizmo_surface(&axes.vectors[axis])?;
            let range = mesh.add_gizmo_polygon(transformed_vertex_positions, surface_id)?;
            mesh.add_gizmo_face(range)?;
            gizmo_twists.push(GizmoTwist {
                axis,
                transform: Transform::new(&axis_names[axis], None),
                multiplier: Multiplier(1),
            })?;
        }
    }

    Ok(())
}

pub fn build_4d_gizmo<'a>(
    mesh: &mut Mesh,
    gizmo_twists: &mut PerGizmoFace<GizmoTwist>,
    solver: &mut ConstraintSolver<Axis>,
    axes: &ProductPuzzleAxes,
    axis_names: &NameSpecBiMap<Axis>,
    mut warn_fn: impl FnMut(eyre::Report),
) -> Result<()> {
    let axis_from_vector =
        ApproxHashMap::from_iter(APPROX, axes.vectors.iter().map(|(ax, v)| (v.clone(), ax)));

    let mut space = Space::new(4)?;
    let mut seen_axes = TiMask::new_empty(axes.len());
    'facet: for facet_id in gizmo_facets(&mut space, axes)? {
        let init_axis = *axis_from_vector
            .get(space.get(facet_id).hyperplane()?.pole().into_vector())
            .ok_or_eyre("unknown axis vector")?;

        if seen_axes.contains(init_axis) {
            continue; // already handled!
        }
        seen_axes.insert(init_axis);

        let mut unfolded_cell_id = space.unfold(facet_id.into())?;

        let mut vector_to_twist_family = ApproxHashMap::new(APPROX);
        for (family, gizmo_pole_distance) in &axes.stabilizer_twists {
            if let Some(coset) = solver.solve(&ConstraintSet::from([[family.primary, init_axis]])) {
                let right_coset = coset.to_right_coset();
                let init_secondary = family
                    .secondary
                    .transform_by_group_element(&axes.action, right_coset.rhs);
                // IIFE to mimic try_block
                let init_vector = (|| {
                    init_secondary
                        .vector(&axes.vectors)
                        .rejected_from(&axes.vectors[init_axis])?
                        .normalize_to(*gizmo_pole_distance)
                })()
                .ok_or_eyre("gizmo pole distance cannot be zero")?;
                vector_to_twist_family.insert(
                    init_vector.clone(),
                    (init_secondary.clone(), *gizmo_pole_distance),
                );
                let subgroup_generators = right_coset
                    .subgroup
                    .generators
                    .into_iter()
                    .map(|g| (g, axes.group.motor(g)))
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
                                secondary.transform_by_group_element(&axes.action, *g);
                            entry.insert((new_secondary.clone(), *gizmo_pole_distance));
                            Some((new_vector, new_secondary))
                        } else {
                            None
                        }
                    },
                );
            }
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
                    &axis_names[init_axis],
                    StabilizerFamily {
                        primary: init_axis,
                        secondary
                    }
                    .debug_str(&axis_names),
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
        let faces: Vec<(Vec<Point>, StabilizableAxisSet)> = faces
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
                        .debug_str(axis_names)
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
        for (axis, e, m) in orbit_axes_with_representatives(init_axis, axes, &mut seen_axes) {
            // Generate mesh for each face
            for (vertex_positions, secondary) in &faces {
                let transformed_vertex_positions = vertex_positions.iter().map(|p| m.transform(p));
                let transformed_secondary = secondary.transform_by_group_element(&axes.action, e);
                let surface_id = mesh.add_gizmo_surface(&axes.vectors[axis])?;
                let range = mesh.add_gizmo_polygon(transformed_vertex_positions, surface_id)?;
                mesh.add_gizmo_face(range)?;
                let family_str = StabilizerFamily {
                    primary: axis,
                    secondary: transformed_secondary,
                }
                .iter_flatten()
                .map(|ax| &axis_names[ax])
                .join("_"); // TODO: proper separator
                gizmo_twists.push(GizmoTwist {
                    axis,
                    transform: Transform::new(family_str, None),
                    multiplier: Multiplier(1),
                })?;
            }
        }
    }

    Ok(())
}

fn gizmo_facets<'a, 'b>(
    space: &'a mut Space,
    axes: &ProductPuzzleAxes,
) -> Result<Vec<hypershape::FacetId>> {
    let mirror_planes = axes
        .coxeter_matrix
        .mirrors()?
        .cols()
        .filter_map(|mirror_vector| Hyperplane::new(mirror_vector, 0.0));
    let carve_planes = axes
        .orbits
        .iter()
        .filter_map(|orbit| Hyperplane::from_pole(&axes.vectors[orbit.first()]));

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
    axes: &ProductPuzzleAxes,
    seen: &mut TiMask<Axis>,
) -> Vec<(Axis, GroupElementId, Motor)> {
    hypergroup::orbit_collect(
        (init, GroupElementId::IDENTITY, Motor::ident(axes.ndim())),
        axes.group.generators(),
        |_, (ax, e, m), &g| {
            let new_axis = axes.action.act(g, *ax);
            if !seen.contains(new_axis) {
                seen.insert(new_axis);
                let new_elem = axes.group.compose(g, *e);
                let new_motor = axes.group.motor(g) * m;
                Some((new_axis, new_elem, new_motor))
            } else {
                None
            }
        },
    )
}

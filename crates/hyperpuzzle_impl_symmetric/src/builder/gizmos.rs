use std::collections::HashMap;

use eyre::{OptionExt, Result, eyre};
use hypergroup::{
    ConstraintSet, ConstraintSolver, CoxeterMatrix, GroupAction, GroupElementId, IsometryGroup,
};
use hypermath::{
    APPROX, ApproxHashMap, Hyperplane, Vector, VectorRef, approx_collections, pga::Motor,
};
use hyperpuzzle_core::{Axis, Mesh, NameSpecBiMap, PerAxis, PerGizmoFace};
use hyperpuzzle_impl_nd_euclid::GizmoTwist;
use hypershape::{Cut, ElementCutOutput, ElementId, ElementIdConvert, FaceId, FacetId, Space};
use hypuz_notation::{Multiplier, Transform};
use itertools::Itertools;

use crate::{StabilizerFamily, builder::ProductPuzzleAxes};

pub fn build_3d_gizmo<'a>(
    mesh: &mut Mesh,
    gizmo_twists: &mut PerGizmoFace<GizmoTwist>,
    axes: &ProductPuzzleAxes,
    axis_names: &NameSpecBiMap<Axis>,
) -> Result<()> {
    let mut space = Space::new(3)?;
    let gizmo_faces = gizmo_facets(&mut space, axes)?
        .into_iter()
        .map(|f| f.as_element().id())
        .collect_vec();

    let mut seen_axis_vectors = ApproxHashMap::new(APPROX);

    for face_id in gizmo_faces {
        let unfolded_face_id = space.unfold(face_id)?;
        let face = space.get(unfolded_face_id).as_face()?;
        let facet = space.get(unfolded_face_id).as_facet()?;

        let init_axis_vector = facet.hyperplane()?.normal().clone();

        if seen_axis_vectors
            .insert(init_axis_vector.clone(), ())
            .is_some()
        {
            continue; // already handled!
        }

        let vertex_positions = face.vertices_in_order()?.map(|v| v.pos()).collect_vec();

        let orbit = hypergroup::orbit_collect(
            (init_axis_vector, Motor::ident(space.ndim())),
            &axes.coxeter_matrix.generator_motors()?,
            |_, (v, m), g| {
                let mut new_vector = g.transform(v);
                if let approx_collections::hash_map::Entry::Vacant(e) =
                    seen_axis_vectors.entry_with_mut_key(&mut new_vector)
                {
                    e.insert(());
                    Some((new_vector, g * m))
                } else {
                    None
                }
            },
        );

        let axis_from_vector =
            ApproxHashMap::from_iter(APPROX, axes.vectors.iter().map(|(ax, v)| (v.clone(), ax)));

        for (axis_vector, m) in orbit {
            let axis = *axis_from_vector
                .get(axis_vector.clone())
                .ok_or_eyre("bad axis vector")?;
            let transformed_vertex_positions = vertex_positions
                .iter()
                .map(|p| m.transform(p))
                .collect_vec();
            let surface_id = mesh.add_gizmo_surface(&axis_vector)?;
            let range = mesh.add_gizmo_polygon(&transformed_vertex_positions, surface_id)?;
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
    // let mut space = Space::new(4)?;
    // let mirror_planes = axis_symmetry
    //     .mirrors()?
    //     .cols()
    //     .filter_map(|mirror_vector| Hyperplane::new(mirror_vector, 0.0));
    // let carve_planes = axis_orbit_vectors
    //     .into_iter()
    //     .filter_map(Hyperplane::from_pole);

    // let gizmo_polychoron = space.add_folded_shape(mirror_planes, carve_planes)?;
    // let gizmo_polychoron = space.get(gizmo_polychoron);
    // let mut gizmo_cells = gizmo_polychoron
    //     .facets()
    //     .map(|c| c.as_element().id())
    //     .filter(|&c| !gizmo_polychoron.boundary_portals().contains_element(c))
    //     .collect_vec();
    // dbg!(gizmo_polychoron.id(), &gizmo_cells);

    // let mut twists_by_axis = HashMap::<Axis, Vec<(PseudoAxis, f64)>>::new();
    // for (family, distance) in stabilizer_twist_orbits {
    //     twists_by_axis
    //         .entry(family.primary)
    //         .or_default()
    //         .push((family.secondary, *distance));
    // }
    // dbg!(&twists_by_axis);

    // let mut twist_faces: HashMap<Axis, Vec<(ElementId, PseudoAxis, Vector)>> = HashMap::new();

    // // Carve the gizmo polytope
    // for (family, distance) in stabilizer_twist_orbits {
    //     for cell_id in &mut gizmo_cells {
    //         let Some(&facet_axis) = axis_from_vector.get(
    //             space
    //                 .get(*cell_id)
    //                 .as_facet()?
    //                 .hyperplane()?
    //                 .normal()
    //                 .clone(),
    //         ) else {
    //             continue;
    //         };

    //         for (&primary_axis, twists) in &mut twists_by_axis {
    //             if let Some(conj_coset) = solver.solve(&ConstraintSet::from([[
    //                 axes.axis_to_pseudo_axis(primary_axis),
    //                 axes.axis_to_pseudo_axis(facet_axis),
    //             ]])) {
    //                 let right_coset = conj_coset.to_right_coset();
    //                 let axis_twist_faces = twist_faces.entry(facet_axis).or_default();

    //                 // TODO: think through left vs. right coset again
    //                 let subgroup_generator_motors = right_coset
    //                     .subgroup
    //                     .generators
    //                     .into_iter()
    //                     .map(|e| axis_group.motor(e))
    //                     .collect_vec();
    //                 for (secondary_axis, distance) in twists {
    //                     let secondary_axis =
    //                         pseudo_axis_action.act(right_coset.rhs, *secondary_axis);
    //                     let secondary_axis_vector = pseudo_axes[secondary_axis]
    //                         .iter()
    //                         .map(|&a| &axis_vectors[a])
    //                         .sum::<Vector>()
    //                         .rejected_from(&axis_vectors[primary_axis])
    //                         .ok_or_eyre("axis vector cannot be zero")?;
    //                     for transformed_secondary_axis_vector in hypergroup::orbit_geometric(
    //                         &subgroup_generator_motors,
    //                         secondary_axis_vector,
    //                     ) {
    //                         if let Some(h) =
    //                             Hyperplane::new(&transformed_secondary_axis_vector, *distance)
    //                         {
    //                             let mut cut = Cut::carve(h);
    //                             *axis_twist_faces = std::mem::take(axis_twist_faces)
    //                                 .into_iter()
    //                                 .filter_map(|(p, secondary_axis, secondary_axis_vector)| {
    //                                     match cut.cut(&mut space, p) {
    //                                         Err(e) => {
    //                                             warn_fn(e);
    //                                             None
    //                                         }
    //                                         Ok(ElementCutOutput::NonFlush {
    //                                             inside: Some(inside),
    //                                             ..
    //                                         }) => Some((
    //                                             inside,
    //                                             secondary_axis,
    //                                             secondary_axis_vector,
    //                                         )),
    //                                         _ => {
    //                                             warn_fn(eyre!("gizmo face is empty"));
    //                                             None
    //                                         }
    //                                     }
    //                                 })
    //                                 .collect_vec();
    //                             let cut_output = cut.cut(&mut space, *cell_id)?;
    //                             if let Some(remaining_cell) = cut_output.inside() {
    //                                 *cell_id = remaining_cell;
    //                             } else {
    //                                 warn_fn(eyre!("twist gizmo is empty"));
    //                             }
    //                             if let Some(new_face) = cut_output.intersection() {
    //                                 axis_twist_faces.push((
    //                                     new_face,
    //                                     secondary_axis.clone(),
    //                                     transformed_secondary_axis_vector,
    //                                 ));
    //                             } else {
    //                                 // ok. probably not in the fundamental region
    //                             }
    //                         } else {
    //                             warn_fn(eyre!("cannot construct hyperplane for twist gizmo"));
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
    // dbg!(&twist_faces);

    // // Expand each gizmo face
    // let axis_gizmo_surfaces = axis_vectors.try_map_ref(|_, v| mesh.add_gizmo_surface(v))?;
    // for (primary_axis, faces) in twist_faces {
    //     dbg!(&primary_axis, &faces);
    //     let primary_axis_vector = &axis_vectors[primary_axis];

    //     for (face_id, secondary_axis, secondary_axis_vector) in faces {
    //         dbg!(&face_id, &secondary_axis, &secondary_axis_vector);
    //         let unfolded_face_id = space.unfold(face_id)?;
    //         let face = space.get(unfolded_face_id).as_face()?;

    //         let vertex_positions = face.vertices_in_order()?.map(|v| v.pos()).collect_vec();

    //         for (elem, _) in
    //             axis_group.orbit_geometric((primary_axis_vector.clone(), secondary_axis_vector))
    //         {
    //             dbg!(&elem);
    //             let transformed_primary_axis = axis_action.act(elem, primary_axis);
    //             let transformed_secondary_axis = pseudo_axis_action.act(elem, secondary_axis);
    //             let transformed_family = StabilizerFamily {
    //                 primary: transformed_primary_axis,
    //                 secondary: transformed_secondary_axis,
    //             };

    //             let m = axis_group.motor(elem);
    //             let transformed_points = vertex_positions
    //                 .iter()
    //                 .map(|p| m.transform(p))
    //                 .collect_vec();
    //             let range =
    //                 mesh.add_gizmo_polygon(&transformed_points, axis_gizmo_surfaces[primary_axis])?;
    //             mesh.add_gizmo_face(range);
    //             dbg!(
    //                 &transformed_points,
    //                 transform_from_stabilizer_family(&transformed_family)
    //             );
    //             gizmo_twists.push(GizmoTwist {
    //                 axis: primary_axis,
    //                 transform: transform_from_stabilizer_family(&transformed_family),
    //                 multiplier: Multiplier(1),
    //             });
    //         }
    //     }
    // }

    Ok(())
}

fn gizmo_facets<'a, 'b>(
    space: &'a mut Space,
    axes: &ProductPuzzleAxes,
) -> Result<Vec<hypershape::Facet<'a>>> {
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
        .collect())
}

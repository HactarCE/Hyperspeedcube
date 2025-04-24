//! Algorithms for generating twist gizmo geometry.

use std::collections::{HashMap, hash_map};

use eyre::{OptionExt, Result, bail, eyre};
use float_ord::FloatOrd;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use pga::Blade;

use crate::NdEuclidTwistSystemEngineData;

pub(super) fn build_twist_gizmos(
    space: &Space,
    mesh: &mut Mesh,
    twists: &TwistSystem,
    engine_data: &NdEuclidTwistSystemEngineData,
    warn_fn: impl Copy + Fn(eyre::Report),
) -> Result<PerGizmoFace<Twist>> {
    let NdEuclidTwistSystemEngineData {
        axis_vectors,
        twist_transforms,
        gizmo_pole_distances,
        ..
    } = engine_data;

    // Assemble a list of gizmo pole vectors and their associated twists.
    let mut gizmo_poles: PerAxis<Vec<(Vector, Twist)>> = PerAxis::new_with_len(twists.axes.len());
    for (twist, twist_info) in &twists.twists {
        if let Some(pole_distance) = gizmo_pole_distances[twist] {
            // The axis vector is fixed by the twist.
            let axis_vector = &axis_vectors[twist_info.axis];

            let face_normal = if space.ndim() == 4 {
                // Compute the other vector fixed by the twist.
                (|| {
                    let axis_vector = Blade::from_vector(axis_vector);
                    let origin = Blade::origin();
                    Blade::wedge(
                        &twist_transforms[twist].grade_project(2),
                        &Blade::wedge(&origin, &axis_vector)?,
                    )?
                    .antidual(4)?
                    .to_vector()?
                    .normalize()
                })()
                .ok_or_eyre("error computing normal vector for twist gizmo")?
            } else {
                axis_vector.clone()
            };

            let gizmo_pole = face_normal * pole_distance as _;
            gizmo_poles[twist_info.axis].push((gizmo_pole, twist));
        };
    }

    // Build twist gizmos.
    let mut gizmo_face_twists = PerGizmoFace::new();
    if space.ndim() == 3 {
        let gizmo_poles = gizmo_poles.iter_values().flatten().cloned().collect_vec();
        let resulting_gizmo_faces =
            build_3d_gizmo(space, mesh, twists, engine_data, &gizmo_poles, warn_fn)?;
        for (_gizmo_face, twist) in resulting_gizmo_faces {
            gizmo_face_twists.push(twist)?;
        }
    } else if space.ndim() == 4 {
        for (axis, axis_gizmo_poles) in gizmo_poles {
            let resulting_gizmo_faces = build_4d_gizmo(
                space,
                mesh,
                twists,
                engine_data,
                axis,
                axis_gizmo_poles,
                warn_fn,
            )?;
            for (_gizmo_face, twist) in resulting_gizmo_faces {
                gizmo_face_twists.push(twist)?;
            }
        }
    }
    if gizmo_face_twists.len() != mesh.gizmo_face_count {
        bail!("error generating gizmo: face count mismatch");
    }

    Ok(gizmo_face_twists)
}

fn build_3d_gizmo(
    space: &Space,
    mesh: &mut Mesh,
    twists: &TwistSystem,
    engine_data: &NdEuclidTwistSystemEngineData,
    gizmo_poles: &[(Vector, Twist)],
    warn_fn: impl Fn(eyre::Report),
) -> Result<Vec<(GizmoFace, Twist)>> {
    if twists.is_empty() {
        return Ok(vec![]);
    }

    let polyhedron = space.get_primordial_cube()?.id();

    let mut gizmo_surfaces = HashMap::new();
    for (_, twist_info) in &twists.twists {
        let axis = twist_info.axis;
        if let hash_map::Entry::Vacant(e) = gizmo_surfaces.entry(axis) {
            e.insert(mesh.add_gizmo_surface(&engine_data.axis_vectors[axis])?);
        }
    }

    build_gizmo(
        space,
        mesh,
        twists,
        polyhedron.to_element_id(space),
        hypershape::PRIMORDIAL_CUBE_RADIUS,
        gizmo_poles,
        "twist gizmo",
        |twist| gizmo_surfaces[&twists.twists[twist].axis],
        warn_fn,
    )
}

fn build_4d_gizmo(
    space: &Space,
    mesh: &mut Mesh,
    twists: &TwistSystem,
    engine_data: &NdEuclidTwistSystemEngineData,
    axis: Axis,
    gimzo_poles: Vec<(Vector, Twist)>,
    warn_fn: impl Fn(eyre::Report),
) -> Result<Vec<(GizmoFace, Twist)>> {
    use hypershape::flat::*;

    let axis_vector = &engine_data.axis_vectors[axis];
    let axis_name = &twists.axes.names[axis];

    if twists.is_empty() {
        return Ok(vec![]);
    }

    // Cut a primordial polyhedron at the axis.
    let initial_cut_params = CutParams {
        divider: Hyperplane::from_pole(axis_vector).ok_or_eyre("bad axis vector")?,
        inside: PolytopeFate::Remove,
        outside: PolytopeFate::Remove,
    };
    let primordial_cube = space.get_primordial_cube()?;
    let polyhedron = match Cut::new(space, initial_cut_params).cut(primordial_cube)? {
        hypershape::ElementCutOutput::Flush => bail!("bad axis vector"),
        hypershape::ElementCutOutput::NonFlush { intersection, .. } => {
            intersection.ok_or_eyre("bad axis vector")?
        }
    };

    let min_radius = axis_vector.mag();

    let gizmo_surface = mesh.add_gizmo_surface(axis_vector)?;

    build_gizmo(
        space,
        mesh,
        twists,
        polyhedron,
        min_radius,
        &gimzo_poles,
        &format!("twist gizmo for axis {axis_name:?}"),
        |_| gizmo_surface,
        warn_fn,
    )
}

fn build_gizmo(
    space: &Space,
    mesh: &mut Mesh,
    twists: &TwistSystem,
    mut polyhedron: ElementId,
    min_radius: Float,
    gizmo_poles: &[(Vector, Twist)],
    gizmo_name: &str,
    mut get_gizmo_surface: impl FnMut(Twist) -> u32,
    warn_fn: impl Fn(eyre::Report),
) -> Result<Vec<(GizmoFace, Twist)>> {
    use hypershape::flat::*;

    // Cut a primordial cube for the twist gizmo.
    let max_pole_radius = gizmo_poles
        .iter()
        .map(|(v, _)| v.mag())
        .max_by_key(|&x| FloatOrd(x))
        .unwrap_or(0.0);
    let primordial_face_radius = Float::max(max_pole_radius, min_radius) * 2.0; // can be any number greater than 1
    for axis in 0..space.ndim() {
        for distance in [-1.0, 1.0] {
            let cut_normal = Vector::unit(axis) * distance;
            let cut_plane =
                Hyperplane::new(cut_normal, primordial_face_radius).ok_or_eyre("bad hyperplane")?;
            let mut cut = Cut::carve(space, cut_plane);
            let result = cut.cut(polyhedron)?.inside();
            polyhedron = result.ok_or_eyre("error cutting primordial cube for twist gizmo")?;
        }
    }

    // Cut a face for each twist.
    let mut face_polygons: Vec<(ElementId, Twist)> = vec![];
    for (new_pole, new_twist) in gizmo_poles {
        let Some(cut_plane) = Hyperplane::from_pole(new_pole) else {
            let new_twist_name = &twists.names[*new_twist];
            warn_fn(eyre!(
                "bad facet pole for twist {new_twist_name:?} on twist gizmo",
            ));
            continue;
        };
        let mut cut = Cut::carve(space, cut_plane);

        let mut new_face_polygons = vec![];

        // Cut each existing facet.
        for (f, twist) in face_polygons {
            match cut.cut(f)? {
                hypershape::ElementCutOutput::Flush => {
                    let twist_name = &twists.names[twist];
                    let new_twist_name = &twists.names[*new_twist];
                    warn_fn(eyre!(
                        "twists {twist_name:?} and {new_twist_name:?} overlap on twist gizmo",
                    ));
                }
                hypershape::ElementCutOutput::NonFlush {
                    inside: Some(new_f),
                    ..
                } => new_face_polygons.push((new_f, twist)),
                _ => {
                    let twist_name = &twists.names[twist];
                    let new_twist_name = &twists.names[*new_twist];
                    warn_fn(eyre!(
                        "twist {twist_name:?} is eclipsed by {new_twist_name:?} on twist gizmo",
                    ));
                }
            }
        }

        // Cut the polyhedron.
        let polyhedron_cut_output = cut.cut(polyhedron)?;
        match polyhedron_cut_output {
            hypershape::ElementCutOutput::Flush => bail!("polytope is flush"),
            hypershape::ElementCutOutput::NonFlush {
                inside,
                outside: _,
                intersection,
            } => {
                // Update the polyhedron.
                polyhedron = inside.ok_or_eyre("twist gizmo becomes null")?;

                // Add the new facet.
                match intersection {
                    Some(new_facet) => new_face_polygons.push((new_facet, *new_twist)),
                    None => {
                        let new_twist_name = &twists.names[*new_twist];
                        warn_fn(eyre!("twist {new_twist_name:?} is eclipsed on twist gizmo"));
                    }
                }
            }
        }

        face_polygons = new_face_polygons;
    }

    if space.get(polyhedron).boundary().count() > face_polygons.len() {
        let r = primordial_face_radius;
        warn_fn(eyre!(
            "{gizmo_name} is infinite; it has been bounded with a radius-{r} cube",
        ));
    }

    // Add vertices to the mesh and record a map from vertex IDs in `space`
    // to vertex IDs in `mesh`.
    let vertex_map: HashMap<(VertexId, u32), u32> = face_polygons
        .iter()
        .flat_map(|&(polygon, twist)| {
            let surface = get_gizmo_surface(twist);
            space.get(polygon).vertex_set().map(move |v| (v, surface))
        })
        .map(|(vertex, surface)| {
            let old_id = vertex.id();
            let new_id = mesh.add_gizmo_vertex(vertex.pos(), surface)?;
            eyre::Ok(((old_id, surface), new_id))
        })
        .try_collect()?;

    let mut resulting_gizmo_faces = vec![];

    // Generate mesh for face polygons and edges.
    for (face_polygon, twist) in face_polygons {
        let surface = get_gizmo_surface(twist);

        let face_polygon = space.get(face_polygon).as_face()?;

        let triangles_start = mesh.triangle_count() as u32;
        let edges_start = mesh.edge_count() as u32;

        for edge in face_polygon.edge_endpoints()? {
            mesh.edges
                .push(edge.map(|v| vertex_map[&(v.id(), surface)]));
        }
        for tri in face_polygon.triangles()? {
            mesh.triangles.push(tri.map(|v| vertex_map[&(v, surface)]));
        }

        let triangles_end = mesh.triangle_count() as u32;
        let edges_end = mesh.edge_count() as u32;
        let new_gizmo_face =
            mesh.add_gizmo_face(triangles_start..triangles_end, edges_start..edges_end)?;
        resulting_gizmo_faces.push((new_gizmo_face, twist));
    }

    Ok(resulting_gizmo_faces)
}

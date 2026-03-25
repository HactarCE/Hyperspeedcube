use eyre::{OptionExt, Result};
use hypermath::{APPROX, ApproxHashMap, Hyperplane, Vector};
use hyperpuzzle_core::{Axis, Mesh, PerGizmoFace};
use hyperpuzzle_impl_nd_euclid::GizmoTwist;
use hypershape::{Cut, Space, ToElementId};
use hypuz_notation::{Multiplier, Transform};
use itertools::Itertools;

pub fn build_3d_gizmo(
    mesh: &mut Mesh,
    faces: &[(Vector, Axis, Transform)],
    gizmo_twists: &mut PerGizmoFace<GizmoTwist>,
) -> Result<()> {
    let space = Space::new(3);
    let mut gizmo_polytope = space
        .add_primordial_cube(hypershape::PRIMORDIAL_CUBE_RADIUS)?
        .id()
        .to_element_id(&space);
    for (vector, _, _) in faces {
        let cut_plane = Hyperplane::from_pole(vector).ok_or_eyre("bad axis vector")?; // TODO: warn instead of error
        gizmo_polytope = Cut::carve(&space, cut_plane)?
            .cut(gizmo_polytope)?
            .inside()
            .ok_or_eyre("twist gizmo does not exist")?; // TODO: warn instead of error
    }

    let gizmo_faces_by_vector = ApproxHashMap::from_iter(
        APPROX,
        faces
            .iter()
            .map(|(v, axis, transform)| (v.clone(), (*axis, transform))),
    );

    for face in space.get(gizmo_polytope).face_set() {
        let face_pole = face.as_element().as_facet()?.hyperplane()?.pole();
        let Some(&(axis, transform)) = gizmo_faces_by_vector.get(face_pole.clone()) else {
            continue; // TODO: warn
        };

        let vertex_positions = face.vertices_in_order()?.map(|v| v.pos()).collect_vec();
        let surface_id = mesh.add_gizmo_surface(&face_pole)?;
        let range = mesh.add_gizmo_polygon(&vertex_positions, surface_id)?;
        mesh.add_gizmo_face(range)?;
        gizmo_twists.push(GizmoTwist {
            axis,
            transform: transform.clone(),
            multiplier: Multiplier(1),
        })?;
    }

    Ok(())
}

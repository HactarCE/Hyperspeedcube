use std::collections::HashSet;

use eyre::{bail, eyre, OptionExt, Result};
use float_ord::FloatOrd;
use hypermath::prelude::*;
use hypershape::{Cut, CutParams, ElementId, PolytopeFate, Space};

use super::*;

/// Twist gizmo for a 3D puzzle or an axis of a 4D puzzle.
#[derive(Debug)]
pub struct TwistGizmoPolyhedron {
    /// Vertices of the polyhedron.
    pub(super) verts: Vec<[f32; 4]>,
    /// Edges of the polyhedron.
    pub edges: Vec<[u32; 2]>,
    /// Faces of the polyhedron, each of which corresponds to a twist on the
    /// axis. The `u32`s are indices into the vertex array. Each polygon is
    /// specified in counterclockwise order.
    pub faces: Vec<(Twist, Vec<u32>)>,
    /// Axis vector in 4D space.
    pub axis_vector: [f32; 4],
}
impl TwistGizmoPolyhedron {
    /// Returns the vertices of the gizmo, transformed according to `transform`
    /// and `shrink_factor`.
    pub fn compute_vertex_positions(
        &self,
        transform: pga::Motor,
        shrink_factor: f32,
    ) -> Vec<[f32; 4]> {
        let m = transform.euclidean_rotation_matrix();
        let rows: [[f32; 4]; 4] = [0, 1, 2, 3].map(|j| [0, 1, 2, 3].map(|i| m.get(i, j) as f32));
        self.verts
            .iter()
            .map(|&v| {
                // Apply the shrink factor.
                let v: [f32; 4] = std::array::from_fn(|i| {
                    hypermath::util::lerp(v[i], self.axis_vector[i], shrink_factor)
                });
                // Apply the transform.
                std::array::from_fn(|j| rows[j].iter().zip(v).map(|(mx, vx)| mx as f32 * vx).sum())
            })
            .collect()
    }

    /// Returns the center of the gizmo polyhedron.
    pub fn center(&self, transform: pga::Motor) -> Vector {
        let v = Vector::from_iter(self.axis_vector.map(|x| x as Float));
        transform.transform_point(v)
    }

    /// Constructs a twist gizmo polyhedron given a set of poles.
    pub fn new<'a>(
        space: &Space,
        axis_vector: impl VectorRef,
        poles: Vec<(Vector, Twist)>,
        axis_name: &str,
        mut get_twist_name: impl FnMut(Twist) -> &'a str,
        mut warn_fn: impl FnMut(eyre::Report),
    ) -> Result<Self> {
        // Cut a primordial polyhedron.
        let initial_cut_params = CutParams {
            divider: Hyperplane::from_pole(&axis_vector).ok_or_eyre("bad axis vector")?,
            inside: PolytopeFate::Remove,
            outside: PolytopeFate::Remove,
        };
        let primordial_cube = space.get_primordial_cube()?;
        let mut polyhedron = match Cut::new(space, initial_cut_params).cut(primordial_cube)? {
            hypershape::ElementCutOutput::Flush => bail!("bad axis vector"),
            hypershape::ElementCutOutput::NonFlush { intersection, .. } => {
                intersection.ok_or_eyre("bad axis vector")?
            }
        };

        // Cut a primordial cube for the twist gizmo.
        let max_pole_radius = poles
            .iter()
            .map(|(v, _)| v.mag())
            .max_by_key(|&x| FloatOrd(x))
            .unwrap_or(0.0);
        let axis_radius = axis_vector.mag();
        let primordial_face_radius = Float::max(max_pole_radius, axis_radius) * 2.0; // can be any number greater than 1
        for axis in 0..space.ndim() {
            for distance in [-1.0, 1.0] {
                let cut_normal = Vector::unit(axis) * distance;
                let cut_plane = Hyperplane::new(cut_normal, primordial_face_radius)
                    .ok_or_eyre("bad hyperplane")?;
                let mut cut = Cut::carve(space, cut_plane);
                let result = cut.cut(polyhedron)?.inside();
                polyhedron = result.ok_or_eyre("error cutting primordial cube for twist gizmo")?;
            }
        }

        // Cut a face for each twist.
        let mut face_polygons: Vec<(ElementId, Twist)> = vec![];
        for (new_pole, new_twist) in poles {
            let Some(cut_plane) = Hyperplane::from_pole(new_pole) else {
                warn_fn(eyre!(
                    "bad facet pole for twist {:?} on twist gizmo",
                    get_twist_name(new_twist),
                ));
                continue;
            };
            let mut cut = Cut::carve(space, cut_plane);

            let mut new_face_polygons = vec![];

            // Cut each existing facet.
            for (f, twist) in face_polygons {
                match cut.cut(f)? {
                    hypershape::ElementCutOutput::Flush => warn_fn(eyre!(
                        "twists {:?} and {:?} overlap on twist gizmo",
                        get_twist_name(twist),
                        get_twist_name(new_twist),
                    )),
                    hypershape::ElementCutOutput::NonFlush {
                        inside: Some(new_f),
                        ..
                    } => new_face_polygons.push((new_f, twist)),
                    _ => warn_fn(eyre!(
                        "twist {:?} is eclipsed by {:?} on twist gizmo",
                        get_twist_name(twist),
                        get_twist_name(new_twist)
                    )),
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
                        Some(new_facet) => new_face_polygons.push((new_facet, new_twist)),
                        None => warn_fn(eyre!(
                            "twist {:?} is eclipsed on twist gizmo",
                            get_twist_name(new_twist),
                        )),
                    }
                }
            }

            face_polygons = new_face_polygons;
        }

        if space.get(polyhedron).boundary().count() > face_polygons.len() {
            let r = primordial_face_radius;
            warn_fn(eyre!(
                "twist gizmo for axis {axis_name:?} is infinite; \
                 it has been bounded with a radius-{r} cube",
            ));
        }

        // Generate mesh for face polygons. Map from `VertexID`s in `space` to
        // `u32` indices into `verts`.
        let mut vertex_map = crate::util::LazyIdMap::new(0, |i| i + 1);
        let mut edges = HashSet::new();
        let mut faces = vec![];
        for (face_polygon, twist) in face_polygons {
            let mut face = vec![];
            for edge in space.get(face_polygon).as_face()?.edge_endpoints()? {
                let mut edge = edge.map(|v| vertex_map.get_or_insert(v.id()));
                face.push(edge[0]); // just one vertex
                edge.sort();
                edges.insert(edge);
            }
            faces.push((twist, face));
        }

        let verts = vertex_map
            .keys()
            .iter()
            .map(|&k| vector_to_f32_4(space.get(k).pos()))
            .collect();
        let edges = edges.into_iter().collect();
        let axis_vector = vector_to_f32_4(axis_vector);

        Ok(Self {
            verts,
            edges,
            faces,
            axis_vector,
        })
    }
}

fn vector_to_f32_4(v: impl VectorRef) -> [f32; 4] {
    [0, 1, 2, 3].map(|i| v.get(i) as f32)
}

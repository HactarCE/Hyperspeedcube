use std::collections::{hash_map, HashMap};

use eyre::{bail, ensure, eyre, OptionExt, Result, WrapErr};
use float_ord::FloatOrd;
use hypermath::collections::approx_hashmap::FloatHash;
use hypermath::collections::{ApproxHashMap, ApproxHashMapKey, IndexOutOfRange};
use hypermath::prelude::*;
use hypershape::{ElementId, Space, ToElementId};
use itertools::Itertools;
use pga::{Blade, Motor};

use super::{AxisSystemBuilder, CustomOrdering, NameSet};
use crate::builder::NamingScheme;
use crate::puzzle::{Axis, PerTwist, Twist};
use crate::{
    AxisInfo, GizmoFace, Mesh, PerAxis, PerGizmoFace, PuzzleDevData, PuzzleElement, TwistInfo,
};

/// Twist during puzzle construction.
#[derive(Debug, Clone)]
pub struct TwistBuilder {
    /// Axis that is twisted.
    pub axis: Axis,
    /// Transform to apply to pieces.
    pub transform: Motor,
    /// Value in the quarter-turn metric (or its contextual equivalent).
    pub qtm: usize,
    /// Distance of the pole for the corresponding facet in the 4D facet gizmo.
    pub gizmo_pole_distance: Option<f32>,
}
impl TwistBuilder {
    /// Canonicalizes the twist.
    pub fn canonicalize(self) -> Result<Self> {
        let transform = self
            .transform
            .canonicalize_up_to_180()
            .ok_or(BadTwist::BadTransform)?;
        Ok(Self { transform, ..self })
    }
    /// Returns the key used to hash or look up the twist.
    pub fn key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform)
    }
    /// Returns the key used to look up the reverse twist.
    pub fn rev_key(&self) -> Result<TwistKey, BadTwist> {
        TwistKey::new(self.axis, &self.transform.reverse())
    }
}

/// Unique key for a twist.
#[derive(Debug, Clone)]
pub struct TwistKey {
    /// Axis that is twisted.
    axis: Axis,
    /// Transform to apply to pieces.
    transform: Motor,
}
impl ApproxHashMapKey for TwistKey {
    type Hash = (Axis, <Motor as ApproxHashMapKey>::Hash);

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        (self.axis, self.transform.approx_hash(float_hash_fn))
    }
}
impl TwistKey {
    /// Constructs a twist key from an axis and a transform, which does not need
    /// to be canonicalized.
    pub fn new(axis: Axis, transform: &Motor) -> Result<Self, BadTwist> {
        let transform = transform
            .canonicalize_up_to_180()
            .ok_or(BadTwist::BadTransform)?;
        Ok(Self { axis, transform })
    }
}

/// Twist system being constructed.
#[derive(Debug, Default)]
pub struct TwistSystemBuilder {
    /// Axis system being constructed.
    pub axes: AxisSystemBuilder,

    /// Twist data (not including name).
    by_id: PerTwist<TwistBuilder>,
    /// Map from twist data to twist ID for each axis.
    ///
    /// Does not include inverses.
    data_to_id: ApproxHashMap<TwistKey, Twist>,
    /// User-specified twist names.
    pub names: NamingScheme<Twist>,
}
impl TwistSystemBuilder {
    /// Constructs a empty twist system with a given axis system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether there are no twists in the twist system.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Returns the number of twists in the twist system.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Returns the twist axes in canonical alphabetical order.
    pub fn alphabetized(&self) -> Vec<Twist> {
        let mut ordering =
            CustomOrdering::with_len(self.len()).expect("error constructing default ordering");
        ordering.sort_by_name(self.names.ids_to_names());
        ordering.ids_in_order().to_vec()
    }

    /// Adds a new twist.
    pub fn add(&mut self, data: TwistBuilder) -> Result<Result<Twist, BadTwist>> {
        let data = data.canonicalize()?;
        let key = data.key()?;

        // Reject the identity twist.
        if data.transform.is_ident() {
            return Ok(Err(BadTwist::Identity));
        }

        // Check that there is not already an identical twist.
        if let Some(&id) = self.data_to_id.get(&key) {
            let name = self
                .names
                .get(id)
                .and_then(|name| name.canonical_name())
                .unwrap_or_default();
            return Ok(Err(BadTwist::DuplicateTwist { id, name }));
        }

        let id = self.by_id.push(data)?;
        self.data_to_id.insert(key, id);

        Ok(Ok(id))
    }
    /// Adds a new twist and assigns it a name.
    ///
    /// If the twist is invalid, `warn_fn` is called with info about what went
    /// wrong and no twist is created.
    pub fn add_named(
        &mut self,
        data: TwistBuilder,
        name: NameSet,
        warn_fn: impl Fn(String),
    ) -> Result<Option<Twist>> {
        let id = match self.add(data)? {
            Ok(ok) => ok,
            Err(err) => {
                warn_fn(err.to_string());
                return Ok(None);
            }
        };
        self.names
            .set_name(id, Some(name), |e| warn_fn(e.to_string()));
        Ok(Some(id))
    }

    /// Returns a reference to a twist by ID, or an error if the ID is out of
    /// range.
    pub fn get(&self, id: Twist) -> Result<&TwistBuilder, IndexOutOfRange> {
        self.by_id.get(id)
    }

    /// Returns a twist ID from its axis and transform.
    pub fn data_to_id(&self, key: &TwistKey) -> Option<Twist> {
        self.data_to_id.get(key).copied()
    }

    /// Returns the inverse of a twist, or an error if the ID is out of range.
    pub fn inverse(&self, id: Twist) -> Result<Option<Twist>> {
        Ok(self.data_to_id(&self.get(id)?.rev_key()?))
    }

    /// Finalizes the axis system and twist system, and validates them to check
    /// for errors in the definition.
    pub fn build(
        &self,
        space: &Space,
        mesh: &mut Mesh,
        dev_data: &mut PuzzleDevData,
        warn_fn: impl Copy + Fn(eyre::Report),
    ) -> Result<(PerAxis<AxisInfo>, PerTwist<TwistInfo>, PerGizmoFace<Twist>)> {
        // Assemble list of axes.
        let mut axes = PerAxis::new();
        let mut axis_id_map = HashMap::new(); // map from old ID to new ID
        for (old_id, (name_set, _display)) in super::iter_autonamed(
            &self.axes.names,
            &self.axes.ordering,
            crate::util::iter_uppercase_letter_names(),
        ) {
            let old_axis = self.axes.get(old_id)?;
            let vector = old_axis.vector().clone();
            let layers = old_axis
                .build_layers()
                .wrap_err_with(|| format!("building axis {name_set:?}"))?;
            let mut string_set = name_set.string_set()?;
            ensure!(!string_set.is_empty(), "axis is missing canonical name");
            let new_id = axes.push(AxisInfo {
                name: string_set.remove(0),
                aliases: string_set, // all except first
                vector,
                layers,
            })?;

            axis_id_map.insert(old_id, new_id);
        }

        let axis_id_map_reverse: HashMap<Axis, Axis> =
            axis_id_map.iter().map(|(&k, &v)| (v, k)).collect();

        // Assemble list of twists.
        let mut twists = PerTwist::new();
        let mut twist_id_map = HashMap::new(); // map from old ID to new ID
        let mut gizmo_twists: PerAxis<Vec<(Vector, Twist)>> = axes.map_ref(|_, _| vec![]);
        for old_id in self.alphabetized() {
            let old_twist = self.get(old_id)?;
            let axis = *axis_id_map.get(&old_twist.axis).ok_or_eyre("bad axis ID")?;

            let (name, aliases);
            match self.names.get(old_id) {
                Some(name_set) => {
                    let mut string_set = name_set.string_set()?;
                    ensure!(!string_set.is_empty(), "twist is missing canonical name");
                    name = string_set.remove(0);
                    aliases = string_set; // all except first
                }
                None => {
                    name = (old_id.0 + 1).to_string(); // 1-indexed
                    aliases = vec![];
                }
            };

            let new_id = twists.push(TwistInfo {
                name,
                aliases,
                qtm: old_twist.qtm,
                axis,
                transform: old_twist.transform.clone(),
                opposite: None,    // will be assigned later
                reverse: Twist(0), // will be assigned later
            })?;
            twist_id_map.insert(old_id, new_id);

            if let Some(pole_distance) = old_twist.gizmo_pole_distance {
                // The axis vector is fixed by the twist.
                let axis_vector = &axes[axis].vector;

                let face_normal = if space.ndim() == 4 {
                    // Compute the other vector fixed by the twist.
                    (|| {
                        let axis_vector = Blade::from_vector(4, axis_vector);
                        let origin = Blade::origin(4);
                        Blade::wedge(
                            &old_twist.transform.grade_project(2),
                            &Blade::wedge(&origin, &axis_vector)?,
                        )?
                        .antidual()
                        .to_vector()?
                        .normalize()
                    })()
                    .ok_or_eyre("error computing normal vector for twist gizmo")?
                } else {
                    axis_vector.clone()
                };

                let gizmo_pole = face_normal * pole_distance as _;
                gizmo_twists[axis].push((gizmo_pole, new_id));
            };

            // TODO: check that transform keeps layer manifolds fixed
        }
        // TODO: assign opposite twists.

        // Build twist gizmos.
        let mut gizmo_face_twists = PerGizmoFace::new();
        if space.ndim() == 3 {
            let gizmo_twists = gizmo_twists.iter_values().flatten().cloned().collect_vec();
            let resulting_gizmo_faces =
                Self::build_3d_gizmo(space, mesh, &gizmo_twists, &twists, &axes, warn_fn)?;
            for (_gizmo_face, twist) in resulting_gizmo_faces {
                gizmo_face_twists.push(twist)?;
            }
        } else if space.ndim() == 4 {
            for (axis, axis_twists) in gizmo_twists {
                let resulting_gizmo_faces =
                    Self::build_4d_gizmo(space, mesh, &axes[axis], &axis_twists, &twists, warn_fn)?;
                for (_gizmo_face, twist) in resulting_gizmo_faces {
                    gizmo_face_twists.push(twist)?;
                }
            }
        }
        if gizmo_face_twists.len() != mesh.gizmo_face_count {
            bail!("error generating gizmo: face count mismatch");
        }

        // Assign reverse twists.
        let mut twists_without_reverse = vec![];
        for (id, twist) in &mut twists {
            match self
                .data_to_id(&TwistKey::new(
                    *axis_id_map_reverse
                        .get(&twist.axis)
                        .ok_or_eyre("twist not in reverse map")?,
                    &twist.transform.reverse(),
                )?)
                .and_then(|old_id| twist_id_map.get(&old_id))
            {
                Some(&reverse_twist) => twist.reverse = reverse_twist,
                None => twists_without_reverse.push(id),
            }
        }
        if let Some(&id) = twists_without_reverse.first() {
            let name = &twists.get(id)?.name;
            warn_fn(eyre!(
                "some twists (such as {name:?}) have no reverse twist; \
                one was autogenerated for it, but you should include \
                one in the puzzle definition"
            ));
        }
        for id in twists_without_reverse {
            let new_twist_id = twists.next_idx()?;
            let twist = twists.get_mut(id)?;
            twist.reverse = new_twist_id;
            let rev_twist_name = |original| format!("<reverse of {original:?}>");
            let new_twist_info = TwistInfo {
                name: rev_twist_name(&twist.name),
                aliases: twist.aliases.iter().map(rev_twist_name).collect(),
                qtm: twist.qtm,
                axis: twist.axis,
                transform: twist.transform.reverse(),
                opposite: None,
                reverse: id,
            };
            twists.push(new_twist_info)?;
        }

        dev_data
            .orbits
            .extend(self.axes.axis_orbits.iter().map(|dev_orbit| {
                dev_orbit.map(|old_id| axis_id_map.get(&old_id).copied().map(PuzzleElement::Axis))
            }));

        Ok((axes, twists, gizmo_face_twists))
    }

    fn build_3d_gizmo(
        space: &Space,
        mesh: &mut Mesh,
        twists: &[(Vector, Twist)],
        twist_infos: &PerTwist<TwistInfo>,
        axis_infos: &PerAxis<AxisInfo>,
        warn_fn: impl Fn(eyre::Report),
    ) -> Result<Vec<(GizmoFace, Twist)>> {
        if twists.is_empty() {
            return Ok(vec![]);
        }

        let polyhedron = space.get_primordial_cube()?.id();

        let mut gizmo_surfaces = HashMap::new();
        for (_, twist) in twists {
            let axis = twist_infos[*twist].axis;
            let axis_info = &axis_infos[axis];
            if let hash_map::Entry::Vacant(e) = gizmo_surfaces.entry(axis) {
                e.insert(mesh.add_gizmo_surface(&axis_info.vector)?);
            }
        }

        Self::build_gizmo(
            space,
            polyhedron.to_element_id(space),
            crate::PRIMORDIAL_CUBE_RADIUS,
            mesh,
            twists,
            "twist gizmo",
            |twist| &twist_infos[twist].name,
            |twist| gizmo_surfaces[&twist_infos[twist].axis],
            warn_fn,
        )
    }
    fn build_4d_gizmo(
        space: &Space,
        mesh: &mut Mesh,
        axis: &AxisInfo,
        twists: &[(Vector, Twist)],
        twist_infos: &PerTwist<TwistInfo>,
        warn_fn: impl Fn(eyre::Report),
    ) -> Result<Vec<(GizmoFace, Twist)>> {
        use hypershape::flat::*;

        if twists.is_empty() {
            return Ok(vec![]);
        }

        // Cut a primordial polyhedron at the axis.
        let initial_cut_params = CutParams {
            divider: Hyperplane::from_pole(&axis.vector).ok_or_eyre("bad axis vector")?,
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

        let min_radius = axis.vector.mag();

        let gizmo_surface = mesh.add_gizmo_surface(&axis.vector)?;

        Self::build_gizmo(
            space,
            polyhedron,
            min_radius,
            mesh,
            twists,
            &format!("twist gizmo for axis {}", &axis.name),
            |twist| &twist_infos[twist].name,
            |_| gizmo_surface,
            warn_fn,
        )
    }
    fn build_gizmo<'a>(
        space: &Space,
        mut polyhedron: ElementId,
        min_radius: Float,
        mesh: &mut Mesh,
        twists: &[(Vector, Twist)],
        gizmo_name: &str,
        mut get_twist_name: impl FnMut(Twist) -> &'a str,
        mut get_gizmo_surface: impl FnMut(Twist) -> u32,
        warn_fn: impl Fn(eyre::Report),
    ) -> Result<Vec<(GizmoFace, Twist)>> {
        use hypershape::flat::*;

        // Cut a primordial cube for the twist gizmo.
        let max_pole_radius = twists
            .iter()
            .map(|(v, _)| v.mag())
            .max_by_key(|&x| FloatOrd(x))
            .unwrap_or(0.0);
        let primordial_face_radius = Float::max(max_pole_radius, min_radius) * 2.0; // can be any number greater than 1
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
        for (new_pole, new_twist) in twists {
            let Some(cut_plane) = Hyperplane::from_pole(new_pole) else {
                let new_twist_name = get_twist_name(*new_twist);
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
                        let twist_name = get_twist_name(twist);
                        let new_twist_name = get_twist_name(*new_twist);
                        warn_fn(eyre!(
                            "twists {twist_name:?} and {new_twist_name:?} overlap on twist gizmo",
                        ));
                    }
                    hypershape::ElementCutOutput::NonFlush {
                        inside: Some(new_f),
                        ..
                    } => new_face_polygons.push((new_f, twist)),
                    _ => {
                        let twist_name = get_twist_name(twist);
                        let new_twist_name = get_twist_name(*new_twist);
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
                            let new_twist_name = get_twist_name(*new_twist);
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
}

/// Error indicating a bad twist.
#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum BadTwist {
    #[error("twist transform cannot be identity")]
    Identity,
    #[error("identical twist already exists with ID {id} and name {name:?}")]
    DuplicateTwist { id: Twist, name: String },
    #[error("bad twist transform")]
    BadTransform,
}

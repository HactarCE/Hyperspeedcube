#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::ops::Range;
use std::sync::{Arc, Weak};

use eyre::{bail, OptionExt, Result};
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::{ShapeBuilder, TwistSystemBuilder};
use crate::builder::AxisSystemBuilder;
use crate::puzzle::*;

/// Puzzle being constructed.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Puzzle ID.
    pub id: String,
    /// Name of the puzzle.
    pub name: String,

    /// Symmetry group of the whole puzzle.
    pub symmetry: Option<SchlafliSymbol>,

    /// Shape of the puzzle.
    pub shape: Arc<Mutex<ShapeBuilder>>,
    /// Twist system of the puzzle.
    pub twists: Arc<Mutex<TwistSystemBuilder>>,
}
impl PuzzleBuilder {
    /// Returns the nubmer of dimensions of the underlying space the puzzle is
    /// built in. Equivalent to `self.shape.lock().space.ndim()`.
    pub fn ndim(&self) -> u8 {
        self.space().lock().ndim()
    }
    /// Returns the underlying space the puzzle is built in. Equivalent to
    /// `self.shape.lock().space`
    pub fn space(&self) -> Arc<Mutex<Space>> {
        Arc::clone(&self.shape.lock().space)
    }
    /// Returns the axis system of the puzzle. Equivalent to
    /// `self.twists.lock().axes`.
    pub fn axis_system(&self) -> Arc<Mutex<AxisSystemBuilder>> {
        Arc::clone(&self.twists.lock().axes)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces, stickers, etc.
    pub fn build(&self) -> Result<Arc<Puzzle>> {
        let name = self.name.clone();
        let id = self.id.clone();

        let space_arc = Arc::new(Mutex::new(Space::new(self.ndim())));
        let shape_mutex = self.shape.lock().clone(&space_arc)?;
        let shape = shape_mutex.lock();
        let twist_system_mutex = self.twists.lock().clone(&space_arc)?;
        let twist_system = twist_system_mutex.lock();
        // Take `space` out of the `Arc<Mutex<T>>`.
        let space = std::mem::replace(&mut *space_arc.lock(), Space::new(self.ndim()));
        drop(space_arc); // Don't use that new space! It's dead to us.
        let ndim = space.ndim();

        let mut mesh = Mesh::new_empty(self.ndim());
        mesh.color_count = shape.colors.len();

        // Only colored surfaces have an entry in `surface_colors`.
        let mut surface_colors: ApproxHashMap<Hyperplane, Color> = ApproxHashMap::new();
        for &id in shape.colors.ordering.ids_in_order() {
            for surface in shape.colors.get(id)?.surfaces() {
                surface_colors.insert(surface.clone(), id);
            }
        }
        // All surfaces have an entry in `surface_to_facet`.
        let mut surface_to_facet: ApproxHashMap<Hyperplane, Facet> = ApproxHashMap::new();

        // As we construct the mesh, we'll renumber all the pieces and stickers
        // to exclude inactive ones.
        let mut pieces = PerPiece::<PieceInfo>::new();
        let mut stickers = PerSticker::<StickerInfo>::new();
        let mut facets = PerFacet::<TempFacetData>::new();
        let colors = super::iter_autonamed(
            &shape.colors.names,
            &shape.colors.ordering,
            crate::util::iter_uppercase_letter_names(),
        )
        .map(|(id, name)| {
            let default_color = shape.colors.get(id)?.default_color.clone();
            eyre::Ok(ColorInfo {
                name,
                default_color,
            })
        })
        .try_collect()?;

        let mut simplices = SimplicialComplex::new(&space);

        // Construct the mesh for each active piece.
        for old_piece_id in shape.active_pieces.iter() {
            let piece = &shape.pieces[old_piece_id];

            let piece_centroid = simplices.centroid(piece.polytope)?.center();

            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type: PieceType(0), // TODO: piece types
                centroid: piece_centroid.clone(),
                polytope: piece.polytope,
            })?;

            // Add stickers to the mesh sorted by color. It's important that
            // internal stickers are processed last, so that they are all in a
            // consecutive range for `piece_internals_index_ranges`.
            let facets_of_piece = space[piece.polytope].boundary()?.clone();
            let mut stickers_of_piece: Vec<TempStickerData> = facets_of_piece
                .iter()
                .map(|polytope| {
                    // Select the orientation of the facet hyperplane such that
                    // the centroid of the piece is on the inside.
                    let mut plane = space.hyperplane_of_facet(polytope)?;
                    if plane.location_of_point(&piece_centroid) == PointWhichSide::Outside {
                        plane = plane.flip();
                    }

                    let color = *surface_colors.get(&plane).unwrap_or(&Color::INTERNAL);
                    eyre::Ok(TempStickerData {
                        polytope,
                        plane,
                        color,
                    })
                })
                // Skip internals for 4D+.
                .filter_ok(|data| ndim < 4 || data.color != Color::INTERNAL)
                .try_collect()?;
            // Sort the stickers, as mentioned above.
            stickers_of_piece.sort();

            let sticker_shrink_vectors = compute_sticker_shrink_vectors(
                &space,
                &mut simplices,
                piece.polytope,
                &stickers_of_piece,
            )?;

            let mut piece_internals_indices_start = None;

            for sticker in stickers_of_piece {
                if sticker.color != Color::INTERNAL {
                    let sticker_id = stickers.push(StickerInfo {
                        piece: piece_id,
                        color: sticker.color,
                    })?;
                    pieces[piece_id].stickers.push(sticker_id);
                }

                let sticker_centroid = simplices.centroid(sticker.polytope)?;
                let sticker_plane = sticker.plane;
                let facet_id = match surface_to_facet.entry(sticker_plane.clone()) {
                    hash_map::Entry::Occupied(e) => *e.get(),
                    hash_map::Entry::Vacant(e) => {
                        let facet_id = facets.push(TempFacetData::new(&sticker_plane)?)?;
                        *e.insert(facet_id)
                    }
                };

                facets[facet_id].centroid += sticker_centroid;

                let (polygon_index_range, triangles_index_range, edges_index_range) =
                    build_shape_polygons(
                        &space,
                        &mut mesh,
                        &mut simplices,
                        &sticker_shrink_vectors,
                        sticker.polytope,
                        &piece_centroid,
                        piece_id,
                        facet_id,
                    )?;

                if sticker.color == Color::INTERNAL {
                    if piece_internals_indices_start.is_none() {
                        piece_internals_indices_start = Some((
                            polygon_index_range.start,
                            triangles_index_range.start,
                            edges_index_range.start,
                        ));
                    }
                } else {
                    mesh.add_sticker(
                        polygon_index_range,
                        triangles_index_range,
                        edges_index_range,
                    )?;
                }
            }

            let mut piece_internals_polygon_range = 0..0;
            let mut piece_internals_triangle_range = 0..0;
            let mut piece_internals_edge_range = 0..0;
            if let Some((polygon_start, tri_start, edge_start)) = piece_internals_indices_start {
                piece_internals_polygon_range = polygon_start..mesh.polygon_count;
                piece_internals_triangle_range = tri_start..mesh.triangle_count() as u32;
                piece_internals_edge_range = edge_start..mesh.edge_count() as u32;
            }
            mesh.add_piece(
                &piece_centroid,
                piece_internals_polygon_range,
                piece_internals_triangle_range,
                piece_internals_edge_range,
            )?;
        }

        for (_, facet_data) in facets {
            mesh.add_facet(facet_data.centroid.center(), facet_data.normal)?;
        }

        let axis_system = twist_system.axes.lock();
        let mut axes = PerAxis::new();
        let mut axis_map = HashMap::new();
        for (old_id, name) in super::iter_autonamed(
            &axis_system.names,
            &axis_system.ordering,
            crate::util::iter_uppercase_letter_names(),
        ) {
            let old_axis = axis_system.get(old_id)?;
            let vector = old_axis.vector().clone();
            let old_layers = &old_axis.layers;

            // Check that the layer planes are monotonic.
            let mut layer_planes = vec![];
            for layer in old_layers.iter_values() {
                layer_planes.extend(layer.top.clone());
                layer_planes.push(layer.bottom.flip());
            }
            for (a, b) in layer_planes.iter().zip(layer_planes.iter().skip(1)) {
                // We expect `a` is above `b`.
                if a == b {
                    continue;
                }
                if !approx_eq(&a.normal().dot(b.normal()).abs(), &1.0) {
                    bail!("layers of axis {name:?} are not parallel")
                }
                let is_b_below_a = a.location_of_point(b.pole()) == PointWhichSide::Inside;
                let is_a_above_b = b.location_of_point(a.pole()) == PointWhichSide::Outside;
                if !is_b_below_a || !is_a_above_b {
                    bail!("layers of axis {name:?} are not monotonic");
                }
            }

            let mut last_bottom = None;
            let layers = old_layers
                .iter_values()
                .map(|layer| {
                    // Bound the top of each layer at the bottom of the previous one.
                    LayerInfo {
                        bottom: layer.bottom.clone(),
                        top: layer.top.clone().or(std::mem::replace(
                            &mut last_bottom,
                            Some(layer.bottom.flip()),
                        )),
                    }
                })
                .collect();

            let new_id = axes.push(AxisInfo {
                name,
                vector,
                layers,
            })?;

            axis_map.insert(old_id, new_id);
        }
        let mut twists = PerTwist::new();
        for old_id in twist_system.alphabetized() {
            let twist = twist_system.get(old_id)?;
            let _new_id = twists.push(TwistInfo {
                name: match twist_system.names.get(old_id) {
                    Some(s) => s.clone(),
                    None => (old_id.0 + 1).to_string(), // 1-indexed
                },
                qtm: 1, // TODO: QTM
                axis: *axis_map.get(&twist.axis).ok_or_eyre("bad axis ID")?,
                transform: twist.transform.clone(),
                opposite: None,    // will be assigned later
                reverse: Twist(0), // will be assigned later
            });

            // TODO: check that transform keeps layer manifolds fixed
        }
        // TODO: assign opposite twists.
        // TODO: assign reverse twists

        let axis_by_name = axes
            .iter()
            .map(|(id, info)| (info.name.clone(), id))
            .collect();
        let twist_by_name = twists
            .iter()
            .map(|(id, info)| (info.name.clone(), id))
            .collect();

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            name,
            id,

            mesh,

            pieces,
            stickers,
            piece_types: [PieceTypeInfo {
                name: "Piece".to_string(), // TODO piece types
            }]
            .into_iter()
            .collect(),
            colors,

            scramble_moves_count: 500, // TODO

            notation: Notation {},

            axes,
            axis_by_name,

            twists,
            twist_by_name,

            space: Mutex::new(space),
        }))
    }
}

/// Piece of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceBuilder {
    /// Polytope of the piece.
    pub polytope: PolytopeId,
    /// If the piece is defunct because it was cut, these are the pieces it was
    /// cut up into.
    pub cut_result: PieceSet,
}
impl PieceBuilder {
    pub(super) fn new(polytope: PolytopeId) -> Self {
        Self {
            polytope,
            cut_result: PieceSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct TempStickerData {
    /// Polytope of the sticker.
    polytope: PolytopeId,
    /// Plane of the sticker.
    plane: Hyperplane,
    /// Color of the sticker.
    color: Color,
}
impl PartialEq for TempStickerData {
    fn eq(&self, other: &Self) -> bool {
        self.polytope == other.polytope && self.color == other.color
    }
}
impl Eq for TempStickerData {}
impl PartialOrd for TempStickerData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TempStickerData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.color, self.polytope).cmp(&(other.color, other.polytope))
    }
}

#[derive(Debug)]
struct TempFacetData {
    centroid: Centroid,
    normal: Vector,
}
impl TempFacetData {
    fn new(plane: &Hyperplane) -> Result<Self> {
        Ok(Self {
            centroid: Centroid::ZERO,
            normal: plane.normal().clone(),
        })
    }
}

/// Computes the sticker shrink vectors for a piece.
///
/// Each vertex shrinks along a vector pointing toward the centroid of the
/// piece, projected onto whatever sticker facets the vertex is part of. For
/// example, if a vertex is on an edge (1D manifold) of a 3D polytope, then its
/// shrink vector will point toward the centroid of the piece, projected onto
/// that edge. If a vertex is on a corner of its polytope, then its shrink
/// vector is zero.
fn compute_sticker_shrink_vectors(
    space: &Space,
    simplices: &mut SimplicialComplex<'_>,
    piece_shape: PolytopeId,
    stickers: &[TempStickerData],
) -> Result<HashMap<VertexId, Vector>> {
    // For the purposes of sticker shrink, we don't care about internal facets.
    let colored_sticker_polytopes = stickers
        .iter()
        .filter(|sticker| sticker.color != Color::INTERNAL)
        .map(|sticker| sticker.polytope)
        .collect_vec();

    // Make our own facet IDs that will stay within this object. We only care
    // about the facets that this piece has stickers on.
    let mut facet_hyperplanes = PerFacet::new();

    // For each element of the polytope, compute a set of the facet manifolds
    // that have a sticker containing the element.
    let mut elements_and_facet_sets_by_rank: Vec<HashMap<PolytopeId, FacetSet>> =
        vec![HashMap::new(); space.ndim() as usize + 1];
    for &sticker_polytope in &colored_sticker_polytopes {
        let facet = facet_hyperplanes.push(space.hyperplane_of_facet(sticker_polytope)?)?;

        for element in space.elements_of(sticker_polytope) {
            let rank = space[element].rank();
            elements_and_facet_sets_by_rank[rank as usize]
                .entry(element)
                .or_default()
                .insert(facet);
        }
    }

    // Find the largest (by rank) elements contained by all the sticker facets
    // of the piece.
    let centroid_of_greatest_common_elements: Option<Centroid> = elements_and_facet_sets_by_rank
        .iter()
        .rev()
        .map(|elements_and_facet_sets| {
            // Find elements that are contained by all sticker facets of the
            // piece.
            let elements_with_maximal_facet_set = elements_and_facet_sets
                .iter()
                .filter(|(_element, facet_set)| facet_set.len() == colored_sticker_polytopes.len())
                .map(|(element, _facet_set)| *element);
            // Add up their centroids. Technically we should take the centroid
            // of their convex hull, but this works well enough.
            simplices.combined_centroid(elements_with_maximal_facet_set)
        })
        // Select the elements with the largest rank and nonzero centroid.
        .find_map(|result_option| result_option.transpose())
        .transpose()?;
    // If such elements exist, then all vertices can shrink to the same point.
    if let Some(centroid) = centroid_of_greatest_common_elements {
        let shrink_target = centroid.center();
        return Ok(space
            .vertex_set(piece_shape)
            .iter()
            .map(|v| (v, &shrink_target - &space[v]))
            .collect());
    }

    // Otherwise, find the best elements for each set of facets. If a vertex is
    // not contained by any facets, then it will shrink toward the centroid of
    // the piece.
    let piece_centroid = simplices.centroid(piece_shape)?.center();

    // Compute the shrink target for each possible facet set that has a good
    // shrink target.
    let unique_facet_sets_of_vertices = elements_and_facet_sets_by_rank[0].values().unique();
    let shrink_target_by_facet_set: HashMap<&FacetSet, Vector> = unique_facet_sets_of_vertices
        .map(|facet_set| {
            // Find the largest elements of the piece that are contained by all
            // the facets in this set. There must be at least one vertex.
            let centroid_of_greatest_common_elements: Centroid = elements_and_facet_sets_by_rank
                .iter()
                .rev()
                .map(|elements_and_facet_sets| {
                    // Find elements that are contained by all sticker facets of
                    // the vertex.
                    let elements_with_superset_of_facets = elements_and_facet_sets
                        .iter()
                        .filter(|(_element, fs)| facet_set.iter().all(|f| fs.contains(f)))
                        .map(|(element, _fs)| *element);
                    // Add up their centroids. Technically we should take the
                    // centroid of their convex hull, but this works well
                    // enough.
                    simplices.combined_centroid(elements_with_superset_of_facets)
                })
                // Select the elements with the largest rank.
                .find_map(|result_option| result_option.transpose())
                // There must be some element with a superset of `facet_set`
                // because `facet_set` came from a vertex.
                .ok_or_eyre("no element with facet subset")??;

            eyre::Ok((facet_set, centroid_of_greatest_common_elements.center()))
        })
        .try_collect()?;

    // Compute shrink vectors for all vertices.
    let shrink_vectors = space.vertex_set(piece_shape).into_iter().map(|vertex_id| {
        let polytope_id = space.vertex_to_polytope(vertex_id);
        let facet_set = &elements_and_facet_sets_by_rank[0]
            .get(&polytope_id)
            .map(Cow::Borrowed)
            .unwrap_or_default();
        let vertex_pos = &space[vertex_id];
        let shrink_vector = match shrink_target_by_facet_set.get(&**facet_set) {
            Some(target) => target - vertex_pos,
            None => &piece_centroid - vertex_pos,
        };

        (vertex_id, shrink_vector)
    });
    Ok(shrink_vectors.collect())
}

fn build_shape_polygons(
    space: &Space,
    mesh: &mut Mesh,
    simplices: &mut SimplicialComplex<'_>,
    sticker_shrink_vectors: &HashMap<VertexId, Vector>,
    sticker_shape: PolytopeId,
    piece_centroid: &Vector,
    piece_id: Piece,
    facet_id: Facet,
) -> Result<(Range<usize>, Range<u32>, Range<u32>)> {
    let polygons_start = mesh.polygon_count;
    let triangles_start = mesh.triangle_count() as u32;
    let edges_start = mesh.edge_count() as u32;

    for polygon in space.subelements_with_rank(sticker_shape, 2) {
        let polygon_id = mesh.next_polygon_id()?;

        // Triangulate the polygon.
        let tris = simplices.triangles(polygon)?;

        // Compute tangent vectors.
        let basis = simplices.basis_for_polytope(polygon)?;
        if basis.len() != 2 {
            bail!("bad basis: expected 2 vectors but got {}", basis.len());
        }
        let mut u_tangent = &basis[0];
        let mut v_tangent = &basis[1];
        // Ensure that tangent vectors face the right way in 3D.
        let mut normal = vector![];
        if space.ndim() == 3 {
            let init = space.arbitrary_vertex(polygon);
            normal = u_tangent.cross_product_3d(&v_tangent);
            if normal.dot(&space[init] - &piece_centroid) < 0.0 {
                normal = -normal;
                std::mem::swap(&mut u_tangent, &mut v_tangent)
            }
        }

        #[cfg(debug_assertions)]
        hypermath::assert_approx_eq!(u_tangent.dot(&v_tangent), 0.0);

        // The simplices and mesh each have their own set of vertex IDs, so
        // we need to be able to map between them.
        let mut vertex_id_map: HashMap<VertexId, u32> = HashMap::new();
        for old_vertex_ids in tris {
            let mut new_vertex_ids = [0; 3];
            for (i, old_vertex_id) in old_vertex_ids.into_iter().enumerate() {
                new_vertex_ids[i] = match vertex_id_map.entry(old_vertex_id) {
                    hash_map::Entry::Occupied(e) => *e.get(),
                    hash_map::Entry::Vacant(e) => {
                        let position = &space[old_vertex_id];

                        let sticker_shrink_vector = sticker_shrink_vectors
                            .get(&old_vertex_id)
                            .ok_or_eyre("missing sticker shrink vector for vertex")?;

                        let new_vertex_id = mesh.add_vertex(MeshVertexData {
                            position,
                            u_tangent: &u_tangent,
                            v_tangent: &v_tangent,
                            sticker_shrink_vector,
                            piece_id,
                            facet_id,
                            polygon_id,
                        });
                        *e.insert(new_vertex_id)
                    }
                };
            }

            // Ensure that triangles face the right way in 3D.
            if space.ndim() == 3 {
                let [a, b, c] = new_vertex_ids.map(|v| mesh.vertex_position(v));
                let tri_normal = Vector::cross_product_3d(&(&c - b), &(&c - a));
                if normal.dot(tri_normal) < 0.0 {
                    new_vertex_ids.swap(0, 1);
                }
            }

            mesh.triangles.push(new_vertex_ids);
        }

        for edge @ [a, b] in space.edges_of(polygon) {
            // We should have seen all these vertices before because they show
            // up in triangles, but check just in case so we don't panic.
            if !(vertex_id_map.contains_key(&a) && vertex_id_map.contains_key(&b)) {
                bail!("vertex ID for edge is not part of a triangle");
            }
            mesh.edges.push(edge.map(|id| vertex_id_map[&id]));
        }
    }

    let polygons_end = mesh.polygon_count;
    let triangles_end = mesh.triangle_count() as u32;
    let edges_end = mesh.edge_count() as u32;
    Ok((
        polygons_start..polygons_end,
        triangles_start..triangles_end,
        edges_start..edges_end,
    ))
}

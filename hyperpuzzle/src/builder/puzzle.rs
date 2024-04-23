#![allow(clippy::too_many_arguments, clippy::too_many_lines)]

use std::collections::{hash_map, HashMap};
use std::ops::Range;
use std::sync::{Arc, Weak};

use eyre::{bail, ensure, OptionExt, Result};
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::centroid::Centroid;
use super::simplexifier::{Simplexifier, VertexId};
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

        let space_arc = Arc::new(Mutex::new(Space::new(self.ndim())?));
        let shape_mutex = self.shape.lock().clone(&space_arc)?;
        let shape = shape_mutex.lock();
        let twist_system_mutex = self.twists.lock().clone(&space_arc)?;
        let twist_system = twist_system_mutex.lock();
        // Take `space` out of the `Arc<Mutex<T>>`.
        let mut space = std::mem::replace(&mut *space_arc.lock(), Space::new(self.ndim())?);
        drop(space_arc); // Don't use that new space! It's dead to us.

        let mut mesh = Mesh::new_empty(self.ndim());
        mesh.color_count = shape.colors.len();

        // Only colored manifolds have an entry in `manifold_colors`.
        let mut manifold_colors: HashMap<ManifoldRef, Color> = HashMap::new();
        for &id in shape.colors.ordering.ids_in_order() {
            for manifold in shape.colors.get(id)?.manifolds().iter() {
                manifold_colors.insert(manifold, id);
            }
        }
        // All manifolds have an entry in `manifold_to_facet`.
        let mut manifold_to_facet: HashMap<ManifoldRef, Facet> = HashMap::new();

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

        let mut simplexifier = Simplexifier::new(&space);

        // Construct the mesh for each active piece.
        for old_piece_id in shape.active_pieces.iter() {
            let piece = &shape.pieces[old_piece_id];

            let piece_centroid = simplexifier.shape_centroid_point(piece.polytope.id)?;

            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type: PieceType(0), // TODO: piece types
                centroid: piece_centroid.clone(),
                polytope: piece.polytope,
            })?;

            // Add stickers to mesh, sorted ordered by color. It's important
            // that internal stickers are processed last, so that they are all
            // in a consecutive range for `piece_internals_index_ranges`.
            let mut piece_stickers = space
                .boundary_of(piece.polytope)
                .map(|sticker_shape| {
                    let facet_manifold = space.manifold_of(sticker_shape);
                    let color = *manifold_colors
                        .get(&facet_manifold)
                        .unwrap_or(&Color::INTERNAL);
                    (color, sticker_shape)
                })
                // Skip internals for 4D+.
                .filter(|(color, _sticker_shape)| space.ndim() < 4 || *color != Color::INTERNAL)
                .collect_vec();
            piece_stickers.sort();

            let sticker_shrink_vectors = compute_sticker_shrink_vectors(
                &space,
                &mut simplexifier,
                piece.polytope,
                &piece_stickers,
            )?;

            let mut piece_internals_indices_start = None;

            for (sticker_color, sticker_shape) in piece_stickers {
                if sticker_color != Color::INTERNAL {
                    let sticker_id = stickers.push(StickerInfo {
                        piece: piece_id,
                        color: sticker_color,
                    })?;
                    pieces[piece_id].stickers.push(sticker_id);
                }

                let sticker_centroid = simplexifier.shape_centroid(sticker_shape.id)?;
                let manifold = space.manifold_of(sticker_shape);
                let facet_id = match manifold_to_facet.entry(manifold) {
                    hash_map::Entry::Occupied(e) => *e.get(),
                    hash_map::Entry::Vacant(e) => {
                        let facet_id = facets.push(TempFacetData::new(&space, manifold)?)?;
                        *e.insert(facet_id)
                    }
                };

                facets[facet_id].centroid += sticker_centroid;

                let (polygon_index_range, triangles_index_range, edges_index_range) =
                    build_shape_polygons(
                        &space,
                        &mut mesh,
                        &mut simplexifier,
                        &sticker_shrink_vectors,
                        sticker_shape,
                        piece_id,
                        facet_id,
                    )?;

                if sticker_color == Color::INTERNAL {
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

            // Check that the manifolds are manotonic.
            let mut layer_manifolds = vec![];
            for layer in old_layers.iter_values() {
                layer_manifolds.extend(layer.top);
                layer_manifolds.push(-layer.bottom);
            }
            for (&a, &b) in layer_manifolds.iter().zip(layer_manifolds.iter().skip(1)) {
                // We expect `a` is above `b`.
                if a == b {
                    continue;
                }
                let is_b_below_a =
                    space.which_side_has_manifold(space.manifold(), a, b.id)? == WhichSide::Inside;
                let is_a_above_b =
                    space.which_side_has_manifold(space.manifold(), b, a.id)? == WhichSide::Outside;
                if !is_b_below_a || !is_a_above_b {
                    bail!("layers of axis {name:?} are not monotonic");
                }
            }

            let layers = old_layers
                .iter_values()
                .map(|layer| LayerInfo {
                    bottom: layer.bottom,
                    top: layer.top,
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
                transform: space.add_isometry(twist.transform.clone())?,
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
    /// Polytope of the peice in the space.
    pub polytope: AtomicPolytopeRef,
    /// If the piece is defunct because it was cut, these are the pieces it was
    /// cut up into.
    pub cut_result: PieceSet,
}
impl PieceBuilder {
    pub(super) fn new(polytope: AtomicPolytopeRef) -> Self {
        Self {
            polytope,
            cut_result: PieceSet::new(),
        }
    }
}

#[derive(Debug)]
struct TempFacetData {
    centroid: Centroid,
    normal: Vector,
}
impl TempFacetData {
    fn new(space: &Space, manifold: ManifoldRef) -> Result<Self> {
        let ipns_blade = space.blade_of(manifold).opns_to_ipns(space.ndim());
        ensure!(
            ipns_blade.ipns_is_flat(),
            "4D backface culling assumes flat faces",
        );

        Ok(Self {
            centroid: Centroid::ZERO,
            normal: ipns_blade
                .ipns_plane_normal()
                .ok_or_eyre("no plane normal")?,
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
    simplexifier: &mut Simplexifier<'_>,
    piece_shape: AtomicPolytopeRef,
    stickers: &[(Color, AtomicPolytopeRef)],
) -> Result<HashMap<VertexId, Vector>> {
    // For the purposes of sticker shrink, we don't care about internal facets.
    let colored_sticker_shapes = stickers
        .iter()
        .filter(|&&(color, _sticker_shape)| color != Color::INTERNAL)
        .map(|&(_color, sticker_shape)| sticker_shape)
        .collect_vec();

    let ndim = space.ndim();

    // Make our own facet IDs that will stay within this object. We only care
    // about the facets that this piece has stickers on.
    let mut facet_blades_ipns = PerFacet::new();

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    enum Element {
        Point(VertexId),
        NonPoint(AtomicPolytopeId),
    }
    fn sum_centroids(
        simplexifier: &mut Simplexifier<'_>,
        elements: impl Iterator<Item = Element>,
    ) -> Result<Option<Centroid>> {
        let mut ret = Centroid::ZERO;
        for element in elements {
            ret += match element {
                Element::Point(p) => Ok(Centroid::new(&simplexifier[p], 1.0)),
                Element::NonPoint(p) => simplexifier.shape_centroid(p),
            }?;
        }
        Ok((!ret.is_zero()).then_some(ret))
    }

    // For each element of the polytope, compute a set of the facet manifolds
    // that have a sticker containing the element.
    let mut facet_set_per_vertex: HashMap<VertexId, FacetSet> = HashMap::new();
    let mut elements_and_facet_sets_by_rank: Vec<HashMap<Element, FacetSet>> =
        vec![HashMap::new(); space.ndim() as usize + 1];
    for &sticker_shape in &colored_sticker_shapes {
        let manifold = space.manifold_of(sticker_shape);
        let facet = facet_blades_ipns.push(space.blade_of(manifold).opns_to_ipns(ndim))?;

        for vertex in simplexifier.vertex_set(sticker_shape)? {
            facet_set_per_vertex
                .entry(vertex)
                .or_default()
                .insert(facet);
        }
        for element in space.elements_of(sticker_shape.id)? {
            if space.ndim_of(element) > 0 {
                elements_and_facet_sets_by_rank[space.ndim_of(element) as usize]
                    .entry(Element::NonPoint(element))
                    .or_default()
                    .insert(facet);
            }
        }
    }
    elements_and_facet_sets_by_rank[0] = facet_set_per_vertex
        .iter()
        .map(|(&vertex, facet_set)| (Element::Point(vertex), facet_set.clone()))
        .collect();

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
                .filter(|(_element, facet_set)| facet_set.len() == colored_sticker_shapes.len())
                .map(|(element, _facet_set)| *element);
            // Add up their centroids. Technically we should take the centroid
            // of their convex hull, but this works well enough.
            sum_centroids(simplexifier, elements_with_maximal_facet_set)
        })
        // Select the elements with the largest rank.
        .find_map(|result_option| result_option.transpose())
        .transpose()?;
    // If such elements exist, then all vertices can shrink to the same point.
    if let Some(centroid) = centroid_of_greatest_common_elements {
        let shrink_target = centroid.center();
        return Ok(simplexifier
            .vertex_set(piece_shape)?
            .into_iter()
            .map(|vertex| {
                let vertex_position = &simplexifier[vertex];
                (vertex, &shrink_target - vertex_position)
            })
            .collect());
    }

    // Otherwise, find the best elements for each set of facets. If a vertex is
    // not contained by any facets, then it will shrink toward the centroid of
    // the piece.
    let piece_centroid = simplexifier.shape_centroid_point(piece_shape.id)?;

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
                    sum_centroids(simplexifier, elements_with_superset_of_facets)
                })
                // Select the elements with the largest rank.
                .find_map(|result_option| result_option.transpose())
                // There must be some element with a superset of `facet_set`
                // because `facet_set` came from a vertex.
                .expect("no element with facet subset")?;

            eyre::Ok((facet_set, centroid_of_greatest_common_elements.center()))
        })
        .try_collect()?;

    // Compute shrink vectors for all vertices.
    let shrink_vectors = simplexifier
        .vertex_set(piece_shape)?
        .into_iter()
        .map(|vertex| {
            let facet_set = facet_set_per_vertex.remove(&vertex).unwrap_or_default();
            let vertex_pos = &simplexifier[vertex];
            let shrink_vector = match shrink_target_by_facet_set.get(&facet_set) {
                Some(target) => target - vertex_pos,
                None => &piece_centroid - vertex_pos,
            };

            (vertex, shrink_vector)
        });
    Ok(shrink_vectors.collect())
}

fn build_shape_polygons(
    space: &Space,
    mesh: &mut Mesh,
    simplexifier: &mut Simplexifier<'_>,
    sticker_shrink_vectors: &HashMap<VertexId, Vector>,
    sticker_shape: AtomicPolytopeRef,
    piece_id: Piece,
    facet_id: Facet,
) -> Result<(Range<usize>, Range<u32>, Range<u32>)> {
    let polygons_start = mesh.polygon_count;
    let triangles_start = mesh.triangle_count() as u32;
    let edges_start = mesh.edge_count() as u32;

    for polygon in space.children_with_ndim(sticker_shape, 2) {
        let polygon_id = mesh.next_polygon_id()?;

        // Get the tangent space so that we can compute tangent vectors
        // for each vertex.
        let manifold = space.manifold_of(polygon);
        let blade = space.blade_of(manifold);
        if blade.cga_opns_ndim() != Some(2) {
            bail!("polygon must lie on 2D manifold")
        }
        let tangent_space = blade.opns_tangent_space();

        // Triangulate the polygon.
        let tris = simplexifier.triangles(polygon)?;

        // The simplexifier and mesh each have their own set of vertex IDs, so
        // we need to be able to map between them.
        let mut vertex_id_map: HashMap<VertexId, u32> = HashMap::new();
        for old_vertex_ids in tris {
            let mut new_vertex_ids = [0; 3];
            for (i, old_vertex_id) in old_vertex_ids.into_iter().enumerate() {
                new_vertex_ids[i] = match vertex_id_map.entry(old_vertex_id) {
                    hash_map::Entry::Occupied(e) => *e.get(),
                    hash_map::Entry::Vacant(e) => {
                        let position = &simplexifier[old_vertex_id];
                        let tangent_vectors = tangent_space.at(position);
                        let Some([u_tangent, v_tangent]) = tangent_vectors.as_deref() else {
                            bail!("bad tangent space");
                        };

                        let sticker_shrink_vector = sticker_shrink_vectors
                            .get(&old_vertex_id)
                            .ok_or_eyre("missing sticker shrink vector for vertex")?;

                        let new_vertex_id = mesh.add_vertex(MeshVertexData {
                            position,
                            u_tangent,
                            v_tangent,
                            sticker_shrink_vector,
                            piece_id,
                            facet_id,
                            polygon_id,
                        });
                        *e.insert(new_vertex_id)
                    }
                };
            }
            mesh.triangles.push(new_vertex_ids);
        }

        for edge in simplexifier.polygon_edges(polygon)? {
            // We should have seen all these vertices before because they show
            // up in triangles.
            mesh.edges.push(edge.map(|id| vertex_id_map[&id]))
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

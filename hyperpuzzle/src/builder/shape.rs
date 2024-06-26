use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::ops::Range;
use std::sync::Arc;

use eyre::{bail, Context, OptionExt, Result};
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use smallvec::smallvec;

use super::{ColorSystemBuilder, PieceBuilder};
use crate::puzzle::*;

/// Soup of shapes being constructed.
#[derive(Debug)]
pub struct ShapeBuilder {
    /// Space where the puzzle exists.
    pub space: Arc<Space>,

    /// Puzzle pieces.
    pub pieces: PerPiece<PieceBuilder>,
    /// Puzzle pieces that are not defunct (removed or cut) and so should be
    /// included in the final puzzle.
    pub active_pieces: PieceSet,

    /// Facet colors.
    pub colors: ColorSystemBuilder,
}
impl ShapeBuilder {
    /// Constructs a shape builder that starts with an empty Euclidean space.
    pub fn new_empty(space: Arc<Space>) -> Self {
        Self {
            space,

            pieces: PerPiece::new(),
            active_pieces: PieceSet::new(),

            colors: ColorSystemBuilder::new(),
        }
    }

    /// Constructs a shape builder that starts with a single solid piece (the
    /// primordial cube)
    pub fn new_with_primordial_cube(space: Arc<Space>) -> Result<Self> {
        let mut this = Self::new_empty(Arc::clone(&space));
        let primordial_cube = space.add_primordial_cube(crate::PRIMORDIAL_CUBE_RADIUS)?;
        let root_piece_builder = PieceBuilder::new(primordial_cube, VecMap::new())?;
        let root_piece = this.pieces.push(root_piece_builder)?;
        this.active_pieces.insert(root_piece);
        Ok(this)
    }

    /// Returns the number of dimensions of the underlying space.
    pub fn ndim(&self) -> u8 {
        self.space.ndim()
    }

    /// Cuts each piece by a cut, throwing away the portions that are outside
    /// the cut. Each piece in the old set becomes defunct, and each piece in
    /// the new set inherits its active status from the corresponding piece in
    /// the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn carve(
        &mut self,
        pieces: Option<&PieceSet>,
        cut_plane: Hyperplane,
        inside_color: Option<Color>,
    ) -> Result<()> {
        let mut cut = Cut::carve(&self.space, cut_plane);
        self.cut_and_deactivate_pieces(&mut cut, pieces, inside_color, None)
    }
    /// Cuts each piece by a cut, keeping all results. Each piece in the old set
    /// becomes defunct, and each piece in the new set inherits its active
    /// status from the corresponding piece in the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn slice(
        &mut self,
        pieces: Option<&PieceSet>,
        cut_plane: Hyperplane,
        inside_color: Option<Color>,
        outside_color: Option<Color>,
    ) -> Result<()> {
        let mut cut = Cut::slice(&self.space, cut_plane);
        self.cut_and_deactivate_pieces(&mut cut, pieces, inside_color, outside_color)
    }
    fn cut_and_deactivate_pieces(
        &mut self,
        cut: &mut Cut,
        pieces: Option<&PieceSet>,
        inside_color: Option<Color>,
        outside_color: Option<Color>,
    ) -> Result<()> {
        let pieces = match pieces {
            Some(piece_set) => self.update_piece_set(piece_set),
            None => self.active_pieces.clone(),
        };

        for old_piece in pieces.iter() {
            let inside_polytope;
            let outside_polytope;
            let mut inside_stickers = VecMap::new();
            let mut outside_stickers = VecMap::new();

            // Cut the old piece and add the new pieces as active.
            let old_piece_polytope = self.pieces[old_piece].polytope;
            match cut.cut(old_piece_polytope).context("error cutting piece")? {
                ElementCutOutput::Flush => bail!("piece is flush with cut"),

                out @ ElementCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection,
                } => {
                    if intersection.is_none()
                        && out
                            .is_unchanged_from(self.space.get(old_piece_polytope).as_element().id())
                    {
                        // Leave this piece unchanged.
                        continue;
                    }

                    inside_polytope = inside;
                    outside_polytope = outside;

                    if let Some(p) = intersection {
                        if let Some(c) = inside_color {
                            inside_stickers.insert(self.space.get(p).as_facet()?.id(), c);
                        }
                        if let Some(c) = outside_color {
                            outside_stickers.insert(self.space.get(p).as_facet()?.id(), c);
                        }
                    }
                }
            }

            // Cut the old stickers.
            for (&old_sticker_polytope, &old_color) in &self.pieces[old_piece].stickers {
                match cut
                    .cut(old_sticker_polytope)
                    .context("error cutting sticker")?
                {
                    ElementCutOutput::Flush => (), // Leave the sticker unchanged
                    ElementCutOutput::NonFlush {
                        inside, outside, ..
                    } => {
                        // Use `get_or_insert()` instead to keep old color for
                        // flush stickers instead of assigning the new color.
                        if let Some(p) = inside {
                            inside_stickers.insert(self.space.get(p).as_facet()?.id(), old_color);
                        }
                        if let Some(p) = outside {
                            outside_stickers.insert(self.space.get(p).as_facet()?.id(), old_color);
                        }
                    }
                }
            }

            let new_inside_piece = match inside_polytope {
                Some(p) => {
                    let inside_piece = PieceBuilder::new(self.space.get(p), inside_stickers)?;
                    Some(self.pieces.push(inside_piece)?)
                }
                None => None,
            };
            let new_outside_piece = match outside_polytope {
                Some(p) => {
                    let outside_piece = PieceBuilder::new(self.space.get(p), outside_stickers)?;
                    Some(self.pieces.push(outside_piece)?)
                }
                None => None,
            };

            self.active_pieces.extend(new_inside_piece);
            self.active_pieces.extend(new_outside_piece);

            // The old piece is defunct, so deactivate it and record its cut
            // result.
            self.active_pieces.remove(&old_piece);
            self.pieces[old_piece].cut_result =
                itertools::chain(new_inside_piece, new_outside_piece).collect();

            self.active_pieces.remove(&old_piece);
        }

        Ok(())
    }

    /// Updates a piece set, replacing defunct pieces with their cut results.
    /// Call this before doing anything with a piece set to prevent operating on
    /// defunct pieces.
    pub fn update_piece_set(&self, piece_set: &PieceSet) -> PieceSet {
        let mut queue = piece_set.iter().collect_vec();
        let mut output = PieceSet::new();
        while let Some(old_piece) = queue.pop() {
            if self.active_pieces.contains(&old_piece) {
                output.insert(old_piece);
            } else {
                queue.extend(self.pieces[old_piece].cut_result.iter());
            }
        }
        output
    }

    /// Constructs a mesh and assembles piece & sticker data for the shape.
    pub fn build(&self) -> Result<(Mesh, PerPiece<PieceInfo>, PerSticker<StickerInfo>)> {
        let space = &self.space;
        let ndim = space.ndim();

        let mut mesh = Mesh::new_empty(ndim);
        mesh.color_count = self.colors.len();

        // All surfaces have an entry in `hyperplane_to_surface`.
        let mut hyperplane_to_surface: ApproxHashMap<Hyperplane, Surface> = ApproxHashMap::new();

        // As we construct the mesh, we'll renumber all the pieces and stickers
        // to exclude inactive ones.
        let mut pieces = PerPiece::<PieceInfo>::new();
        let mut stickers = PerSticker::<StickerInfo>::new();
        let mut surfaces = PerSurface::<TempSurfaceData>::new();

        // Construct the mesh for each active piece.
        for old_piece_id in self.active_pieces.iter() {
            let piece = &self.pieces[old_piece_id];

            let piece_centroid = space.get(piece.polytope).centroid()?.center();

            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type: PieceType(0), // TODO: piece types
                centroid: piece_centroid.clone(),
                polytope: piece.polytope,
            })?;

            // Add stickers to the mesh sorted by color. It's important that
            // internal stickers are processed last, so that they are all in a
            // consecutive range for `piece_internals_index_ranges`.
            let mut stickers_of_piece: Vec<TempStickerData> = space
                .get(piece.polytope)
                .facets()
                .map(|facet_polytope| {
                    // Select the orientation of the facet hyperplane such that
                    // the centroid of the piece is on the inside.
                    let mut plane = facet_polytope.hyperplane()?;
                    if plane.location_of_point(&piece_centroid) == PointWhichSide::Outside {
                        plane = plane.flip();
                    }

                    eyre::Ok(TempStickerData {
                        facet: facet_polytope.id(),
                        plane,
                        color: piece.sticker_color(facet_polytope.id()),
                    })
                })
                // Skip internals for 4D+.
                .filter_ok(|data| ndim < 4 || data.color != Color::INTERNAL)
                .try_collect()?;
            // Sort the stickers, as mentioned above.
            stickers_of_piece.sort();

            let sticker_shrink_vectors =
                compute_sticker_shrink_vectors(space.get(piece.polytope), &stickers_of_piece)?;

            let mut piece_internals_indices_start = None;

            for sticker in stickers_of_piece {
                if sticker.color != Color::INTERNAL {
                    let sticker_id = stickers.push(StickerInfo {
                        piece: piece_id,
                        color: sticker.color,
                    })?;
                    pieces[piece_id].stickers.push(sticker_id);
                }

                let sticker_centroid = space.get(sticker.facet).centroid()?;
                let sticker_plane = sticker.plane;
                let surface_id = match hyperplane_to_surface.entry(sticker_plane.clone()) {
                    hash_map::Entry::Occupied(e) => *e.get(),
                    hash_map::Entry::Vacant(e) => {
                        let surface_id = surfaces.push(TempSurfaceData::new(&sticker_plane)?)?;
                        *e.insert(surface_id)
                    }
                };

                surfaces[surface_id].centroid += sticker_centroid;

                let (polygon_index_range, triangles_index_range, edges_index_range) =
                    build_shape_polygons(
                        &space,
                        &mut mesh,
                        &sticker_shrink_vectors,
                        space.get(sticker.facet),
                        &piece_centroid,
                        piece_id,
                        surface_id,
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

        for (_, surface_data) in surfaces {
            mesh.add_puzzle_surface(surface_data.centroid.center(), surface_data.normal)?;
        }

        Ok((mesh, pieces, stickers))
    }
}

#[derive(Debug)]
struct TempSurfaceData {
    centroid: Centroid,
    normal: Vector,
}
impl TempSurfaceData {
    fn new(plane: &Hyperplane) -> Result<Self> {
        Ok(Self {
            centroid: Centroid::ZERO,
            normal: plane.normal().clone(),
        })
    }
}

#[derive(Debug, Clone)]
struct TempStickerData {
    /// Facet of the sticker.
    facet: FacetId,
    /// Plane of the sticker.
    plane: Hyperplane,
    /// Color of the sticker.
    color: Color,
}
impl PartialEq for TempStickerData {
    fn eq(&self, other: &Self) -> bool {
        self.facet == other.facet && self.color == other.color
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
        (self.color, self.facet).cmp(&(other.color, other.facet))
    }
}

fn build_shape_polygons(
    space: &Space,
    mesh: &mut Mesh,
    sticker_shrink_vectors: &HashMap<VertexId, Vector>,
    sticker_shape: Facet<'_>,
    piece_centroid: &Vector,
    piece_id: Piece,
    surface_id: Surface,
) -> Result<(Range<usize>, Range<u32>, Range<u32>)> {
    let polygons_start = mesh.polygon_count;
    let triangles_start = mesh.triangle_count() as u32;
    let edges_start = mesh.edge_count() as u32;

    for polygon in sticker_shape.as_element().face_set() {
        let polygon_id = mesh.next_polygon_id()?;

        // Triangulate the polygon.
        let tris = polygon.triangles()?;

        // Compute tangent vectors.
        let mut basis = polygon.tangent_vectors()?;
        // Ensure that tangent vectors face the right way in 3D.
        let mut normal = vector![];
        if space.ndim() == 3 {
            let init = polygon.arbitrary_vertex()?;
            normal = basis[0].cross_product_3d(&basis[1]);
            if normal.dot(init.pos() - &piece_centroid) < 0.0 {
                normal = -normal;
                basis.reverse();
            }
        }
        let [u_tangent, v_tangent] = &basis;

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
                        let position = space.get(old_vertex_id).pos();

                        let sticker_shrink_vector = sticker_shrink_vectors
                            .get(&old_vertex_id)
                            .ok_or_eyre("missing sticker shrink vector for vertex")?;

                        let new_vertex_id = mesh.add_puzzle_vertex(MeshVertexData {
                            position: &position,
                            u_tangent,
                            v_tangent,
                            sticker_shrink_vector,
                            piece_id,
                            surface_id,
                            polygon_id,
                        })?;
                        *e.insert(new_vertex_id)
                    }
                };
            }

            // Ensure that triangles face the right way in 3D.
            if space.ndim() == 3 {
                let [a, b, c] = new_vertex_ids.map(|v| mesh.vertex_position(v));
                let tri_normal = Vector::cross_product_3d(&(&c - a), &(&c - b));
                if normal.dot(tri_normal) < 0.0 {
                    new_vertex_ids.swap(0, 1);
                }
            }

            mesh.triangles.push(new_vertex_ids);
        }

        for edge @ [a, b] in polygon.edge_endpoints()? {
            // We should have seen all these vertices before because they show
            // up in triangles, but check just in case so we don't panic.
            if !(vertex_id_map.contains_key(&a.id()) && vertex_id_map.contains_key(&b.id())) {
                bail!("vertex ID for edge is not part of a triangle");
            }
            mesh.edges.push(edge.map(|v| vertex_id_map[&v.id()]));
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

/// Computes the sticker shrink vectors for a piece.
///
/// Each vertex shrinks along a vector pointing toward the centroid of the
/// piece, projected onto whatever sticker facets the vertex is part of. For
/// example, if a vertex is on an edge (1D manifold) of a 3D polytope, then its
/// shrink vector will point toward the centroid of the piece, projected onto
/// that edge. If a vertex is on a corner of its polytope, then its shrink
/// vector is zero.
fn compute_sticker_shrink_vectors(
    piece_shape: Polytope<'_>,
    stickers: &[TempStickerData],
) -> Result<HashMap<VertexId, Vector>> {
    let space = piece_shape.space();

    // For the purposes of sticker shrink, we don't care about internal facets.
    let colored_sticker_facets = stickers
        .iter()
        .filter(|sticker| sticker.color != Color::INTERNAL)
        .map(|sticker| space.get(sticker.facet))
        .collect_vec();

    // Make our own surface IDs that will stay within this object. We only care
    // about the surfaces that this piece has stickers on.
    let mut next_surface_id = Surface(0);

    // TODO: I don't think this is correctly dimension-generic.

    // For each element of the polytope, compute a set of the surface manifolds
    // that have a sticker containing the element.
    let mut elements_and_surface_sets_by_rank: Vec<HashMap<ElementId, SurfaceSet>> =
        vec![HashMap::new(); space.ndim() as usize + 1];
    for &sticker_facet in &colored_sticker_facets {
        let temp_surface_id = next_surface_id.take_and_increment()?;

        for ridge in sticker_facet.subelements() {
            let rank = ridge.rank();
            elements_and_surface_sets_by_rank[rank as usize]
                .entry(ridge.id())
                .or_default()
                .insert(temp_surface_id);
        }
    }

    // Find the largest (by rank) elements contained by all the sticker facets
    // of the piece.
    let centroid_of_greatest_common_elements: Option<Centroid> = elements_and_surface_sets_by_rank
        .iter()
        .rev()
        .map(|elements_and_facet_sets| {
            // Find elements that are contained by all sticker facets of the
            // piece.
            let elements_with_maximal_facet_set = elements_and_facet_sets
                .iter()
                .filter(|(_element, facet_set)| facet_set.len() == colored_sticker_facets.len())
                .map(|(element, _facet_set)| *element);
            // Add up their centroids. Technically we should take the centroid
            // of their convex hull, but this works well enough.
            space.combined_centroid(elements_with_maximal_facet_set)
        })
        // Select the elements with the largest rank and nonzero centroid.
        .find_map(|result_option| result_option.transpose())
        .transpose()?;
    // If such elements exist, then all vertices can shrink to the same point.
    if let Some(centroid) = centroid_of_greatest_common_elements {
        let shrink_target = centroid.center();
        return Ok(piece_shape
            .vertex_set()
            .map(|v| (v.id(), &shrink_target - v.pos()))
            .collect());
    }

    // Otherwise, find the best elements for each set of facets. If a vertex is
    // not contained by any facets, then it will shrink toward the centroid of
    // the piece.
    let piece_centroid = piece_shape.centroid()?.center();

    // Compute the shrink target for each possible facet set that has a good
    // shrink target.
    let unique_facet_sets_of_vertices = elements_and_surface_sets_by_rank[0].values().unique();
    let shrink_target_by_surface_set: HashMap<&SurfaceSet, Vector> = unique_facet_sets_of_vertices
        .map(|facet_set| {
            // Find the largest elements of the piece that are contained by all
            // the facets in this set. There must be at least one vertex.
            let centroid_of_greatest_common_elements: Centroid = elements_and_surface_sets_by_rank
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
                    space.combined_centroid(elements_with_superset_of_facets)
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
    let shrink_vectors = piece_shape.vertex_set().into_iter().map(|vertex| {
        let surface_set = &elements_and_surface_sets_by_rank[0]
            .get(&vertex.as_element().id())
            .map(Cow::Borrowed)
            .unwrap_or_default();
        let vertex_pos = vertex.pos();
        let shrink_vector = match shrink_target_by_surface_set.get(&**surface_set) {
            Some(target) => target - vertex_pos,
            None => &piece_centroid - vertex_pos,
        };

        (vertex.id(), shrink_vector)
    });
    Ok(shrink_vectors.collect())
}

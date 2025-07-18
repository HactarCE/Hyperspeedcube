use std::borrow::Cow;
use std::collections::{HashMap, hash_map};
use std::ops::Range;
use std::sync::Arc;

use eyre::{Context, OptionExt, Result, bail, ensure, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;
use hypershape::prelude::*;
use indexmap::IndexMap;
use itertools::Itertools;
use regex::Regex;
use smallvec::smallvec;

use super::{ColorSystemBuilder, PieceBuilder, PieceTypeBuilder};

// TODO: build color system separately and statically?

const DEFAULT_PIECE_TYPE_NAME: &str = "piece";
const DEFAULT_PIECE_TYPE_DISPLAY: &str = "Piece";

/// Soup of pieces being constructed.
#[derive(Debug)]
pub struct ShapeBuilder {
    /// Space where the puzzle exists.
    pub space: Arc<Space>,

    /// Puzzle pieces.
    pub pieces: PerPiece<PieceBuilder>,
    /// Puzzle pieces that are not defunct (removed or cut) and so should be
    /// included in the final puzzle.
    pub active_pieces: PieceSet,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: bool,

    /// Puzzle piece types.
    piece_types: PerPieceType<PieceTypeBuilder>,
    /// Map from piece type name to ID.
    piece_types_by_name: HashMap<String, PieceType>,
    /// Map from piece type name to user-friendly display name.
    piece_type_display_names: IndexMap<String, String>,
    /// Piece types that got overwritten.
    overwritten_piece_types: Vec<(Piece, PieceType)>,

    /// Facet colors.
    pub colors: ColorSystemBuilder,
}
impl ShapeBuilder {
    /// Constructs a shape builder that starts with an empty Euclidean space.
    pub fn new_empty(puzzle_id: &str, space: Arc<Space>) -> Self {
        Self {
            space,

            pieces: PerPiece::new(),
            active_pieces: PieceSet::new(),

            remove_internals: true,

            piece_types: PerPieceType::new(),
            piece_types_by_name: HashMap::new(),
            piece_type_display_names: IndexMap::new(),
            overwritten_piece_types: vec![],

            colors: ColorSystemBuilder::new_ad_hoc(puzzle_id),
        }
    }

    /// Constructs a shape builder that starts with a single solid piece (the
    /// primordial cube)
    pub fn new_with_primordial_cube(puzzle_id: &str, space: Arc<Space>) -> Result<Self> {
        let mut this = Self::new_empty(puzzle_id, Arc::clone(&space));
        let primordial_cube = space.add_primordial_cube(hypershape::PRIMORDIAL_CUBE_RADIUS)?;
        let root_piece_builder = PieceBuilder::new(primordial_cube, VecMap::new());
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
    ) -> Result<()> {
        let mut cut = Cut::slice(&self.space, cut_plane);
        self.cut_and_deactivate_pieces(&mut cut, pieces, inside_color, None)
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
            for entry in &self.pieces[old_piece].stickers {
                let (&old_sticker_polytope, &old_color) = (entry.key(), &entry.value);
                match cut
                    .cut(old_sticker_polytope)
                    .context("error cutting sticker")?
                {
                    ElementCutOutput::Flush => {
                        // Assign new sticker color if we have a new one;
                        // otherwise leave the sticker color unchanged.
                        inside_stickers
                            .insert(old_sticker_polytope, inside_color.unwrap_or(old_color));
                        outside_stickers
                            .insert(old_sticker_polytope, outside_color.unwrap_or(old_color));
                    }
                    ElementCutOutput::NonFlush {
                        inside, outside, ..
                    } => {
                        // Leave the sticker color unchanged.
                        if let Some(p) = inside {
                            inside_stickers.insert(self.space.get(p).as_facet()?.id(), old_color);
                        }
                        if let Some(p) = outside {
                            outside_stickers.insert(self.space.get(p).as_facet()?.id(), old_color);
                        }
                    }
                }
            }

            let new_inside_piece = self.add_opt_piece(inside_polytope, inside_stickers)?;
            let new_outside_piece = self.add_opt_piece(outside_polytope, outside_stickers)?;
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

    fn add_opt_piece(
        &mut self,
        polytope: Option<ElementId>,
        stickers: VecMap<FacetId, Color>,
    ) -> Result<Option<Piece>> {
        let Some(polytope) = polytope else {
            return Ok(None);
        };
        let p = self.space.get(polytope).as_polytope()?;

        if self.remove_internals && stickers.is_empty() && !p.has_primordial_facet() {
            return Ok(None);
        }

        Ok(Some(self.pieces.push(PieceBuilder::new(p, stickers))?))
    }

    /// Updates a piece set, replacing defunct pieces with their cut results.
    /// Call this before doing anything with a piece set to prevent operating on
    /// defunct pieces.
    pub fn update_piece_set(&self, piece_set: &PieceSet) -> PieceSet {
        let mut queue = piece_set.iter().collect_vec();
        let mut output = PieceSet::new();
        while let Some(old_piece) = queue.pop() {
            if self.active_pieces.contains(old_piece) {
                output.insert(old_piece);
            } else {
                queue.extend(self.pieces[old_piece].cut_result.iter());
            }
        }
        output
    }

    /// Returns the list of piece types.
    pub fn piece_types(&self) -> &PerPieceType<PieceTypeBuilder> {
        &self.piece_types
    }
    /// Returns the piece type with the given name, creating one if it does not
    /// exist.
    pub fn get_or_add_piece_type(
        &mut self,
        name: String,
        display: Option<String>,
    ) -> Result<PieceType> {
        lazy_static! {
            static ref PIECE_TYPE_NAME_REGEX: Regex =
                Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(/[a-zA-Z0-9_]*)*$").expect("bad regex");
        }

        if !PIECE_TYPE_NAME_REGEX.is_match(&name) {
            bail!("invalid piece type name: {name:?}")
        }

        // Check display name.
        if let Some(new_display) = display {
            match self.piece_type_display_names.entry(name.clone()) {
                indexmap::map::Entry::Occupied(e) => {
                    let old_display = e.get();
                    if *old_display != new_display {
                        bail!(
                            "conflicting display names for piece type \
                             {name:?}: {old_display:?} and {new_display:?}",
                        );
                    }
                }
                indexmap::map::Entry::Vacant(e) => {
                    e.insert(new_display);
                }
            }
        }

        // TODO: validate piece type name
        match self.piece_types_by_name.entry(name.clone()) {
            hash_map::Entry::Occupied(e) => Ok(*e.get()),
            hash_map::Entry::Vacant(e) => {
                let id = self.piece_types.push(PieceTypeBuilder { name })?;
                Ok(*e.insert(id))
            }
        }
    }
    /// Returns the existing piece type with the given name, or `None` if it
    /// does not exist.
    pub fn piece_type_from_name(&self, name: &str) -> Option<PieceType> {
        self.piece_types_by_name.get(name).copied()
    }

    /// Returns the set of active pieces in the region defined by a membership
    /// test.
    pub fn active_pieces_in_region<'a>(
        &'a mut self,
        has_point: impl 'a + Fn(&Point) -> bool,
    ) -> impl 'a + Iterator<Item = Piece> {
        self.active_pieces
            .clone()
            .into_iter()
            .filter(move |&p| has_point(self.pieces[p].interior_point(&self.space)))
    }

    /// Marks the type of all pieces in the region defined by a membership test.
    pub fn mark_piece_by_region(
        &mut self,
        name: &str,
        display: Option<String>,
        has_point: impl Fn(&Point) -> bool,
        warn_fn: impl FnOnce(eyre::Error),
    ) -> Result<()> {
        let piece_type = match self.get_or_add_piece_type(name.to_string(), display) {
            Ok(id) => id,
            Err(e) => {
                warn_fn(e);
                return Ok(());
            }
        };
        let mut count = 0;

        let pieces_to_mark = self.active_pieces_in_region(has_point).collect_vec();
        for piece in pieces_to_mark {
            count += 1;
            self.mark_piece(piece, piece_type);
        }

        if count != 1 {
            warn_fn(eyre!("{count} pieces were marked with type {name}"));
        }

        Ok(())
    }
    /// Marks the type of a piece.
    pub fn mark_piece(&mut self, piece: Piece, piece_type: PieceType) {
        let old_piece_type = self.pieces[piece].piece_type.replace(piece_type);
        if let Some(old) = old_piece_type {
            self.overwritten_piece_types.push((piece, old));
        }
    }

    /// Unifies piece types using the provided generators.
    pub fn unify_piece_types(
        &mut self,
        transforms: &[pga::Motor],
        warn_fn: &mut impl FnMut(eyre::Error),
    ) {
        let active_pieces = self.active_pieces.iter().collect_vec();
        let mut disjoint_sets = disjoint::DisjointSet::with_len(active_pieces.len());

        // Union-find using the given generators.
        for (i, &piece) in active_pieces.iter().enumerate() {
            let point = self.pieces[piece].interior_point(&self.space).clone();
            for t in transforms {
                let p = t.transform(&point);
                // TODO: compute whether pieces intersect
                // TODO: optimize using space partitioning tree
                for (j, &other_piece) in active_pieces.iter().enumerate() {
                    if APPROX.eq(&p, self.pieces[other_piece].interior_point(&self.space)) {
                        disjoint_sets.join(i, j);
                    }
                }
            }
        }

        for set in disjoint_sets.sets() {
            // Find piece type for each group.
            let applicable_piece_types = set
                .iter()
                .filter_map(|&i| self.pieces[active_pieces[i]].piece_type)
                .sorted()
                .dedup()
                .collect_vec();

            // Warn if there's more than one applicable piece type.
            if applicable_piece_types.len() > 1 {
                let piece_count = set.len();
                let names = applicable_piece_types
                    .iter()
                    .map(|&piece_type| &self.piece_types[piece_type].name)
                    .join(", ");
                warn_fn(eyre!(
                    "{piece_count} pieces are assigned multiple piece types: {names:?}",
                ));
            }

            // Assign the piece type.
            if let Some(&piece_type) = applicable_piece_types.first() {
                for i in set {
                    self.pieces[active_pieces[i]]
                        .piece_type
                        .get_or_insert(piece_type);
                }
            }
        }
    }

    /// Marks pieces without a piece type as having some default type.
    pub fn mark_untyped_pieces(&mut self) -> Result<()> {
        let untyped_pieces = self.untyped_pieces();
        if !untyped_pieces.is_empty() {
            self.piece_type_display_names
                .entry(DEFAULT_PIECE_TYPE_NAME.to_string())
                .or_insert_with(|| DEFAULT_PIECE_TYPE_DISPLAY.to_string());
            let default_piece_type =
                self.get_or_add_piece_type(DEFAULT_PIECE_TYPE_NAME.to_string(), None)?;
            for p in untyped_pieces {
                if self.pieces[p].piece_type.is_none() {
                    self.mark_piece(p, default_piece_type);
                }
            }
        }
        Ok(())
    }

    /// Deletes pieces without a specific piece type.
    pub fn delete_untyped_pieces(&mut self, warn_fn: &mut impl FnMut(eyre::Error)) {
        let untyped_pieces = self.untyped_pieces();
        if untyped_pieces.is_empty() {
            warn_fn(eyre!("no untyped pieces"));
        }
        for piece in untyped_pieces {
            self.active_pieces.remove(&piece);
        }
    }
    fn untyped_pieces(&self) -> Vec<Piece> {
        self.active_pieces
            .iter()
            .filter(|&p| self.pieces[p].piece_type.is_none())
            .collect_vec()
    }

    /// Constructs a mesh and assembles piece & sticker data for the shape.
    pub fn build(&self, warn_fn: &mut impl FnMut(eyre::Error)) -> Result<ShapeBuildOutput> {
        let space = &self.space;
        let ndim = space.ndim();

        let mut mesh = Mesh::new_empty(ndim);
        mesh.color_count = self.colors.len();

        let piece_type_ids_new_to_old: PerPieceType<PieceType> = self
            .pieces
            .iter()
            .filter_map(|(_, p)| p.piece_type)
            .sorted()
            .dedup()
            .collect();
        let mut piece_type_ids_old_to_new = self.piece_types.map_ref(|_, _| None);
        for (new_id, &old_id) in &piece_type_ids_new_to_old {
            piece_type_ids_old_to_new[old_id] = Some(new_id);
        }

        let piece_types = piece_type_ids_new_to_old.map(|_new_id, old_id| {
            let piece_type = &self.piece_types[old_id];
            PieceTypeInfo {
                name: piece_type.name.clone(),
                display: self
                    .piece_type_display_names
                    .get(&piece_type.name)
                    .unwrap_or(&piece_type.name)
                    .clone(),
            }
        });
        if !self.overwritten_piece_types.is_empty() {
            let count = self.overwritten_piece_types.len();
            warn_fn(eyre!("{count} piece types overwritten"));
        }

        // Check for UK spelling, just to troll Luna Harran.
        for piece_type in piece_types.iter_values() {
            if piece_type.name.to_lowercase().contains("centre") {
                warn_fn(eyre!(
                    "incorrect spelling detected in piece type {:?}",
                    piece_type.name
                ));
                return Ok(ShapeBuildOutput::new_empty(self.ndim()));
            }
        }

        // All surfaces have an entry in `hyperplane_to_surface`.
        let mut hyperplane_to_surface: ApproxHashMap<Hyperplane, Surface> =
            ApproxHashMap::new(APPROX);

        // As we construct the mesh, we'll renumber all the pieces and stickers
        // to exclude inactive ones.
        let mut pieces = PerPiece::<PieceInfo>::new();
        let mut piece_polytopes = PerPiece::<PolytopeId>::new();
        let mut stickers = PerSticker::<StickerInfo>::new();
        let mut sticker_planes = PerSticker::<Hyperplane>::new();
        let mut surfaces = PerSurface::<TempSurfaceData>::new();

        // Construct the mesh for each active piece.
        for old_piece_id in self.active_pieces.iter() {
            let piece = &self.pieces[old_piece_id];

            let piece_centroid = space.get(piece.polytope).centroid()?.center();
            // If computing the centroid is expensive, we can replace it with
            // some arbitrary interior point that's easier to compute.

            // IIFE to mimic try_block
            let Some(piece_type) = (|| piece_type_ids_old_to_new[piece.piece_type?])() else {
                warn_fn(eyre!("piece has no piece type"));
                continue;
            };

            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type,
            })?;
            piece_polytopes.push(piece.polytope)?;

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
                    sticker_planes.push(sticker.plane.clone())?;
                    pieces[piece_id].stickers.push(sticker_id);
                }

                let sticker_centroid = space.get(sticker.facet).centroid()?;
                let sticker_plane = sticker.plane;
                let surface_id = match hyperplane_to_surface.entry(sticker_plane.clone()) {
                    approx_collections::hash_map::Entry::Occupied(e) => *e.get(),
                    approx_collections::hash_map::Entry::Vacant(e) => {
                        let surface_id = surfaces.push(TempSurfaceData::new(&sticker_plane)?)?;
                        *e.insert(surface_id)
                    }
                };

                surfaces[surface_id].centroid += sticker_centroid;

                let (polygon_index_range, triangles_index_range, edges_index_range) =
                    build_shape_polygons(
                        space,
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

        for (expected_id, surface_data) in surfaces {
            let surface_id =
                mesh.add_puzzle_surface(&surface_data.centroid.center(), surface_data.normal)?;
            ensure!(surface_id == expected_id.0 as u32);
        }

        let piece_type_hierarchy =
            build_piece_type_hierarchy(&piece_types, &self.piece_type_display_names, warn_fn);

        let piece_type_masks = build_piece_type_masks(&pieces, &piece_types);

        Ok(ShapeBuildOutput {
            mesh,
            pieces,
            piece_polytopes,
            stickers,
            sticker_planes,

            piece_types,
            piece_type_hierarchy,
            piece_type_masks,
        })
    }
}

#[derive(Debug)]
pub struct ShapeBuildOutput {
    pub mesh: Mesh,
    pub pieces: PerPiece<PieceInfo>,
    pub piece_polytopes: PerPiece<PolytopeId>,
    pub stickers: PerSticker<StickerInfo>,
    pub sticker_planes: PerSticker<Hyperplane>,

    pub piece_types: PerPieceType<PieceTypeInfo>,
    pub piece_type_hierarchy: PieceTypeHierarchy,
    pub piece_type_masks: HashMap<String, PieceMask>,
}
impl ShapeBuildOutput {
    fn new_empty(ndim: u8) -> Self {
        Self {
            mesh: Mesh::new_empty(ndim),
            pieces: PerPiece::new(),
            piece_polytopes: PerPiece::new(),
            stickers: PerSticker::new(),
            sticker_planes: PerSticker::new(),

            piece_types: PerPieceType::new(),
            piece_type_hierarchy: PieceTypeHierarchy::new(0),
            piece_type_masks: HashMap::new(),
        }
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
    piece_centroid: &Point,
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
            if normal.dot(init.pos() - piece_centroid) < 0.0 {
                normal = -normal;
                basis.reverse();
            }
        }
        let [u_tangent, v_tangent] = &basis;

        #[cfg(debug_assertions)]
        hypermath::assert_approx_eq!(u_tangent.dot(v_tangent), 0.0);

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
    let shrink_target_by_surface_set: HashMap<&SurfaceSet, Point> = unique_facet_sets_of_vertices
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
    let shrink_vectors = piece_shape.vertex_set().map(|vertex| {
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

fn build_piece_type_hierarchy(
    piece_types: &PerPieceType<PieceTypeInfo>,
    piece_type_display_names: &IndexMap<String, String>,
    warn_fn: &mut impl FnMut(eyre::Report),
) -> PieceTypeHierarchy {
    let mut ret = PieceTypeHierarchy::new(piece_types.len());

    for (path, display_name) in piece_type_display_names {
        if let Err(e) = ret.set_display(path, display_name.clone()) {
            warn_fn(e);
        }
    }

    for (id, piece_type_info) in piece_types {
        if let Err(e) = ret.set_piece_type_id(&piece_type_info.name, id) {
            warn_fn(e);
        }
    }

    for path in ret.paths_with_no_display_name() {
        warn_fn(eyre!("piece type {path:?} has no display name"));
    }

    ret
}

fn build_piece_type_masks(
    pieces: &PerPiece<PieceInfo>,
    piece_types: &PerPieceType<PieceTypeInfo>,
) -> HashMap<String, PieceMask> {
    let mut ret = HashMap::new();

    for (piece_type_id, piece_type_info) in piece_types {
        let piece_iter =
            pieces.iter_filter(|_piece_id, piece_info| piece_info.piece_type == piece_type_id);
        let mask = PieceMask::from_iter(pieces.len(), piece_iter);

        for path in PieceTypeHierarchy::path_prefixes(&piece_type_info.name) {
            match ret.entry(path.to_owned()) {
                hash_map::Entry::Occupied(e) => {
                    *e.into_mut() |= &mask;
                }
                hash_map::Entry::Vacant(e) => {
                    e.insert(mask.clone());
                }
            }
        }
    }

    ret
}

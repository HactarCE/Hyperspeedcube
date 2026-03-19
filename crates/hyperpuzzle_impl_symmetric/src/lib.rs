//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

#![allow(missing_docs)]

use std::collections::{HashMap, hash_map};
use std::sync::{Arc, Weak};

use eyre::{Result, bail, ensure};
use hypermath::prelude::*;
use hyperpuzzle_core::group::{CoxeterMatrix, GroupAction, GroupElementId, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::builder::ColorSystemBuilder;
use hyperpuzzle_impl_nd_euclid::{
    NdEuclidPuzzleGeometry, NdEuclidPuzzleStateRenderData, NdEuclidPuzzleUiData,
};
use hypershape::{ElementId, Space};

mod geometry;

use geometry::{PieceFacetGeometry, PieceGeometry, PolytopeGeometry, StickerData, SurfaceGeometry};
use itertools::Itertools;

// pub fn product_abstract_puzzles(puz1: AbstractPuzzle, puz2: AbstractPuzzle) -> AbstractPuzzle {}

// faces = faces * verts + edges * edges + verts * faces
//
// sticker is ONE OF:
// - list of triangles
// - edge
// - single vertex
//
// piece is a polytope
// - list of facets
// - each facet is a list of triangles, edges, and vertices
// - each facet also has optional sticker info

hypuz_util::typed_index_struct! {
    pub struct NamedPoint(u16);
}

#[derive(Debug)]
pub struct NameBiMap<I> {
    id_to_name: TiVec<I, String>,
    name_to_id: HashMap<String, I>,
}
impl<I: TypedIndex> NameBiMap<I> {
    pub fn new() -> Self {
        Self {
            id_to_name: TiVec::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn concat(a: &Self, b: &Self) -> Self {
        let lift_a = |i: I| i;
        let lift_b = |i: I| I::try_from_index(i.to_index() + a.len()).expect("overflow");
        Self {
            id_to_name: std::iter::chain(a.id_to_name.iter_values(), b.id_to_name.iter_values())
                .cloned()
                .collect(),
            name_to_id: std::iter::chain(
                a.name_to_id
                    .iter()
                    .map(|(a_name, &a_index)| (a_name.clone(), lift_a(a_index))),
                b.name_to_id
                    .iter()
                    .map(|(b_name, &b_index)| (b_name.clone(), lift_b(b_index))),
            )
            .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.id_to_name.len()
    }
}

pub(crate) fn make_partial_symmetric_puzzle(
    ndim: u8,
    symmetry: IsometryGroup,
    carve_planes: &[Hyperplane],
    slice_planes: &[Hyperplane],
) -> Result<PartialSymmetricPuzzle> {
    let generator_motors = symmetry.generator_motors().map_ref(|_, &m| m.clone());

    let space = Space::new(ndim);

    let mut pieces = PerPiece::from_iter([TempPiece {
        polytope: space
            .add_primordial_cube(hypershape::PRIMORDIAL_CUBE_RADIUS)?
            .as_element()
            .id(),
        stickers: vec![],
    }]);

    let mut surface_geometries = PerSurface::new();

    for init_plane in carve_planes {
        for plane in hypergroup::orbit_geometric(&*generator_motors, init_plane.clone()) {
            let new_surface = surface_geometries.push(SurfaceGeometry {
                ndim,
                centroid: Point::ORIGIN,
                normal: plane.normal().clone(),
            })?;
            let cut = hypershape::Cut::carve(&space, plane);
            pieces = cut_pieces(pieces, cut, Some(new_surface))?;
            if pieces.is_empty() {
                bail!("empty geometry");
            }
        }
    }

    ensure!(pieces.len() == 1, "expected exactly 1 piece");

    for sticker_data in &pieces[Piece(0)].stickers {
        surface_geometries[sticker_data.surface].centroid =
            space.get(sticker_data.polytope).centroid()?.center();
    }

    for init_plane in slice_planes {
        for plane in hypergroup::orbit_geometric(&*generator_motors, init_plane.clone()) {
            let cut = hypershape::Cut::slice(&space, plane);
            pieces = cut_pieces(pieces, cut, None)?;
            if pieces.is_empty() {
                bail!("empty geometry");
            }
        }
    }

    let mut piece_geometries = PerPiece::new();
    let mut sticker_pieces = PerSticker::new();
    let mut sticker_surfaces = PerSticker::new();

    for (piece, piece_data) in pieces {
        let piece_polytope = space.get(piece_data.polytope);
        let mut facet_id_to_sticker = HashMap::new();
        let mut sticker_set = StickerSet::new();
        for sticker_data in piece_data.stickers {
            let sticker_id = sticker_pieces.push(piece)?;
            sticker_surfaces.push(sticker_data.surface)?;
            facet_id_to_sticker.insert(sticker_data.polytope, sticker_id);
            sticker_set.insert(sticker_id);
        }

        let facets = piece_polytope
            .boundary()
            .map(|b| (b, facet_id_to_sticker.get(&b.id()).copied()))
            .filter(|(_, sticker)| ndim <= 3 || sticker.is_some()) // remove internals in 4D+
            .map(|(b, sticker)| {
                eyre::Ok(PieceFacetGeometry {
                    polytope: PolytopeGeometry::from_polytope_element(b)?,
                    sticker_data: sticker.map(|sticker| StickerData {
                        surface: sticker_surfaces[sticker],
                    }),
                })
            })
            .try_collect()?;

        piece_geometries.push(PieceGeometry {
            polytope: PolytopeGeometry::from_polytope_element(piece_polytope)?,
            centroid: piece_polytope.centroid()?.center(),
            facets,
        })?;
    }

    Ok(PartialSymmetricPuzzle {
        ndim,

        named_point_group_action: symmetry.action_on_points(&TiVec::new())?,
        named_point_names: NameBiMap::new(),

        axis_group_action: symmetry.action_on_points(&TiVec::new())?,
        axis_names: NameBiMap::new(),

        sticker_color_action: symmetry.action_on_points(&TiVec::new())?,
        sticker_color_names: NameBiMap::new(),

        piece_geometries,
        surface_geometries,

        isometry_group: symmetry.clone(),
    })
}

struct TempSticker {
    polytope: ElementId,
    surface: Surface,
}

struct TempPiece {
    polytope: ElementId,
    stickers: Vec<TempSticker>,
}

fn cut_pieces(
    pieces: PerPiece<TempPiece>,
    mut cut: hypershape::Cut,
    new_sticker_surface: Option<Surface>,
) -> Result<PerPiece<TempPiece>> {
    let mut new_pieces = PerPiece::new();
    for (_, piece) in pieces {
        let mut new_inside_stickers = vec![];
        let mut new_outside_stickers = vec![];

        // Cut stickers
        for sticker in piece.stickers {
            match cut.cut(sticker.polytope)? {
                hypershape::ElementCutOutput::Flush => (),
                hypershape::ElementCutOutput::NonFlush {
                    inside, outside, ..
                } => {
                    let surface = sticker.surface;
                    if let Some(polytope) = inside {
                        new_inside_stickers.push(TempSticker { polytope, surface });
                    }
                    if let Some(polytope) = outside {
                        new_outside_stickers.push(TempSticker { polytope, surface });
                    }
                }
            }
        }

        // Cut piece
        match cut.cut(piece.polytope)? {
            hypershape::ElementCutOutput::Flush => bail!("piece is flush with cut"),
            hypershape::ElementCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(polytope) = intersection
                    && let Some(surface) = new_sticker_surface
                {
                    new_inside_stickers.push(TempSticker { polytope, surface });
                    new_outside_stickers.push(TempSticker { polytope, surface });
                }

                if let Some(polytope) = inside {
                    let stickers = new_inside_stickers;
                    new_pieces.push(TempPiece { polytope, stickers })?;
                }
                if let Some(polytope) = outside {
                    let stickers = new_outside_stickers;
                    new_pieces.push(TempPiece { polytope, stickers })?;
                }
            }
        }
    }
    Ok(new_pieces)
}

#[derive(Debug)]
struct PartialSymmetricPuzzle {
    ndim: u8,

    named_point_group_action: GroupAction<NamedPoint>,
    named_point_names: NameBiMap<NamedPoint>,

    axis_group_action: GroupAction<Axis>,
    axis_names: NameBiMap<Axis>,

    sticker_color_action: GroupAction<Color>,
    sticker_color_names: NameBiMap<Color>,

    piece_geometries: PerPiece<PieceGeometry>,
    surface_geometries: PerSurface<SurfaceGeometry>,

    isometry_group: IsometryGroup,
}

pub fn direct_product_vectors(
    a_ndim: u8,
    b_ndim: u8,
    a: impl VectorRef,
    b: impl VectorRef,
) -> Vector {
    std::iter::chain(a.iter_ndim(a_ndim), b.iter_ndim(b_ndim)).collect()
}

pub fn direct_product_points(a_ndim: u8, b_ndim: u8, a: &Point, b: &Point) -> Point {
    std::iter::chain(
        a.as_vector().iter_ndim(a_ndim),
        b.as_vector().iter_ndim(b_ndim),
    )
    .collect()
}

impl PartialSymmetricPuzzle {
    pub fn piece_count(&self) -> usize {
        self.piece_geometries.len()
    }
    pub fn surface_count(&self) -> usize {
        self.surface_geometries.len()
    }

    pub fn direct_product(
        a: &PartialSymmetricPuzzle,
        b: &PartialSymmetricPuzzle,
    ) -> Result<PartialSymmetricPuzzle> {
        let ndim = a.ndim + b.ndim;

        let piece_geometries = itertools::iproduct!(
            a.piece_geometries.iter_values(),
            b.piece_geometries.iter_values(),
        )
        .map(|(a_piece, b_piece)| {
            PieceGeometry::direct_product(a_piece, b_piece, a.surface_count())
        })
        .collect();

        // Assume that the centroid of each entire puzzle is the origin.
        let surface_geometries = std::iter::chain(
            a.surface_geometries
                .iter_values()
                .map(|a_surface| a_surface.lift_by_ndim(0, b.ndim)),
            b.surface_geometries
                .iter_values()
                .map(|b_surface| b_surface.lift_by_ndim(a.ndim, 0)),
        )
        .collect();

        Ok(PartialSymmetricPuzzle {
            ndim,

            named_point_group_action: GroupAction::product([
                &a.named_point_group_action,
                &b.named_point_group_action,
            ])?,
            named_point_names: NameBiMap::concat(&a.named_point_names, &b.named_point_names),

            axis_group_action: GroupAction::product([&a.axis_group_action, &b.axis_group_action])?,
            axis_names: NameBiMap::concat(&a.axis_names, &b.axis_names),

            sticker_color_action: GroupAction::product([
                &a.sticker_color_action,
                &b.sticker_color_action,
            ])?,
            sticker_color_names: NameBiMap::concat(&a.sticker_color_names, &b.sticker_color_names),

            piece_geometries,
            surface_geometries,

            isometry_group: IsometryGroup::product([&a.isometry_group, &b.isometry_group])?,
        })
    }

    pub fn build(&self) -> Result<Arc<Puzzle>> {
        let ndim = self.ndim;
        let piece_count = self.piece_count();

        let mut mesh = Mesh::new_empty(self.ndim);

        for (_surface, surface_geometry) in &self.surface_geometries {
            mesh.add_puzzle_surface(&surface_geometry.centroid, &surface_geometry.normal)?;
        }
        let dummy_surface = mesh.add_puzzle_surface(&Point::ORIGIN, Vector::EMPTY)?; // dummy surface for internals and 2D puzzles

        for (piece, piece_geometry) in &self.piece_geometries {
            let piece_internals_polygons_start = mesh.polygon_count;
            let piece_internals_triangles_start = mesh.triangle_count() as u32;
            let piece_internals_edges_start = mesh.edge_count() as u32;

            let mut piece_internals_polygons_end = piece_internals_polygons_start;
            let mut piece_internals_triangles_end = piece_internals_triangles_start;
            let mut piece_internals_edges_end = piece_internals_edges_start;

            let mut facet_geometries = piece_geometry
                .facets
                .iter()
                .map(|f| (&f.polytope, &f.sticker_data))
                .collect_vec();

            // Generate internals in 2D
            if ndim == 2 {
                facet_geometries.push((&piece_geometry.polytope, &None));
            }

            // Iterate over internals, then stickers
            facet_geometries.sort_unstable_by_key(|(_, sticker_data)| sticker_data.is_some());

            for (facet_geometry, sticker_data) in facet_geometries {
                let polygons_start = mesh.polygon_count;
                let triangles_start = mesh.triangle_count() as u32;
                let edges_start = mesh.edge_count() as u32;

                let surface_id_in_mesh = match sticker_data {
                    Some(sticker_data) => sticker_data.surface,
                    None => dummy_surface, // internal
                };

                // Add polygons and triangles.
                let dummy_polygon = mesh.next_polygon_id()?; // for edges with no polygon
                let mut vertex_map = HashMap::new();
                let mut i = 0;
                for &polygon_size in &facet_geometry.polygon_sizes {
                    let polygon_id_in_mesh = mesh.next_polygon_id()?;

                    let j = i + polygon_size as usize;
                    let polygon = &facet_geometry.polygon_verts[i..j];

                    // Calculate tangent vectors.
                    ensure!(polygon.len() >= 3, "mesh polygon is too small");
                    let [a, b, c] = [0, 1, 2].map(|n| facet_geometry.verts.get(polygon[n]));
                    // IIFE to mimic try_block
                    let (mut u_tangent, mut v_tangent) = (|| {
                        let u = (b - &a).normalize()?;
                        let v = (c - &a).rejected_from(&u)?.normalize()?;
                        Some((u, v))
                    })()
                    .unwrap_or_default(); // give up and return zero

                    // Fix polygon orientation in 2D and 3D.
                    if ndim == 2 || ndim == 3 {
                        let polyhedron_center = if ndim == 2 {
                            &point![0.0, 0.0, -1.0]
                        } else {
                            &piece_geometry.centroid
                        };
                        if u_tangent
                            .cross_product_3d(&v_tangent)
                            .dot(a - polyhedron_center)
                            .is_sign_negative()
                        {
                            std::mem::swap(&mut u_tangent, &mut v_tangent);
                        }
                    }

                    let polygon_start = mesh.vertex_count() as u32;
                    for &vertex_id in polygon {
                        let vertex_id_in_mesh = mesh.add_puzzle_vertex(MeshVertexData {
                            position: &facet_geometry.verts.get(vertex_id),
                            u_tangent: &u_tangent,
                            v_tangent: &v_tangent,
                            sticker_shrink_vector: &Vector::zero(0), // TODO
                            piece_id: piece,
                            surface_id: surface_id_in_mesh,
                            polygon_id: polygon_id_in_mesh,
                        })?;
                        vertex_map.entry(vertex_id).or_insert(vertex_id_in_mesh);
                    }

                    for k in 2..polygon_size as u32 {
                        mesh.triangles
                            .push([0, k - 1, k].map(|q| polygon_start + q));
                    }

                    i = j;
                }

                // Add edges.
                let mut get_or_add_vertex_for_edge = |mesh: &mut Mesh, v| {
                    eyre::Ok(match vertex_map.entry(v) {
                        hash_map::Entry::Occupied(entry) => *entry.get(),
                        hash_map::Entry::Vacant(entry) => {
                            *entry.insert(mesh.add_puzzle_vertex(MeshVertexData {
                                position: &facet_geometry.verts.get(v),
                                u_tangent: &Vector::EMPTY,
                                v_tangent: &Vector::EMPTY,
                                sticker_shrink_vector: &Vector::zero(0), // TODO
                                piece_id: piece,
                                surface_id: surface_id_in_mesh,
                                polygon_id: dummy_polygon,
                            })?)
                        }
                    })
                };
                for &[v1, v2] in &facet_geometry.edges {
                    let v1 = get_or_add_vertex_for_edge(&mut mesh, v1)?;
                    let v2 = get_or_add_vertex_for_edge(&mut mesh, v2)?;
                    mesh.edges.push([v1, v2]);
                }

                let polygons_end = mesh.polygon_count;
                let triangles_end = mesh.triangle_count() as u32;
                let edges_end = mesh.edge_count() as u32;

                if sticker_data.is_some() {
                    mesh.add_sticker(
                        polygons_start..polygons_end,
                        triangles_start..triangles_end,
                        edges_start..edges_end,
                    )?;
                } else {
                    // internals are sorted before stickers
                    piece_internals_polygons_end = polygons_end;
                    piece_internals_triangles_end = triangles_end;
                    piece_internals_edges_end = edges_end;
                }
            }

            mesh.add_piece(
                &piece_geometry.centroid,
                piece_internals_polygons_start..piece_internals_polygons_end,
                piece_internals_triangles_start..piece_internals_triangles_end,
                piece_internals_edges_start..piece_internals_edges_end,
            )?;
        }

        let mut stickers = PerSticker::new();
        let pieces = self.piece_geometries.try_map_ref(|piece, piece_geometry| {
            let stickers = piece_geometry
                .facets
                .iter()
                .filter_map(|f| f.sticker_data.as_ref())
                .map(|sticker_data| {
                    let color = Color(sticker_data.surface.0); // TODO: color should not be based on surface
                    stickers.push(StickerInfo { piece, color })
                })
                .try_collect()?;
            eyre::Ok(PieceInfo {
                stickers,
                piece_type: PieceType(0),
            })
        })?;

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new_with_len(piece_count),
            piece_centroids: self
                .piece_geometries
                .map_ref(|_, piece_geometries| piece_geometries.centroid.clone()),

            planes: vec![Hyperplane::new(vector![1.0], 0.0).unwrap()],
            sticker_planes: stickers.map_ref(|_, _| 0),

            mesh,

            axis_vectors: Arc::new(PerAxis::new()),
            axis_layer_depths: PerAxis::new(),
            twist_transforms: Arc::new(PerTwist::new()),

            gizmo_twists: PerGizmoFace::new(),
        });
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);

        // TODO: proper color system
        let mut colors = ColorSystemBuilder::new_ad_hoc("unknown_product_puzzle");
        for _ in &self.surface_geometries {
            colors.add(None, |e| log::warn!("{e}"))?;
        }
        let colors = Arc::new(colors.build(None, None, &mut |e| log::warn!("{e}"))?);

        let piece_types = PerPieceType::from_iter([PieceTypeInfo {
            name: "piece".to_string(),
            display: "Piece".to_ascii_lowercase(),
        }]);
        let mut piece_type_hierarchy = PieceTypeHierarchy::new(6);
        for (id, piece_type_info) in &piece_types {
            if let Err(e) = piece_type_hierarchy.set_piece_type_id(&piece_type_info.name, id) {
                log::warn!("{e}");
            }
        }

        let piece_type_masks =
            HashMap::from_iter([("piece".to_string(), PieceMask::new_full(piece_count))]);

        let axes = Arc::new(AxisSystem::new_empty());
        let twists = Arc::new(TwistSystem::new_empty(&axes));

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(PuzzleListMetadata {
                id: "symmetric_puzzle_test".to_string(),
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: 1,
                },
                name: "Symmetric Puzzle Test".to_string(),
                aliases: vec![],
                tags: TagSet::new(),
            }),
            view_prefs_set: Some(PuzzleViewPreferencesSet::Perspective(match ndim {
                ..=3 => PerspectiveDim::Dim3D,
                4.. => PerspectiveDim::Dim4D,
            })),
            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,
            colors,
            can_scramble: false,
            full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
            axis_layers: PerAxis::new(),
            twists,
            ui_data,
            new: Box::new({
                let grip_group = self.isometry_group.clone();
                move |ty| {
                    SymmetricPuzzleState {
                        ty,
                        grip_group: grip_group.clone(),
                        attitudes: PerPiece::new_with_len(piece_count),
                    }
                    .into()
                }
            }),
            random_move: Box::new(|rng| None),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct SymmetricPuzzleState {
    ty: Arc<Puzzle>,
    grip_group: IsometryGroup,
    attitudes: PerPiece<GroupElementId>,
}

impl PuzzleState for SymmetricPuzzleState {
    fn ty(&self) -> &std::sync::Arc<Puzzle> {
        &self.ty
    }

    fn clone_dyn(&self) -> BoxDynPuzzleState {
        self.clone().into()
    }

    fn do_twist(&self, twist: &Move) -> std::result::Result<Self, Vec<Piece>>
    where
        Self: Sized,
    {
        todo!()
    }

    fn do_twist_dyn(&self, twist: &Move) -> std::result::Result<BoxDynPuzzleState, Vec<Piece>> {
        todo!()
    }

    fn is_solved(&self) -> bool {
        true
    }

    fn compute_grip(&self, axis: Axis, layers: &LayerMask) -> PerPiece<WhichSide> {
        todo!() // TODO
    }

    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!() // TODO
    }

    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        todo!() // TODO
    }

    fn render_data(&self) -> BoxDynPuzzleStateRenderData {
        NdEuclidPuzzleStateRenderData {
            piece_transforms: self.attitudes.map_ref(|_, &e| self.grip_group.motor(e)),
        }
        .into()
    }

    fn partial_twist_render_data(&self, twist: &Move, t: f32) -> BoxDynPuzzleStateRenderData {
        todo!()
    }

    fn animated_render_data(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData {
        todo!()
    }
}

fn autonames() -> impl Iterator<Item = String> {
    (0..)
        .map(hyperpuzzle_core::notation::family::UppercaseGreekPrefix)
        .map(|prefix| prefix.to_string())
}

pub fn add_puzzles_to_catalog(catalog: &hyperpuzzle_core::Catalog) -> Result<()> {
    catalog.add_puzzle(Arc::new(PuzzleSpec {
        meta: Arc::new(PuzzleListMetadata {
            id: "symmetric_puzzle_test".to_string(),
            version: Version {
                major: 0,
                minor: 0,
                patch: 1,
            },
            name: "Symmetric Puzzle Test".to_string(),
            aliases: vec![],
            tags: TagSet::new(),
        }),
        build: Box::new(|build_ctx| {
            // IIFE to mimic try_block
            (|| -> Result<_> {
                Ok(PartialSymmetricPuzzle::direct_product(
                    // &shallow_polygon(5)?,
                    // &shallow_line()?,
                    // // &half_cut_line()?,
                    // &half_cut_line()?,
                    // &shallow_polygon(6)?,
                    //
                    // &shallow_polygon(3)?,
                    // &PartialSymmetricPuzzle::direct_product(
                    &shallow_polygon(5)?,
                    &shallow_polygon(6)?,
                    // )?,
                    //
                    // &rubiks_cube()?,
                    // &megaminx()?,
                    //
                    // &megaminx()?,
                    // &megaminx()?,
                )?
                .build()?)
            })()
            .map(Redirectable::Direct)
            .map_err(|e| e.to_string())
        }),
    }))?;
    Ok(())
}

fn rubiks_cube() -> Result<PartialSymmetricPuzzle> {
    make_partial_symmetric_puzzle(
        3,
        CoxeterMatrix::B(3)?.isometry_group()?,
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0 / 3.0).unwrap()],
    )
}

fn shallow_polygon(n: u16) -> Result<PartialSymmetricPuzzle> {
    let pi_div_n = std::f64::consts::PI as Float / n as Float;

    let edge_length = 2.0 * pi_div_n.tan();
    let edge_depth = (2.0 * pi_div_n).sin() * edge_length;
    let cut_depth = 1.0 - edge_depth / 3.0;
    make_partial_symmetric_puzzle(
        2,
        CoxeterMatrix::I(n)?.isometry_group()?,
        &[Hyperplane::new(vector![0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![0.0, 1.0], cut_depth).unwrap()],
    )
}

fn shallow_line() -> Result<PartialSymmetricPuzzle> {
    make_partial_symmetric_puzzle(
        1,
        CoxeterMatrix::A(1)?.isometry_group()?,
        &[Hyperplane::new(vector![1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![1.0], 1.0 / 3.0).unwrap()],
    )
}

fn half_cut_line() -> Result<PartialSymmetricPuzzle> {
    make_partial_symmetric_puzzle(
        1,
        CoxeterMatrix::A(1)?.isometry_group()?,
        &[Hyperplane::new(vector![1.0], 1.0).unwrap()],
        &[Hyperplane::new(vector![1.0], 0.0).unwrap()],
    )
}

fn megaminx() -> Result<PartialSymmetricPuzzle> {
    make_partial_symmetric_puzzle(
        3,
        CoxeterMatrix::H3().isometry_group()?,
        &[Hyperplane::new(vector![0.0, 0.0, 1.0], 1.0).unwrap()],
        &[Hyperplane::new(
            vector![0.0, 0.0, 1.0],
            std::f64::consts::GOLDEN_RATIO.recip(),
        )
        .unwrap()],
    )
}

fn simplex_a(ndim: u8) -> Result<PartialSymmetricPuzzle> {
    make_partial_symmetric_puzzle(
        ndim,
        CoxeterMatrix::A(ndim)?.isometry_group()?,
        &[Hyperplane::new(Vector::unit(ndim - 1), 1.0).unwrap()],
        &[Hyperplane::new(Vector::unit(ndim - 1), 0.0).unwrap()],
    )
}

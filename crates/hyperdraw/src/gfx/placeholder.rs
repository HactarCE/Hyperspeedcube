//! Special model that displays when a puzzle mesh is empty.

use eyre::{OptionExt, Result};
use hypermath::prelude::*;
use hyperpuzzle_core::*;
use itertools::Itertools;
use smallvec::{smallvec, SmallVec};

use super::DrawParams;
use crate::{IterCyclicPairsExt, PieceStyleValues};

pub fn pieces() -> PerPiece<SmallVec<[Sticker; 8]>> {
    vec![smallvec![Sticker(0), Sticker(1), Sticker(2)]].into()
}
pub fn sticker_colors() -> PerSticker<Color> {
    vec![Color(0), Color(1), Color(2)].into()
}

pub fn modify_draw_params(mut draw_params: DrawParams) -> DrawParams {
    draw_params.cam.view_preset.value.outline_light_intensity = 1.0;
    draw_params.cam.view_preset.value.face_light_intensity = 1.0;
    draw_params.cam.view_preset.value.piece_explode = 0.0;

    draw_params.sticker_colors = vec![
        [0xff, 0xcc, 0x4d], // base shape
        [0x66, 0x45, 0x00], // line set 1
        [0xf4, 0x90, 0x0c], // line set 2
    ];

    draw_params.piece_styles = vec![(
        PieceStyleValues {
            face_opacity: 255,
            face_color: hyperprefs::StyleColorMode::FromSticker,
            outline_opacity: 255,
            outline_color: hyperprefs::StyleColorMode::FromSticker,
            outline_lighting: true,
            outline_size: 18.0,
        },
        PieceMask::new_full(1),
    )];

    draw_params.piece_transforms = PerPiece::from_iter([Matrix::ident(draw_params.ndim)]);

    draw_params
}

#[allow(clippy::single_range_in_vec_init)]
pub fn placeholder_mesh(ndim: u8) -> Result<Mesh> {
    let mut mesh = Mesh {
        ndim,

        color_count: 3,
        polygon_count: 0, // will be modified
        sticker_count: 3,
        piece_count: 1,
        puzzle_surface_count: 3,
        puzzle_vertex_count: 0, // will be modified

        gizmo_face_count: 0,
        gizmo_surface_count: 0,
        gizmo_vertex_count: 0,

        // All these will be modified
        vertex_positions: vec![],
        u_tangents: vec![],
        v_tangents: vec![],
        sticker_shrink_vectors: vec![],
        piece_ids: vec![],
        surface_ids: vec![],
        polygon_ids: vec![],

        piece_centroids: vec![0.0; 3],
        surface_centroids: vec![0.0; 9], // 3 surfaces
        surface_normals: vec![0.0; 9],

        sticker_polygon_ranges: PerSticker::new(), // will be modified
        piece_internals_polygon_ranges: PerPiece::from_iter([0..0]),

        triangles: vec![],                          // will be modified
        sticker_triangle_ranges: PerSticker::new(), // will be modified
        piece_internals_triangle_ranges: PerPiece::from_iter([0..0]),
        gizmo_triangle_ranges: PerGizmoFace::new(),

        edges: vec![],                          // will be modified
        sticker_edge_ranges: PerSticker::new(), // will be modified
        piece_internals_edge_ranges: PerPiece::from_iter([0..0]),
        gizmo_edge_ranges: PerGizmoFace::new(),
    };

    add_sticker(&mut mesh, add_base_shape)?;
    add_sticker(&mut mesh, add_line_set_1)?;
    add_sticker(&mut mesh, add_line_set_2)?;

    Ok(mesh)
}

fn add_sticker(mesh: &mut Mesh, f: fn(&mut Mesh) -> Result<()>) -> Result<()> {
    let polygons_start = mesh.polygon_count;
    let tri_start = mesh.triangle_count() as u32;
    let edges_start = mesh.edge_count() as u32;
    f(mesh)?;
    let polygons_end = mesh.polygon_count;
    let tri_end = mesh.triangle_count() as u32;
    let edges_end = mesh.edge_count() as u32;
    mesh.add_sticker(
        polygons_start..polygons_end,
        tri_start..tri_end,
        edges_start..edges_end,
    )
}

fn add_base_shape(mesh: &mut Mesh) -> Result<()> {
    let polygon_verts = [
        (0.31552208, 0.2391844),
        (0.87531816, 0.1908384),
        (0.91094136, 0.2290064),
        (1.1195925, 0.2264644),
        (1.5547068, 0.368958),
        (1.7811698, 0.5547086),
        (2.0, 0.8524189),
        (2.0, 1.5725204),
        (1.870229, 1.6386776),
        (1.755725, 1.6335896),
        (1.3206107, 1.8066176),
        (0.95674326, 1.8091616),
        (0.33078934, 1.3791311),
        (0.01017886, 0.97455102),
        (0.0, 0.7862561),
        (0.1552163, 0.4096659),
    ];
    let z_coordinates = [-0.95, -0.75, -0.5, -0.15, 0.15, 0.5, 0.75, 0.95];
    let layers = z_coordinates.map(|z: f64| {
        let scale = (1.0 - (z * z * z).abs()).cbrt();
        polygon_verts.map(|xy| transform_point(xy, z * 0.4, scale))
    });

    general_polygon(mesh, layers.first().ok_or_eyre("too few layers")?)?;
    general_polygon(mesh, layers.last().ok_or_eyre("too few layers")?.iter())?;
    for i in 1..layers.len() {
        let l1 = &layers[i - 1];
        let l2 = &layers[i];
        for ((a, b), (c, d)) in std::iter::zip(l1, l2).cyclic_pairs() {
            quad(mesh, [a, b, c, d])?;
        }
    }
    Ok(())
}

fn add_line_set_1(mesh: &mut Mesh) -> Result<()> {
    // top left
    add_edge_seq(
        mesh,
        &[
            [0.911, 0.451, 0.350],
            [0.750, 0.251, 0.250],
            [0.404, 0.424, 0.300],
            [0.495, 0.440, 0.325],
        ],
    )?;
    add_ball(mesh, [0.6739, 0.5580, 0.400])?;

    // top right
    add_edge_seq(mesh, &[[1.058, 0.751, 0.400], [1.506, 0.703, 0.350]])?;
    add_ball(mesh, [1.3342, 0.8456, 0.400])?;

    // bottom
    add_edge_seq(
        mesh,
        &[
            [0.634, 0.975, 0.400],
            [0.837, 1.013, 0.400],
            [1.837, 1.361, 0.300],
        ],
    )?;

    Ok(())
}

fn add_line_set_2(mesh: &mut Mesh) -> Result<()> {
    // from top to bottom
    add_edge_seq(
        mesh,
        &[
            [0.426, 1.515, 0.150],
            [0.491, 1.335, 0.400],
            [0.483, 1.191, 0.400],
        ],
    )?;
    add_edge_seq(mesh, &[[0.426, 1.515, 0.150], [1.334, 1.442, 0.400]])?; // long
    add_edge_seq(
        mesh,
        &[
            [0.426, 1.515, 0.150],
            [0.565, 1.563, 0.300],
            [0.673, 1.512, 0.400],
            [0.781, 1.650, 0.300],
        ],
    )?;

    Ok(())
}

fn quad(mesh: &mut Mesh, verts: [&Vector; 4]) -> Result<()> {
    let [a, b, c, d] = verts;
    let u = b - a;
    let v = c - a;

    let polygon_id = mesh.next_polygon_id()?;
    let a = add_vertex(mesh, a, &u, &v, polygon_id)?;
    let b = add_vertex(mesh, b, &u, &v, polygon_id)?;
    let c = add_vertex(mesh, c, &u, &v, polygon_id)?;
    let d = add_vertex(mesh, d, &u, &v, polygon_id)?;
    mesh.triangles.push([a, b, c]);
    mesh.triangles.push([b, c, d]);
    Ok(())
}

fn general_polygon<'a>(mesh: &mut Mesh, verts: impl IntoIterator<Item = &'a Vector>) -> Result<()> {
    let mut verts = verts.into_iter().peekable();
    let z = verts.peek().ok_or_eyre("too few vertices in polygon")?[2];
    let u = vector![1.0, 0.0, 0.0];
    let v = vector![0.0, z.signum(), 0.0];

    let polygon_id = mesh.next_polygon_id()?;
    let center = add_vertex(mesh, &vector![0.0, 0.0, z], &u, &v, polygon_id)?;
    let polygon_verts: Vec<u32> = verts
        .map(|p| add_vertex(mesh, p, &u, &v, polygon_id))
        .try_collect()?;
    for (a, b) in polygon_verts.into_iter().cyclic_pairs() {
        mesh.triangles.push([a, b, center]);
    }
    Ok(())
}

fn add_ball(mesh: &mut Mesh, [x, y, z]: [f64; 3]) -> Result<()> {
    add_edge_seq(mesh, &[[x, y, z], [x, y + 0.01, z - 0.4]])
}

fn add_edge_seq(mesh: &mut Mesh, edges: &[[f64; 3]]) -> Result<()> {
    for (a, b) in edges
        .iter()
        .map(|&[x, y, z]| transform_point((x, y), z, 1.0))
        .tuple_windows()
    {
        add_edge(mesh, &a, &b)?;
    }
    Ok(())
}

fn add_edge(mesh: &mut Mesh, a: &Vector, b: &Vector) -> Result<()> {
    let polygon_id = mesh.next_polygon_id()?;
    let a = add_vertex(mesh, a, &Vector::EMPTY, &Vector::EMPTY, polygon_id)?;
    let b = add_vertex(mesh, b, &Vector::EMPTY, &Vector::EMPTY, polygon_id)?;
    mesh.edges.push([a, b]);
    Ok(())
}

fn add_vertex(
    mesh: &mut Mesh,
    position: &Vector,
    u_tangent: &Vector,
    v_tangent: &Vector,
    polygon_id: u32,
) -> Result<u32> {
    mesh.add_puzzle_vertex(MeshVertexData {
        position,
        u_tangent,
        v_tangent,
        sticker_shrink_vector: &Vector::EMPTY,
        piece_id: Piece(0),
        surface_id: Surface(0),
        polygon_id,
    })
}

fn transform_point((x, y): (f64, f64), z: f64, xy_scale: f64) -> Vector {
    vector![(x - 1.0) * xy_scale, (1.0 - y) * xy_scale, z]
}

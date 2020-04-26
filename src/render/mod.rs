//! Rendering logic.

use cgmath::{Deg, Matrix4, Perspective, Vector4};
use glium::{DrawParameters, Surface};
use std::collections::HashSet;

mod cache;
mod colors;
mod shaders;
mod verts;

use super::puzzle::{self, traits::*, PuzzleController, PuzzleEnum, PuzzleType};
use super::DISPLAY;
use cache::CACHE;
use verts::*;

const STICKER_SIZE: f32 = 0.9;
const NEAR_PLANE: f32 = 3.0;
const FAR_PLANE: f32 = 20.0;

const VIEW_ANGLE: f32 = 35.0;
// const OUTLINE_COLOR: Option<[f32; 4]> = None;
const OUTLINE_COLOR: Option<[f32; 4]> = Some(colors::OUTLINE_BLACK);
// const OUTLINE_COLOR: Option<[f32; 4]> = colors::OUTLINE_WHITE;
const LINE_WIDTH: f32 = 2.0;

pub fn setup_puzzle(puzzle_type: PuzzleType) {
    match puzzle_type {
        PuzzleType::Rubiks3D => _setup_puzzle::<puzzle::Rubiks3D>(),
    }
}

pub fn draw_puzzle(target: &mut glium::Frame, puzzle: &PuzzleEnum) -> Result<(), glium::DrawError> {
    match puzzle {
        PuzzleEnum::Rubiks3D(cube) => _draw_puzzle(target, cube),
    }
}

fn _setup_puzzle<P: PuzzleTrait>() {
    let mut c = CACHE.borrow_mut();
    let mut surface_indices = vec![];
    let mut outline_indices = vec![];
    let mut base = 0;
    for _ in P::Sticker::iter() {
        // Prepare triangle indices.
        surface_indices.extend(P::Sticker::SURFACE_INDICES.iter().map(|&i| base + i));
        // Prepare line indices.
        outline_indices.extend(P::Sticker::OUTLINE_INDICES.iter().map(|&i| base + i));
        base += P::Sticker::VERTEX_COUNT;
    }
    // Write triangle indices.
    c.tri_indices
        .get(surface_indices.len())
        .write(&surface_indices);
    // Write line indices.
    c.line_indices
        .get(outline_indices.len())
        .write(&outline_indices);
}

fn _draw_puzzle<P: PuzzleTrait>(
    target: &mut glium::Frame,
    puzzle: &PuzzleController<P>,
) -> Result<(), glium::DrawError> {
    let (target_w, target_h) = target.get_dimensions();
    target.clear_color_srgb_and_depth(colors::get_bg(), 1.0);

    let cache = &mut *CACHE.borrow_mut();

    // Prepare model matrices, which must be done here on the CPU so that we can do proper Z ordering.
    let stationary_model_matrix = Matrix4::from_angle_x(Deg(VIEW_ANGLE));
    let moving_model_matrix;
    let moving_pieces: HashSet<P::Piece>;
    if let Some((twist, progress)) = puzzle.current_twist() {
        moving_model_matrix = stationary_model_matrix * twist.matrix(progress);
        moving_pieces = twist.pieces().collect();
    } else {
        moving_model_matrix = stationary_model_matrix;
        moving_pieces = HashSet::new();
    };

    // Each sticker has a Vec<StickerVertex> with all of its vertices and a
    // single f32 containing the average Z value.
    let mut verts_by_sticker: Vec<(Vec<StickerVertex>, f32)> = vec![];
    // Generate vertices.
    for piece in P::Piece::iter() {
        let moving = moving_pieces.contains(&piece);
        for sticker in piece.stickers() {
            let color = colors::get_color(puzzle.displayed().get_sticker(sticker).idx());
            let sticker_verts: Vec<StickerVertex> = sticker
                .verts(STICKER_SIZE)
                .iter()
                .map(|&vert_pos| {
                    let matrix = if moving {
                        moving_model_matrix
                    } else {
                        stationary_model_matrix
                    };
                    let pos: [f32; 4] = (matrix * Vector4::from(vert_pos)).into();
                    StickerVertex { pos, color }
                })
                .collect();
            let average_z =
                sticker_verts.iter().map(|v| v.pos[2]).sum::<f32>() / sticker_verts.len() as f32;
            verts_by_sticker.push((sticker_verts, average_z));
        }
    }
    // // Sort by average Z position for proper transparency.
    verts_by_sticker.sort_by(|(_, z1), (_, z2)| z1.partial_cmp(z2).unwrap());
    let verts: Vec<StickerVertex> = verts_by_sticker
        .into_iter()
        .flat_map(|(verts, _)| verts)
        .collect();

    // Write sticker vertices.
    let stickers_vbo = cache.stickers_vbo.get(verts.len());
    stickers_vbo.write(&verts);

    // To avoid dealing with 5x5 matrices, we'll do translation and rotation in
    // GLSL in separate steps.

    // Create the view translation vector, which just distances the puzzle from
    // the camera along both Z and W. TODO: W component should be -10.0?. Need
    // to figure out 4D perspective projection.
    let view_vector: [f32; 4] = [0.0, 0.0, -10.0, 1.0];
    // Create the perspective matrix.
    let perspective_matrix: [[f32; 4]; 4] = {
        let min_dimen = std::cmp::min(target_w, target_h) as f32;
        let r = target_w as f32 / min_dimen;
        let t = target_h as f32 / min_dimen;
        let perspective_matrix = Matrix4::from(Perspective {
            left: -r,
            right: r,
            bottom: -t,
            top: t,
            near: NEAR_PLANE,
            far: FAR_PLANE,
        });
        perspective_matrix.into()
    };

    let draw_params = DrawParameters {
        blend: glium::Blend::alpha_blending(),
        smooth: Some(glium::Smooth::Nicest),
        depth: glium::Depth {
            test: glium::DepthTest::IfLessOrEqual,
            write: true,
            ..glium::Depth::default()
        },
        line_width: Some(LINE_WIDTH),
        ..DrawParameters::default()
    };

    let override_color: [f32; 4] = OUTLINE_COLOR.unwrap_or_default();

    // Draw triangles.
    target.draw(
        &*stickers_vbo,
        &*cache.tri_indices.unwrap(),
        &shaders::BASIC,
        &uniform! {
            use_override_color: false,
            override_color: override_color,
            view_translation: view_vector,
            perspective_matrix: perspective_matrix,
        },
        &draw_params,
    )?;

    // Draw smooth outline.
    target.draw(
        &*stickers_vbo,
        &*cache.line_indices.unwrap(),
        &shaders::BASIC,
        &uniform! {
            use_override_color: OUTLINE_COLOR.is_some(),
            override_color: override_color,
            view_vector: view_vector,
            perspective_matrix: perspective_matrix,
        },
        &draw_params,
    )?;

    Ok(())
}

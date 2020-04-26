//! Rendering logic.

use cgmath::{Deg, Matrix4, Perspective, SquareMatrix, Vector4};
use glium::{DrawParameters, Surface};
use std::collections::HashSet;

mod cache;
mod colors;
mod shaders;
mod verts;

use super::animator::Animator;
use super::puzzle::traits::*;
use super::DISPLAY;
use cache::CACHE;
use verts::*;

const STICKER_SIZE: f32 = 0.9;
const NEAR_PLANE: f32 = 3.0;
const FAR_PLANE: f32 = 20.0;

const DRAW_OUTLINE: bool = true;
const LINE_WIDTH: f32 = 1.5;

pub fn setup_puzzle<P: PuzzleTrait>() {
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

pub fn draw_puzzle<P: PuzzleTrait>(
    target: &mut glium::Frame,
    animator: &mut Animator<P>,
) -> Result<(), glium::DrawError> {
    let (target_w, target_h) = target.get_dimensions();
    target.clear_color_srgb_and_depth(colors::get_bg(), 1.0);

    let cache = &mut *CACHE.borrow_mut();

    animator.next_frame();

    // Animate current move.
    let moving_pieces: HashSet<P::Piece>;
    let moving_matrix;
    if let Some((twist, progress)) = animator.current_twist() {
        moving_pieces = twist.pieces().collect();
        moving_matrix = twist.matrix(progress);
    } else {
        moving_pieces = HashSet::new();
        moving_matrix = Matrix4::identity();
    };

    let mut verts = vec![];
    // Generate vertices.
    for piece in P::Piece::iter() {
        let moving = moving_pieces.contains(&piece);
        for sticker in piece.stickers() {
            let color = colors::get_color(animator.displayed().get_sticker(sticker).idx());
            verts.extend(sticker.verts(STICKER_SIZE).iter().map(|&vert_pos| {
                let pos = if moving {
                    (moving_matrix * Vector4::from(vert_pos)).into()
                } else {
                    vert_pos
                };
                StickerVertex { pos, color }
            }));
        }
    }

    // Write sticker vertices.
    let stickers_vbo = cache.stickers_vbo.get(verts.len());
    stickers_vbo.write(&verts);

    // To avoid dealing with 5x5 matrices, we'll do translation and rotation in
    // GLSL in separate steps.

    // Create the model matrix.
    let model_matrix: [[f32; 4]; 4] = Matrix4::from_angle_x(Deg(35.0)).into();

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

    let mut draw_params = DrawParameters {
        blend: glium::Blend::alpha_blending(),
        smooth: Some(if DRAW_OUTLINE {
            glium::Smooth::Fastest
        } else {
            glium::Smooth::Nicest
        }),
        depth: glium::Depth {
            test: glium::DepthTest::IfLessOrEqual,
            write: true,
            ..glium::Depth::default()
        },
        ..DrawParameters::default()
    };

    // Draw triangles.
    target.draw(
        &*stickers_vbo,
        &*cache.tri_indices.unwrap(),
        &shaders::BASIC,
        &uniform! {
            lines: false,
            model_matrix: model_matrix,
            view_translation: view_vector,
            perspective_matrix: perspective_matrix,
        },
        &draw_params,
    )?;

    if DRAW_OUTLINE {
        draw_params.smooth = Some(glium::Smooth::Nicest);
        // Draw lines.
        target.draw(
            &*stickers_vbo,
            &*cache.line_indices.unwrap(),
            &shaders::BASIC,
            &uniform! {
                lines: true,
                model_matrix: model_matrix,
                view_vector: view_vector,
                perspective_matrix: perspective_matrix,
            },
            &draw_params,
        )?;
    }

    Ok(())
}

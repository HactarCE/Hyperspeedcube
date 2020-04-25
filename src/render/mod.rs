//! Rendering logic.

use cgmath::{Deg, Matrix4, Perspective};
use glium::{index::PrimitiveType, DrawParameters, IndexBuffer, Surface, VertexBuffer};
use send_wrapper::SendWrapper;
use std::cell::RefCell;

mod colors;
mod shaders;

use super::puzzle::traits::*;
use super::DISPLAY;

const STICKER_SIZE: f32 = 0.9;
const NEAR_PLANE: f32 = 3.0;
const FAR_PLANE: f32 = 20.0;

lazy_static! {
    static ref STICKERS_VBO: SendWrapper<RefCell<VertexBuffer<StickerVertex>>> =
        SendWrapper::new(RefCell::new(
            VertexBuffer::empty_dynamic(&**DISPLAY, 3 * 3 * 6 * 4)
                .expect("Failed to create vertex buffer")
        ));
    static ref STICKERS_INDICES: SendWrapper<RefCell<IndexBuffer<u8>>> =
        SendWrapper::new(RefCell::new({
            let indices: Vec<_> = (0u8..(6 * 9))
                .flat_map(|base| {
                    [0, 1, 2, 3, 2, 1, 1, 2, 3, 2, 1, 0]
                        .iter()
                        .map(move |i| base * 4 + i)
                })
                .collect();
            IndexBuffer::new(&**DISPLAY, PrimitiveType::TrianglesList, &indices)
                .expect("Failed to create index buffer")
        }));
}

#[derive(Debug, Default, Copy, Clone)]
struct StickerVertex {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}
implement_vertex!(StickerVertex, pos, color);

pub fn draw_puzzle<P: PuzzleTrait>(
    target: &mut glium::Frame,
    puzzle: &P,
) -> Result<(), glium::DrawError> {
    let (target_w, target_h) = target.get_dimensions();
    target.clear_color_srgb_and_depth(colors::get_bg(), 1.0);

    let mut verts = Vec::with_capacity(3 * 3 * 6 * 4);
    for sticker in P::Sticker::iter() {
        // Generate vertices.
        let color = colors::get_color(puzzle.get_sticker(sticker).idx());
        verts.extend(
            sticker
                .verts(STICKER_SIZE)
                .iter()
                .map(|&pos| StickerVertex { pos, color }),
        );
    }
    let vbo = STICKERS_VBO.borrow_mut();
    vbo.write(&verts);

    // To avoid dealing with 5x5 matrices, we'll do translation and rotation in
    // GLSL in separate steps.

    // Create the model matrix.
    let model: [[f32; 4]; 4] = Matrix4::from_angle_x(Deg(35.0)).into();
    // Create the view translation vector, which just distances the puzzle from
    // the camera along both Z and W. TODO: W component should be -10.0?. Need
    // to figure out 4D perspective projection.
    let view: [f32; 4] = [0.0, 0.0, -10.0, 1.0];
    // Create the perspective matrix.
    let perspective: [[f32; 4]; 4] = {
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

    target.draw(
        &*vbo,
        &*STICKERS_INDICES.borrow(),
        &shaders::TRIS,
        &uniform! {
            model_matrix: model,
            view_translation: view,
            perspective_matrix: perspective
        },
        &DrawParameters {
            blend: glium::Blend::alpha_blending(),
            depth: glium::Depth {
                test: glium::DepthTest::IfLess,
                write: true,
                ..glium::Depth::default()
            },
            ..DrawParameters::default()
        },
    )
}

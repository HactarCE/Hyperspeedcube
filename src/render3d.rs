use cgmath::{Deg, Matrix4, Perspective};
use glium::{index::PrimitiveType, DrawParameters, IndexBuffer, Surface, VertexBuffer};
use send_wrapper::SendWrapper;
use std::cell::RefCell;

use super::puzzle3d::*;
use super::shaders;
use super::DISPLAY;

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
    pub pos: [f32; 3],
    pub color: [f32; 4],
}
implement_vertex!(StickerVertex, pos, color);

pub fn render(target: &mut glium::Frame, puzzle: &Puzzle) -> Result<(), glium::DrawError> {
    let (target_w, target_h) = target.get_dimensions();
    target.clear_color_srgb_and_depth((0.2, 0.2, 0.2, 1.0), 1.0);

    let mut verts = Vec::with_capacity(3 * 3 * 6 * 4);
    for sticker in Sticker::iter() {
        // Generate vertices.
        let color = puzzle.get_sticker(sticker).color();
        verts.extend(
            sticker
                .verts()
                .iter()
                .map(|&pos| StickerVertex { pos, color }),
        );
    }
    let vbo = STICKERS_VBO.borrow_mut();
    vbo.write(&verts);

    let min_dimen = std::cmp::min(target_w, target_h) as f32;
    let r = target_w as f32 / min_dimen;
    let t = target_h as f32 / min_dimen;
    let f = 17.0;
    let n = 3.0;
    // let perspective_matrix: [[f32; 4]; 4] = [
    //     [1.0 / r, 0.0, 0.05, 0.0],
    //     [0.0, 1.0 / t, 0.05, 0.0],
    //     [0.0, 0.0, -2 / , -10.0],
    //     [0.0, 0.0, -1.0, 0.0],
    // ];
    let model_matrix =
        Matrix4::from_translation([0.0, 0.0, -10.0].into()) * Matrix4::from_angle_x(Deg(35.0));
    let perspective_matrix = Matrix4::from(Perspective {
        left: -r,
        right: r,
        bottom: -t,
        top: t,
        near: n,
        far: f,
    });

    let matrix: [[f32; 4]; 4] = (perspective_matrix * model_matrix).into();

    target.draw(
        &*vbo,
        &*STICKERS_INDICES.borrow(),
        // &STICKERS_INDICES
        //     .borrow()
        //     .slice((6 * t)..(6 * t + 5))
        //     .unwrap(),
        &shaders::TRIS,
        &uniform! {
            matrix: matrix,
        },
        &DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLess,
                write: true,
                ..glium::Depth::default()
            },
            ..DrawParameters::default()
        },
    )
}

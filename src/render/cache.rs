use glium::{index::PrimitiveType, IndexBuffer, VertexBuffer};
use send_wrapper::SendWrapper;
use std::cell::RefCell;

use super::super::DISPLAY;
use super::verts::*;

lazy_static! {
    pub static ref CACHE: SendWrapper<RefCell<RenderCache>> =
        SendWrapper::new(RefCell::new(RenderCache {
            stickers_vbo: Cached::new(|len| VertexBuffer::empty_dynamic(&**DISPLAY, len)
                .expect("Failed to create vertex buffer")),
            tri_indices: Cached::new(|len| IndexBuffer::empty_dynamic(
                &**DISPLAY,
                PrimitiveType::TrianglesList,
                len
            )
            .expect("Failed to create index buffer")),
            line_indices: Cached::new(|len| IndexBuffer::empty_dynamic(
                &**DISPLAY,
                PrimitiveType::LinesList,
                len
            )
            .expect("Failed to create index buffer")),
        }));
}

pub struct RenderCache {
    pub stickers_vbo: Cached<usize, VertexBuffer<StickerVertex>>,
    pub tri_indices: Cached<usize, IndexBuffer<u16>>,
    pub line_indices: Cached<usize, IndexBuffer<u16>>,
}

pub struct Cached<P: Copy + PartialEq, T> {
    generator: fn(P) -> T,
    current_param: Option<P>,
    inner: Option<T>,
}
impl<P: Copy + PartialEq, T> Cached<P, T> {
    pub fn new(generator: fn(P) -> T) -> Self {
        Self {
            generator,
            current_param: None,
            inner: None,
        }
    }
    pub fn get(&mut self, param: P) -> &mut T {
        if Some(param) != self.current_param {
            self.current_param = Some(param);
            self.inner = Some((self.generator)(param));
        }
        self.unwrap()
    }
    pub fn unwrap(&mut self) -> &mut T {
        self.inner.as_mut().unwrap()
    }
}

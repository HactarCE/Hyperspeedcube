use glium::index::{Index, IndexBuffer, IndexBufferSlice, PrimitiveType};
use glium::vertex::{Vertex, VertexBuffer, VertexBufferSlice};
use glium_glyph::glyph_brush::rusttype::Font;
use glium_glyph::GlyphBrush;
use send_wrapper::SendWrapper;
use std::cell::{RefCell, RefMut};

use super::verts::*;
use crate::puzzle::PuzzleType;
use crate::DISPLAY;
lazy_static! {
    pub static ref FONT: Font<'static> =
        Font::from_bytes(include_bytes!("../../resources/font/NotoSans-Regular.ttf"))
            .expect("failed to load font");
    static ref CACHE: SendWrapper<RefCell<RenderCache>> =
        SendWrapper::new(RefCell::new(RenderCache {
            last_puzzle_type: None,
            stickers_vbo: CachedVbo::new(),
            label_backdrops_vbo: CachedVbo::new(),
            tri_indices: CachedIbo::new(PrimitiveType::TrianglesList),
            line_indices: CachedIbo::new(PrimitiveType::LinesList),
            glyph_brush: GlyphBrush::new(&**DISPLAY, vec![FONT.clone()])
        }));
}

pub fn borrow_cache<'a>() -> RefMut<'a, RenderCache> {
    CACHE.borrow_mut()
}

pub struct RenderCache {
    pub last_puzzle_type: Option<PuzzleType>,
    pub stickers_vbo: CachedVbo<RgbaVertex>,
    pub label_backdrops_vbo: CachedVbo<RgbaVertex>,
    pub tri_indices: CachedIbo<u16>,
    pub line_indices: CachedIbo<u16>,
    pub glyph_brush: GlyphBrush<'static, 'static>,
}

#[derive(Debug)]
pub struct CachedVbo<T: Vertex>(Option<VertexBuffer<T>>);
impl<T: Vertex> CachedVbo<T> {
    fn new() -> Self {
        Self(None)
    }
    pub fn slice<'a>(&'a mut self, len: usize) -> VertexBufferSlice<'a, T> {
        if self.0.is_none() || self.0.as_ref().unwrap().len() < len {
            self.0 = Some(
                VertexBuffer::empty_dynamic(&**DISPLAY, len)
                    .expect("failed to create vertex buffer"),
            );
        }
        self.0
            .as_mut()
            .unwrap()
            .slice(..len)
            .expect("failed to slice vertex buffer")
    }
}

#[derive(Debug)]
pub struct CachedIbo<T: Index> {
    prim: PrimitiveType,
    ibo: Option<IndexBuffer<T>>,
}
impl<T: Index> CachedIbo<T> {
    fn new(prim: PrimitiveType) -> Self {
        Self { prim, ibo: None }
    }
    pub fn slice<'a>(&'a mut self, len: usize) -> IndexBufferSlice<'a, T> {
        if self.ibo.is_none() || self.ibo.as_ref().unwrap().len() < len {
            self.ibo = Some(
                IndexBuffer::empty_dynamic(&**DISPLAY, self.prim, len)
                    .expect("failed to create index buffer"),
            );
        }
        self.ibo
            .as_mut()
            .unwrap()
            .slice(..len)
            .expect("failed to slice index buffer")
    }
}

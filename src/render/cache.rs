use glium::framebuffer::SimpleFrameBuffer;
use glium::index::{Index, IndexBuffer, IndexBufferSlice, PrimitiveType};
use glium::texture::{
    DepthTexture2dMultisample, MipmapsOption, SrgbFormat, SrgbTexture2d, SrgbTexture2dMultisample,
};
use glium::vertex::{Vertex, VertexBuffer, VertexBufferSlice};
use glium_glyph::glyph_brush::rusttype::Font;
use glium_glyph::GlyphBrush;
use send_wrapper::SendWrapper;
use std::rc::Rc;

use super::verts::*;
use crate::DISPLAY;

lazy_static! {
    pub static ref FONT: Font<'static> =
        Font::from_bytes(include_bytes!("../../resources/font/NotoSans-Regular.ttf"))
            .expect("failed to load font");
    pub static ref DUMMY_TEXTURE: SendWrapper<Rc<SrgbTexture2d>> = SendWrapper::new(Rc::new(
        SrgbTexture2d::empty(&**DISPLAY, 1, 1).expect("failed to create texture")
    ));
}

pub struct PuzzleRenderCache {
    pub(super) target: CachedMsaaRenderBuffer,
    pub(super) out_tex: CachedSrgbTexture2d,
    pub(super) stickers_vbo: CachedVbo<WireframeVertex>,
    pub(super) label_backdrops_vbo: CachedVbo<RgbaVertex>,
    pub(super) glyph_brush: GlyphBrush<'static, 'static>,
}
impl Default for PuzzleRenderCache {
    fn default() -> Self {
        Self {
            target: CachedMsaaRenderBuffer::new(),
            out_tex: CachedSrgbTexture2d::new(),
            stickers_vbo: CachedVbo::new(),
            label_backdrops_vbo: CachedVbo::new(),
            glyph_brush: GlyphBrush::new(&**DISPLAY, vec![FONT.clone()]),
        }
    }
}

#[derive(Debug)]
pub(super) struct CachedVbo<T: Vertex>(Option<VertexBuffer<T>>);
impl<T: Vertex> CachedVbo<T> {
    fn new() -> Self {
        Self(None)
    }
    pub(super) fn slice(&mut self, len: usize) -> VertexBufferSlice<'_, T> {
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
pub(super) struct CachedIbo<T: Index> {
    prim: PrimitiveType,
    ibo: Option<IndexBuffer<T>>,
}
impl<T: Index> CachedIbo<T> {
    fn new(prim: PrimitiveType) -> Self {
        Self { prim, ibo: None }
    }
    pub(super) fn slice(&mut self, len: usize) -> IndexBufferSlice<'_, T> {
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

#[derive(Debug)]
pub(super) struct CachedSrgbTexture2d(Option<Rc<SrgbTexture2d>>);
impl CachedSrgbTexture2d {
    fn new() -> Self {
        Self(None)
    }

    pub(super) fn get<'a>(
        &'a mut self,
        width: u32,
        height: u32,
    ) -> (SimpleFrameBuffer<'a>, Rc<SrgbTexture2d>) {
        // Invalidate the texture if the size has changed.
        if let Some(tex) = &self.0 {
            if tex.width() != width || tex.height() != height {
                self.0 = None;
            }
        }

        let color = self.0.get_or_insert_with(|| {
            Rc::new(
                SrgbTexture2d::empty_with_format(
                    &**DISPLAY,
                    SrgbFormat::U8U8U8,
                    MipmapsOption::NoMipmap,
                    width,
                    height,
                )
                .expect("failed to create color texture"),
            )
        });

        // Don't worry! glium caches the FBO so we aren't *really* recreating
        // the FBO every frame.
        let fbo =
            SimpleFrameBuffer::new(&**DISPLAY, &**color).expect("failed to create frame buffer");

        (fbo, Rc::clone(color))
    }
}

#[derive(Debug)]
pub(super) struct CachedMsaaRenderBuffer {
    color_texture: Option<Rc<SrgbTexture2dMultisample>>,
    depth_texture: Option<DepthTexture2dMultisample>,
}
impl CachedMsaaRenderBuffer {
    fn new() -> Self {
        Self {
            color_texture: None,
            depth_texture: None,
        }
    }

    pub(super) fn get<'a>(
        &'a mut self,
        width: u32,
        height: u32,
        samples: u32,
    ) -> (SimpleFrameBuffer<'a>, Rc<SrgbTexture2dMultisample>) {
        // Invalidate the textures if the size or sample count has changed.
        if let Some(tex) = &self.color_texture {
            if tex.width() != width || tex.height() != height || tex.samples() != samples {
                self.color_texture = None;
                self.depth_texture = None;
            }
        }

        let color = self.color_texture.get_or_insert_with(|| {
            Rc::new(
                SrgbTexture2dMultisample::empty_with_format(
                    &**DISPLAY,
                    SrgbFormat::U8U8U8,
                    MipmapsOption::NoMipmap,
                    width,
                    height,
                    samples,
                )
                .expect("failed to create color texture"),
            )
        });
        let depth = self.depth_texture.get_or_insert_with(|| {
            DepthTexture2dMultisample::empty_with_format(
                &**DISPLAY,
                glium::texture::DepthFormat::F32,
                MipmapsOption::NoMipmap,
                width,
                height,
                samples,
            )
            .expect("failed to create depth texture")
        });

        // Don't worry! glium caches the FBO so we aren't *really* recreating
        // the FBO every frame.
        let fbo = SimpleFrameBuffer::with_depth_buffer(&**DISPLAY, &**color, &*depth)
            .expect("failed to create frame buffer");

        (fbo, Rc::clone(color))
    }
}

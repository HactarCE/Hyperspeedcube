use glium::framebuffer::{DepthRenderBuffer, RenderBuffer, SimpleFrameBuffer};
use glium::texture::{
    DepthFormat, MipmapsOption, SrgbFormat, SrgbTexture2d, UncompressedFloatFormat,
};
use glium::vertex::{Vertex, VertexBuffer, VertexBufferSlice};
use send_wrapper::SendWrapper;
use std::rc::Rc;

use super::verts::*;
use crate::DISPLAY;

lazy_static! {
    pub static ref DUMMY_TEXTURE: SendWrapper<Rc<SrgbTexture2d>> = SendWrapper::new(Rc::new(
        SrgbTexture2d::empty(&**DISPLAY, 1, 1).expect("failed to create texture")
    ));
}

pub struct PuzzleRenderCache {
    pub(super) target: CachedMsaaRenderBuffer,
    pub(super) out_tex: CachedSrgbTexture2d,
    pub(super) stickers_vbo: CachedVbo<WireframeVertex>,
}
impl Default for PuzzleRenderCache {
    fn default() -> Self {
        Self {
            target: CachedMsaaRenderBuffer::new(),
            out_tex: CachedSrgbTexture2d::new(),
            stickers_vbo: CachedVbo::new(),
        }
    }
}

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

pub(super) struct CachedSrgbTexture2d(Option<Rc<SrgbTexture2d>>);
impl CachedSrgbTexture2d {
    fn new() -> Self {
        Self(None)
    }

    pub(super) fn get(
        &mut self,
        width: u32,
        height: u32,
    ) -> (SimpleFrameBuffer<'_>, Rc<SrgbTexture2d>) {
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

pub(super) struct CachedMsaaRenderBuffer {
    color: Option<RenderBuffer>,
    depth: Option<DepthRenderBuffer>,
}
impl CachedMsaaRenderBuffer {
    fn new() -> Self {
        Self {
            color: None,
            depth: None,
        }
    }

    pub(super) fn get(&mut self, width: u32, height: u32, samples: u32) -> SimpleFrameBuffer<'_> {
        // Invalidate the textures if the size or sample count has changed.
        if let Some(buf) = &self.color {
            if buf.get_dimensions() != (width, height) || buf.get_samples() != Some(samples) {
                self.color = None;
                self.depth = None;
            }
        }

        let color = self.color.get_or_insert_with(|| {
            RenderBuffer::new_multisample(
                &**DISPLAY,
                UncompressedFloatFormat::U8U8U8,
                width,
                height,
                samples,
            )
            .expect("failed to create color render buffer")
        });
        let depth = self.depth.get_or_insert_with(|| {
            DepthRenderBuffer::new_multisample(&**DISPLAY, DepthFormat::F32, width, height, samples)
                .expect("failed to create depth render buffer")
        });

        // Don't worry! glium caches the FBO so we aren't *really* recreating
        // the FBO every frame.
        SimpleFrameBuffer::with_depth_buffer(&**DISPLAY, &*color, &*depth)
            .expect("failed to create frame buffer")
    }
}

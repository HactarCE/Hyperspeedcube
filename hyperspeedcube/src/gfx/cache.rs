use std::sync::Arc;

use super::GraphicsState;

pub(crate) type CachedTexture1d = CachedTexture<u32>;
impl CachedTexture1d {
    pub fn new(
        gfx: Arc<GraphicsState>,
        label: String,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new_generic(CachedTextureInner {
            gfx,

            label,
            dimension: wgpu::TextureDimension::D1,
            format,
            usage,

            size: 1,
            size_to_extent_3d: |x| wgpu::Extent3d {
                width: x,
                height: 1,
                depth_or_array_layers: 1,
            },
        })
    }

    pub fn write<T: bytemuck::Pod>(&mut self, data: &[T]) {
        self.set_size(data.len() as u32);
        self.inner.gfx.queue.write_texture(
            self.texture.as_image_copy(),
            bytemuck::cast_slice(data),
            wgpu::ImageDataLayout::default(),
            self.inner.extent_3d(),
        );
    }
}

pub(crate) type CachedTexture2d = CachedTexture<[u32; 2]>;
impl CachedTexture2d {
    pub fn new(
        gfx: Arc<GraphicsState>,
        label: String,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new_generic(CachedTextureInner {
            gfx,

            label,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,

            size: [1, 1],
            size_to_extent_3d: |[x, y]| wgpu::Extent3d {
                width: x,
                height: y,
                depth_or_array_layers: 1,
            },
        })
    }
}

pub(crate) struct CachedTexture<S> {
    inner: CachedTextureInner<S>,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}
impl<S: PartialEq + Copy> CachedTexture<S> {
    fn new_generic(inner: CachedTextureInner<S>) -> Self {
        let texture = inner.gfx.create_texture(wgpu::TextureDescriptor {
            label: Some(&inner.label),
            size: inner.extent_3d(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: inner.dimension,
            format: inner.format,
            usage: inner.usage,
            view_formats: &[
                inner.format.add_srgb_suffix(),
                inner.format.remove_srgb_suffix(),
            ],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        CachedTexture {
            inner,
            texture,
            view,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.inner.format
    }

    pub fn clone(&self, label: String) -> Self {
        Self::new_generic(CachedTextureInner {
            label,
            ..self.inner.clone()
        })
    }

    pub fn set_size(&mut self, size: S) {
        // Invalidate the buffer if it is the wrong size.
        if size != self.inner.size {
            self.inner.size = size;
            *self = Self::new_generic(self.inner.clone());
        }
    }
}

#[derive(Debug, Clone)]
struct CachedTextureInner<S> {
    gfx: Arc<GraphicsState>,

    label: String,
    dimension: wgpu::TextureDimension,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,

    size: S,
    size_to_extent_3d: fn(S) -> wgpu::Extent3d,
}
impl<S: Copy> CachedTextureInner<S> {
    fn extent_3d(&self) -> wgpu::Extent3d {
        (self.size_to_extent_3d)(self.size)
    }
}

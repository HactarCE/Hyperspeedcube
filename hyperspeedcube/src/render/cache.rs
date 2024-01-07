use std::sync::Arc;

use super::GraphicsState;

pub(super) struct CachedDynamicBuffer {
    label: Option<&'static str>,
    usage: wgpu::BufferUsages,
    element_size: usize,
    len: Option<usize>,
    buffer: Option<wgpu::Buffer>,
}
impl CachedDynamicBuffer {
    pub fn new<T>(label: Option<&'static str>, usage: wgpu::BufferUsages) -> Self {
        Self {
            label,
            usage,
            element_size: std::mem::size_of::<T>(),
            len: None,
            buffer: None,
        }
    }

    pub fn at_min_len(&mut self, gfx: &GraphicsState, min_len: usize) -> &mut wgpu::Buffer {
        // Invalidate the buffer if it is too small.
        if let Some(len) = self.len {
            if len < min_len {
                self.buffer = None;
            }
        }

        self.buffer.get_or_insert_with(|| {
            self.len = Some(min_len);
            gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size: (min_len * self.element_size) as u64,
                usage: self.usage,
                mapped_at_creation: false,
            })
        })
    }

    pub fn slice(
        &mut self,
        gfx: &GraphicsState,
        len: usize,
    ) -> (&wgpu::Buffer, wgpu::BufferSlice<'_>) {
        let element_size = self.element_size;
        let b = self.at_min_len(gfx, len);
        (b, b.slice(0..(len * element_size) as u64))
    }

    pub fn write_all<T: Default + bytemuck::NoUninit>(
        &mut self,
        gfx: &GraphicsState,
        data: &mut Vec<T>,
    ) -> wgpu::BufferSlice<'_> {
        let original_len = data.len();
        super::pad_buffer_to_wgpu_copy_buffer_alignment(data);
        let (buf, buf_slice) = self.slice(gfx, data.len());
        gfx.queue.write_buffer(buf, 0, bytemuck::cast_slice(data));
        data.truncate(original_len); // undo padding
        buf_slice
    }
}

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
    pub view: wgpu::TextureView,
    pub linear_view: wgpu::TextureView,
    pub srgb_view: wgpu::TextureView,

    pub texture: wgpu::Texture,

    inner: CachedTextureInner<S>,
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

        let view_descriptor = |format| wgpu::TextureViewDescriptor {
            format,
            ..Default::default()
        };

        CachedTexture {
            view: texture.create_view(&view_descriptor(None)),
            linear_view: texture
                .create_view(&view_descriptor(Some(inner.format.remove_srgb_suffix()))),
            srgb_view: texture.create_view(&view_descriptor(Some(inner.format.add_srgb_suffix()))),

            texture,

            inner,
        }
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

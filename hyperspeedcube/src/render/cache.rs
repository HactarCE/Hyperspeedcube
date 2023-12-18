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

pub(super) struct CachedTexture {
    label: String,
    dimension: wgpu::TextureDimension,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,

    size: Option<wgpu::Extent3d>,
    texture: Option<(wgpu::Texture, wgpu::TextureView)>,
}
impl CachedTexture {
    pub fn new(
        label: String,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        CachedTexture {
            label,
            dimension,
            format,
            usage,

            size: None,
            texture: None,
        }
    }
    pub fn new_2d(label: String, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self::new(label, wgpu::TextureDimension::D2, format, usage)
    }
    pub fn new_1d(label: String, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Self {
        Self::new(label, wgpu::TextureDimension::D1, format, usage)
    }

    pub fn clone(&self, label: String) -> Self {
        Self {
            label,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,

            size: None,
            texture: None,
        }
    }

    pub fn at_size(
        &mut self,
        gfx: &GraphicsState,
        size: wgpu::Extent3d,
    ) -> &(wgpu::Texture, wgpu::TextureView) {
        // Invalidate the buffer if it is the wrong size.
        if self.size != Some(size) {
            self.texture = None;
        }

        self.texture.get_or_insert_with(|| {
            self.size = Some(size);
            gfx.create_texture(wgpu::TextureDescriptor {
                label: Some(&self.label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: self.dimension,
                format: self.format,
                usage: self.usage,
                view_formats: &[],
            })
        })
    }

    pub fn get(&self) -> Option<&(wgpu::Texture, wgpu::TextureView)> {
        self.texture.as_ref()
    }
}

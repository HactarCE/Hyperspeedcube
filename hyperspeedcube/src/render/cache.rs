use once_cell::unsync::OnceCell;
use std::marker::PhantomData;

use super::GraphicsState;

pub(crate) struct CachedDynamicBuffer {
    label: Option<&'static str>,
    usage: wgpu::BufferUsages,
    element_size: usize,
    len: Option<usize>,
    buffer: Option<wgpu::Buffer>,
}
impl CachedDynamicBuffer {
    pub(super) fn new<T>(label: Option<&'static str>, usage: wgpu::BufferUsages) -> Self {
        Self {
            label,
            usage,
            element_size: std::mem::size_of::<T>(),
            len: None,
            buffer: None,
        }
    }

    pub(super) fn at_min_len(&mut self, gfx: &GraphicsState, min_len: usize) -> &mut wgpu::Buffer {
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

    pub(super) fn slice(
        &mut self,
        gfx: &GraphicsState,
        len: usize,
    ) -> (&wgpu::Buffer, wgpu::BufferSlice) {
        let element_size = self.element_size;
        let b = self.at_min_len(gfx, len);
        (b, b.slice(0..(len * element_size) as u64))
    }

    pub(super) fn write_all<T: Default + bytemuck::NoUninit>(
        &mut self,
        gfx: &GraphicsState,
        data: &mut Vec<T>,
    ) -> wgpu::BufferSlice {
        let original_len = data.len();
        pad_buffer_if_necessary(data);
        let (buf, buf_slice) = self.slice(gfx, data.len());
        gfx.queue.write_buffer(buf, 0, bytemuck::cast_slice(data));
        data.truncate(original_len); // undo padding
        buf_slice
    }
}

pub(crate) struct CachedUniformBuffer<T> {
    label: Option<&'static str>,
    binding: u32,
    buffer: OnceCell<(wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup)>,
    _phantom: PhantomData<T>,
}
impl<T> CachedUniformBuffer<T> {
    pub(super) fn new(label: Option<&'static str>, binding: u32) -> Self {
        Self {
            label,
            binding,
            buffer: OnceCell::new(),
            _phantom: PhantomData,
        }
    }
    pub(super) fn get(
        &self,
        gfx: &GraphicsState,
    ) -> &(wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        self.buffer.get_or_init(|| {
            gfx.create_uniform::<T>(
                self.label,
                self.binding,
                wgpu::ShaderStages::VERTEX_FRAGMENT,
            )
        })
    }

    pub(super) fn buffer(&self, gfx: &GraphicsState) -> &wgpu::Buffer {
        &self.get(gfx).0
    }
    pub(super) fn bind_group_layout(&self, gfx: &GraphicsState) -> &wgpu::BindGroupLayout {
        &self.get(gfx).1
    }
    pub(super) fn bind_group(&self, gfx: &GraphicsState) -> &wgpu::BindGroup {
        &self.get(gfx).2
    }

    pub(super) fn write(&self, gfx: &GraphicsState, data: &T)
    where
        T: bytemuck::NoUninit,
    {
        gfx.queue
            .write_buffer(self.buffer(gfx), 0, bytemuck::bytes_of(data));
    }
}

/// Pads a buffer to `wgpu::COPY_BUFFER_ALIGNMENT`.
fn pad_buffer_if_necessary<T: Default + bytemuck::NoUninit>(buf: &mut Vec<T>) {
    loop {
        let bytes_len = bytemuck::cast_slice::<T, u8>(buf).len();
        if bytes_len > 0 && bytes_len as u64 % wgpu::COPY_BUFFER_ALIGNMENT == 0 {
            break;
        }
        buf.push(T::default());
    }
}

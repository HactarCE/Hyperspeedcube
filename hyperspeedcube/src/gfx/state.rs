use std::fmt;
use std::sync::Arc;

use itertools::Itertools;
use wgpu::util::DeviceExt;

use super::pipelines::Pipelines;

/// Graphics state for the whole window.
pub(crate) struct GraphicsState {
    pub(in crate::gfx) device: Arc<wgpu::Device>,
    pub(in crate::gfx) queue: Arc<wgpu::Queue>,

    pub(in crate::gfx) pipelines: Pipelines,

    pub(in crate::gfx) uv_vertex_buffer: wgpu::Buffer,
    pub(in crate::gfx) default_sampler: wgpu::Sampler,
}
impl fmt::Debug for GraphicsState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GraphicsState").finish_non_exhaustive()
    }
}
impl GraphicsState {
    pub(crate) fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        let device = Arc::clone(&render_state.device);
        let queue = Arc::clone(&render_state.queue);
        let target_format = render_state.target_format;

        let pipelines = Pipelines::new(&device, target_format);

        let uv_vertex_buffer = create_buffer_init::<super::structs::UvVertex>(
            &device,
            "uv_vertices",
            &super::structs::UvVertex::SQUARE,
            wgpu::BufferUsages::VERTEX,
        );
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Self {
            device,
            queue,

            pipelines,

            uv_vertex_buffer,
            default_sampler,
        }
    }

    pub(super) fn create_buffer_init<T: Default + bytemuck::NoUninit>(
        &self,
        label: impl fmt::Display,
        contents: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        create_buffer_init(&self.device, label, contents, usage)
    }
    pub(super) fn create_buffer<T>(
        &self,
        label: impl fmt::Display,
        len: usize,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let size = std::mem::size_of::<T>() * len;
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&label.to_string()),
            size: wgpu::util::align_to(size as u64, wgpu::COPY_BUFFER_ALIGNMENT),
            usage,
            mapped_at_creation: false,
        })
    }

    pub(super) fn create_uniform_buffer<T>(&self, label: impl fmt::Display) -> wgpu::Buffer {
        self.create_buffer::<T>(
            label,
            1,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        )
    }

    pub(super) fn create_texture(&self, mut desc: wgpu::TextureDescriptor<'_>) -> wgpu::Texture {
        fn clamp_u32(n: &mut u32, limit: u32) {
            if *n > limit {
                *n = limit;
            }
        }

        // Respect texture limits.
        let limits = self.device.limits();
        match desc.dimension {
            wgpu::TextureDimension::D1 => {
                clamp_u32(&mut desc.size.width, limits.max_texture_dimension_1d);
            }
            wgpu::TextureDimension::D2 => {
                clamp_u32(&mut desc.size.width, limits.max_texture_dimension_2d);
                clamp_u32(&mut desc.size.height, limits.max_texture_dimension_2d);
            }
            wgpu::TextureDimension::D3 => {
                let max = limits.max_texture_dimension_3d;
                clamp_u32(&mut desc.size.width, max);
                clamp_u32(&mut desc.size.height, max);
                clamp_u32(&mut desc.size.depth_or_array_layers, max);
            }
        }

        self.device.create_texture(&desc)
    }
}

fn create_buffer_init<T: Default + bytemuck::NoUninit>(
    device: &wgpu::Device,
    label: impl fmt::Display,
    contents: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    let mut contents = contents.to_vec();
    super::pad_buffer_to_wgpu_copy_buffer_alignment(&mut contents);

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&label.to_string()),
        contents: bytemuck::cast_slice::<T, u8>(contents.as_slice()),
        usage,
    })
}

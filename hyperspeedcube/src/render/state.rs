use std::fmt;

use itertools::Itertools;
use wgpu::util::DeviceExt;

use super::pipelines::Pipelines;

/// Graphics state for the whole window.
pub(crate) struct GraphicsState {
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    pub(crate) surface: wgpu::Surface,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,

    pub(super) pipelines: Pipelines,

    pub(crate) scale_factor: f32,

    /// 1x1 texture used as a temporary value. Its contents are not important.
    #[allow(unused)]
    dummy_texture: wgpu::Texture,
    dummy_texture_view: wgpu::TextureView,
}
impl GraphicsState {
    pub(crate) async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        // Create surface.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface =
            unsafe { instance.create_surface(&window) }.expect("failed to create surface");

        // Request adapter.
        let adapter = request_adapter(&instance, &surface).await;

        // Request device.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Configure surface.
        // TODO: consider different VSync modes
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("unsupported surface");
        surface.configure(&device, &config);

        let pipelines = Pipelines::new(&device);

        let scale_factor = window.scale_factor() as f32;

        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy_texture"),
            size: wgpu::Extent3d::default(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            size,
            surface,
            device,
            queue,
            config,

            pipelines,

            scale_factor,

            dummy_texture,
            dummy_texture_view,
        }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub(crate) fn set_scale_factor(&mut self, new_scale_factor: f32) {
        self.scale_factor = new_scale_factor;
    }

    /// Returns a 1x1 texture used as a temporary value. Its contents are not
    /// important.
    pub(crate) fn dummy_texture_view(&self) -> &wgpu::TextureView {
        &self.dummy_texture_view
    }

    pub(super) fn create_buffer_init<T: Default + bytemuck::NoUninit>(
        &self,
        label: impl fmt::Display,
        contents: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let mut contents = contents.to_vec();
        super::pad_buffer_to_wgpu_copy_buffer_alignment(&mut contents);

        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&label.to_string()),
                contents: bytemuck::cast_slice::<T, u8>(contents.as_slice()),
                usage,
            })
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

    pub(super) fn create_texture(
        &self,
        mut desc: wgpu::TextureDescriptor,
    ) -> (wgpu::Texture, wgpu::TextureView) {
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

        let tex = self.device.create_texture(&desc);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default()); // TODO: consider not creating a view here
        (tex, view)
    }

    pub(super) fn create_bind_group_of_buffers(
        &self,
        label: &str,
        entries: &[(wgpu::ShaderStages, wgpu::BufferBindingType, &wgpu::Buffer)],
    ) -> wgpu::BindGroup {
        self.create_bind_group_of_buffers_with_offsets(
            label,
            &entries
                .iter()
                .map(|&(vis, ty, buf)| (vis, ty, buf, 0))
                .collect_vec(),
        )
    }
    pub(super) fn create_bind_group_of_buffers_with_offsets(
        &self,
        label: &str,
        entries: &[(
            wgpu::ShaderStages,
            wgpu::BufferBindingType,
            &wgpu::Buffer,
            u64,
        )],
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.create_bind_group_layout_of_buffers(
                &format!("{label}_layout"),
                &entries
                    .iter()
                    .map(|&(vis, ty, _buffer, _offset)| (vis, ty))
                    .collect_vec(),
            ),
            entries: &entries
                .iter()
                .enumerate()
                .map(|(i, &(_vis, _ty, buffer, offset))| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer,
                        offset,
                        size: None,
                    }),
                })
                .collect_vec(),
        })
    }
    pub(super) fn create_bind_group_layout_of_buffers(
        &self,
        label: &str,
        entries: &[(wgpu::ShaderStages, wgpu::BufferBindingType)],
    ) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &entries
                    .iter()
                    .enumerate()
                    .map(|(i, &(visibility, ty))| wgpu::BindGroupLayoutEntry {
                        binding: i as u32,
                        visibility,
                        ty: wgpu::BindingType::Buffer {
                            ty,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    })
                    .collect_vec(),
            })
    }
}

async fn request_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> wgpu::Adapter {
    let mut opts = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
    };

    if let Some(adapter) = instance.request_adapter(&opts).await {
        return adapter;
    }
    opts.force_fallback_adapter = true;
    if let Some(adapter) = instance.request_adapter(&opts).await {
        return adapter;
    }

    panic!("unable to request graphics adapter")
}

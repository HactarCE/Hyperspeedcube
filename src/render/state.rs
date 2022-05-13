use super::shaders::Shaders;

/// Graphics state for the whole window.
pub(crate) struct GraphicsState {
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    pub(crate) surface: wgpu::Surface,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,

    pub(super) shaders: Shaders,

    pub(crate) scale_factor: f64,

    /// 1x1 texture used as a temporary value. Its contents are not important.
    pub(crate) dummy_texture: wgpu::Texture,
}
impl GraphicsState {
    pub(crate) async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        // Create surface.
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };

        // Request adapter.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("unable to request graphics adapter");

        // Request device.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, // VSync on
        };
        surface.configure(&device, &config);

        let shaders = Shaders::new();

        let scale_factor = window.scale_factor();

        let dummy_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy_texture"),
            size: wgpu::Extent3d::default(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
        });

        Self {
            size,
            surface,
            device,
            queue,
            config,

            shaders,

            scale_factor,

            dummy_texture,
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

    pub(crate) fn set_scale_factor(&mut self, new_scale_factor: f64) {
        self.scale_factor = new_scale_factor;
    }

    pub(crate) fn dummy_texture_view(&self) -> wgpu::TextureView {
        self.dummy_texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub(super) fn create_uniform<T>(
        &self,
        label: Option<&str>,
        binding: u32,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: std::mem::size_of::<T>() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: label.map(|s| format!("{s}_bind_group_layout")).as_deref(),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: label.map(|s| format!("{s}_bind_group")).as_deref(),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding,
                    resource: buffer.as_entire_binding(),
                }],
            })
        };

        (buffer, bind_group_layout, bind_group)
    }

    pub(super) fn create_texture(
        &self,
        desc: &wgpu::TextureDescriptor,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = self.device.create_texture(desc);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }
}

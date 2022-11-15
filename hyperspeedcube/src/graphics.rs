use itertools::Itertools;

use crate::{MAX_NDIM, MIN_NDIM};

/// Graphics state for the whole window.
pub(crate) struct GraphicsState {
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    pub(crate) surface: wgpu::Surface,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,

    pub(crate) scale_factor: f64,

    /// 1x1 texture used as a temporary value. Its contents are not important.
    pub(crate) dummy_texture: wgpu::Texture,

    pub(crate) shaders: Shaders,
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
            format: *surface
                .get_supported_formats(&adapter)
                .get(0)
                .expect("unsupported graphics adapter"),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, // VSync on
        };
        surface.configure(&device, &config);

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

        let shaders = Shaders::new(&device);

        Self {
            size,
            surface,
            device,
            queue,
            config,

            scale_factor,

            dummy_texture,

            shaders,
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

    pub(super) fn create_texture_bind_group(
        &self,
        label: Option<&str>,
        visibility: wgpu::ShaderStages,
        ty: wgpu::BindingType,
        view: &wgpu::TextureView,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: label.map(|s| format!("{s}_bind_group_layout")).as_deref(),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility,
                        ty,
                        count: None,
                    }],
                });

        let bind_group = {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: label.map(|s| format!("{s}_bind_group")).as_deref(),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                }],
            })
        };

        (bind_group_layout, bind_group)
    }

    pub(super) fn create_pipeline_layout(
        &self,
        label: &str,
        bind_groups: &[&[(wgpu::ShaderStages, wgpu::BindingType)]],
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> wgpu::PipelineLayout {
        let bind_group_layouts = bind_groups
            .iter()
            .enumerate()
            .map(|(i, binding_types)| {
                let entries = binding_types
                    .iter()
                    .enumerate()
                    .map(|(j, &(visibility, ty))| wgpu::BindGroupLayoutEntry {
                        binding: j as u32,
                        visibility,
                        ty,
                        count: None,
                    })
                    .collect_vec();

                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some(&format!("{label}_pipeline_bind_group_layout_{i}")),
                        entries: &entries,
                    })
            })
            .collect_vec();

        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{label}_pipeline_layout")),
                bind_group_layouts: &bind_group_layouts.iter().collect_vec(),
                push_constant_ranges,
            })
    }

    pub(super) fn create_compute_pipeline(
        &self,
        module: &wgpu::ShaderModule,
        label: &str,
        bind_groups: &[&[(wgpu::ShaderStages, wgpu::BindingType)]],
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> wgpu::ComputePipeline {
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{label}_pipeline")),
                layout: Some(&self.create_pipeline_layout(
                    label,
                    bind_groups,
                    push_constant_ranges,
                )),
                module,
                entry_point: "main",
            })
    }

    pub(super) fn create_texture(
        &self,
        desc: &wgpu::TextureDescriptor,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = self.device.create_texture(desc);
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }

    pub(super) fn create_buffer<T>(
        &self,
        label: &str,
        usage: wgpu::BufferUsages,
        len: usize,
    ) -> wgpu::Buffer {
        let size = len * std::mem::size_of::<T>();
        let size = ndpuzzle::util::next_multiple_of(
            size as u64,
            std::cmp::max(
                wgpu::MAP_ALIGNMENT,
                self.device.limits().min_uniform_buffer_offset_alignment as u64,
            ),
        );

        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    }
    pub(super) fn create_basic_uniform_buffer<T>(&self, label: &str) -> wgpu::Buffer {
        self.create_buffer::<T>(
            label,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            1,
        )
    }
    pub(super) fn create_and_populate_buffer<T: bytemuck::NoUninit>(
        &self,
        label: &str,
        usage: wgpu::BufferUsages,
        data: &[T],
    ) -> wgpu::Buffer {
        let bytes = bytemuck::cast_slice(data);
        let size = ndpuzzle::util::next_multiple_of(bytes.len() as u64, wgpu::MAP_ALIGNMENT);

        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: true,
        });
        buf.slice(..bytes.len() as u64)
            .get_mapped_range_mut()
            .copy_from_slice(bytes);
        buf.unmap();

        buf
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

macro_rules! include_wgsl_with_params {
    ($file_path:literal $(, $var:ident)* $(,)?) => {
        wgpu::ShaderModuleDescriptor {
            label: Some($file_path),
            source: wgpu::ShaderSource::Wgsl(
                include_str!($file_path)
                    $(.replace(
                        concat!("{{", stringify!($var), "}}"),
                        &$var.to_string(),
                    ))*
                    .into(),
            ),
        }
    };
}

pub(super) struct Shaders {
    pub(super) compute_colors: wgpu::ShaderModule,
    pub(super) compute_transform_points: Vec<wgpu::ShaderModule>,
    pub(super) render_polygon_ids: wgpu::ShaderModule,
    pub(super) render_composite_puzzle: wgpu::ShaderModule,
}
impl Shaders {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let workgroup_size = device.limits().max_compute_workgroup_size_x;

        Self {
            compute_colors: device.create_shader_module(include_wgsl_with_params!(
                "shaders/compute_colors.wgsl",
                workgroup_size,
            )),
            compute_transform_points: (MIN_NDIM..=MAX_NDIM)
                .map(|ndim| {
                    device.create_shader_module(include_wgsl_with_params!(
                        "shaders/compute_transform_points.wgsl",
                        ndim,
                        workgroup_size,
                    ))
                })
                .collect(),
            render_polygon_ids: device
                .create_shader_module(wgpu::include_wgsl!("shaders/render_polygon_ids.wgsl")),
            render_composite_puzzle: device
                .create_shader_module(wgpu::include_wgsl!("shaders/render_composite_puzzle.wgsl")),
        }
    }

    pub(super) fn compute_transform_points(&self, ndim: u8) -> Option<&wgpu::ShaderModule> {
        self.compute_transform_points
            .get(ndim.checked_sub(MIN_NDIM)? as usize)
    }
}

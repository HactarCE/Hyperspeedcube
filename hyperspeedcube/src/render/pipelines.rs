use itertools::Itertools;
use std::fmt;

use super::CompositeVertex;

const MIN_NDIM: u8 = 2;
const MAX_NDIM: u8 = 8;

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

macro_rules! bind_group_layout_descriptor {
    (
        $(
            pub($stages:ident) $binding:expr => $binding_type:expr
        ),* $(,)?
    ) => {
        wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[$(
                wgpu::BindGroupLayoutEntry {
                    binding: $binding,
                    visibility: wgpu::ShaderStages::$stages,
                    ty: $binding_type,
                    count: None,
                },
            )*]
        }
    };
}

macro_rules! buffer_bind_group_layout_descriptor {
    (
        $(
            pub($stages:ident) $binding:expr => $buffer_binding_type:expr
         ),* $(,)?
    ) => {
        bind_group_layout_descriptor![
            $(
                pub($stages) $binding => wgpu::BindingType::Buffer {
                    ty: $buffer_binding_type,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            )*
        ]
    };
}

macro_rules! single_type_vertex_buffer {
    ($loc:expr => $fmt:ident) => {
        wgpu::VertexBufferLayout {
            array_stride: ::wgpu::VertexFormat::$fmt.size(),
            step_mode: ::wgpu::VertexStepMode::Vertex,
            attributes: &::wgpu::vertex_attr_array![$loc => $fmt],
        }
    };
}

macro_rules! blend_component {
    ($operation:ident(src * $src_factor:ident, dst * $dst_factor:ident)) => {
        wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::$src_factor,
            dst_factor: wgpu::BlendFactor::$dst_factor,
            operation: wgpu::BlendOperation::$operation,
        }
    };
}

pub(super) struct Pipelines {
    /// Pipeline to populate `vertex_3d_positions` and `vertex_lightings`.
    pub compute_transform_points: Vec<wgpu::ComputePipeline>,
    pub compute_transform_points_bind_groups: PipelineBindGroups,

    /// Pipeline to render the first pass.
    pub render_polygon_ids: wgpu::RenderPipeline,
    pub render_polygon_ids_bind_groups: PipelineBindGroups,

    /// Pipeline to render the second pass.
    pub render_composite_puzzle: wgpu::RenderPipeline,
    pub render_composite_puzzle_bind_groups: PipelineBindGroups,
}
impl Pipelines {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let workgroup_size = device.limits().max_compute_workgroup_size_x;

        let compute_transform_points_bind_groups = PipelineBindGroups::new(
            "compute_transform_points",
            device,
            vec![
                buffer_bind_group_layout_descriptor![
                    pub(COMPUTE) 0 => wgpu::BufferBindingType::Uniform,
                    pub(COMPUTE) 1 => wgpu::BufferBindingType::Uniform,
                    pub(COMPUTE) 2 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 3 => wgpu::BufferBindingType::Storage { read_only: true },
                ],
                buffer_bind_group_layout_descriptor![
                    pub(COMPUTE) 0 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 1 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 2 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 3 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 4 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 5 => wgpu::BufferBindingType::Storage { read_only: true },
                ],
                buffer_bind_group_layout_descriptor![
                    pub(COMPUTE) 0 => wgpu::BufferBindingType::Storage { read_only: true },
                    pub(COMPUTE) 1 => wgpu::BufferBindingType::Storage { read_only: true },
                ],
                buffer_bind_group_layout_descriptor![
                    pub(COMPUTE) 0 => wgpu::BufferBindingType::Storage { read_only: false },
                    pub(COMPUTE) 1 => wgpu::BufferBindingType::Storage { read_only: false },
                ],
            ],
        );
        let compute_transform_points = (MIN_NDIM..=MAX_NDIM)
            .map(|ndim| {
                let compute_transform_points_shader_module =
                    device.create_shader_module(include_wgsl_with_params!(
                        "shaders/compute_transform_points.wgsl",
                        ndim,
                        workgroup_size,
                    ));
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(&format!("compute_transform_points_pipeline_{ndim}")),
                    layout: Some(&compute_transform_points_bind_groups.pipeline_layout(
                        device,
                        &[wgpu::PushConstantRange {
                            stages: wgpu::ShaderStages::COMPUTE,
                            range: 0..4,
                        }],
                    )),
                    module: &compute_transform_points_shader_module,
                    entry_point: "main",
                })
            })
            .collect_vec();

        let render_polygon_ids_bind_groups = PipelineBindGroups::new(
            "render_polygon_ids",
            device,
            vec![buffer_bind_group_layout_descriptor![
                pub(VERTEX) 0 => wgpu::BufferBindingType::Uniform,
            ]],
        );
        let render_polygon_ids_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/render_polygon_ids.wgsl"));
        let render_polygon_ids = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_polygon_ids_pipeline"),
            layout: Some(&render_polygon_ids_bind_groups.pipeline_layout(device, &[])),
            vertex: wgpu::VertexState {
                module: &render_polygon_ids_shader_module,
                entry_point: "vs_main",
                buffers: &[
                    single_type_vertex_buffer![0 => Float32x4], // position
                    single_type_vertex_buffer![1 => Float32],   // lighting
                    single_type_vertex_buffer![2 => Sint32],    // facet_id
                    single_type_vertex_buffer![3 => Sint32],    // polygon_id
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &render_polygon_ids_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg32Sint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let render_composite_puzzle_bind_groups = PipelineBindGroups::new(
            "render_composite_puzzle",
            device,
            vec![
                buffer_bind_group_layout_descriptor![
                    pub(FRAGMENT) 0 => wgpu::BufferBindingType::Uniform,
                    pub(FRAGMENT) 1 => wgpu::BufferBindingType::Uniform,
                ],
                bind_group_layout_descriptor![
                    pub(FRAGMENT) 0 => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Sint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ],
                buffer_bind_group_layout_descriptor![
                    pub(FRAGMENT) 0 => wgpu::BufferBindingType::Storage { read_only: true },
                ],
            ],
        );
        let render_composite_puzzle_shader_module = device
            .create_shader_module(wgpu::include_wgsl!("shaders/render_composite_puzzle.wgsl"));
        let render_composite_puzzle =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_composite_puzzle_pipeline"),
                layout: Some(&render_composite_puzzle_bind_groups.pipeline_layout(device, &[])),
                vertex: wgpu::VertexState {
                    module: &render_composite_puzzle_shader_module,
                    entry_point: "vs_main",
                    buffers: &[CompositeVertex::LAYOUT],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &render_composite_puzzle_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: blend_component!(Add(src * SrcAlpha, dst * One)),
                            alpha: blend_component!(Add(src * One, dst * One)),
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            });

        // TODO: lazily create pipelines
        Self {
            compute_transform_points,
            compute_transform_points_bind_groups,

            render_polygon_ids,
            render_polygon_ids_bind_groups,

            render_composite_puzzle,
            render_composite_puzzle_bind_groups,
        }
    }

    pub(super) fn compute_transform_points(&self, ndim: u8) -> Option<&wgpu::ComputePipeline> {
        self.compute_transform_points
            .get(ndim.checked_sub(MIN_NDIM)? as usize)
    }
}

pub(super) struct PipelineBindGroups {
    label: String,
    bind_group_layout_descriptors: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
    bind_group_layouts: Vec<wgpu::BindGroupLayout>,
}
impl PipelineBindGroups {
    fn new(
        label: impl fmt::Display,
        device: &wgpu::Device,
        bind_group_layout_descriptors: Vec<wgpu::BindGroupLayoutDescriptor<'static>>,
    ) -> Self {
        let bind_group_layouts = bind_group_layout_descriptors
            .iter()
            .map(|bind_group| device.create_bind_group_layout(bind_group))
            .collect_vec();
        PipelineBindGroups {
            label: label.to_string(),
            bind_group_layouts,
            bind_group_layout_descriptors,
        }
    }

    pub fn pipeline_layout(
        &self,
        device: &wgpu::Device,
        push_constant_ranges: &[wgpu::PushConstantRange],
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_pipeline_layout", self.label)),
            bind_group_layouts: &self.bind_group_layouts.iter().collect_vec(),
            push_constant_ranges,
        })
    }

    pub fn bind_groups(
        &self,
        device: &wgpu::Device,
        bind_groups: &[&[wgpu::BindingResource]],
    ) -> Vec<wgpu::BindGroup> {
        assert_eq!(
            self.bind_group_layouts.len(),
            bind_groups.len(),
            "wrong number of bind groups"
        );

        itertools::izip!(
            &self.bind_group_layout_descriptors,
            &self.bind_group_layouts,
            bind_groups,
        )
        .map(|(layout_desc, layout, &bind_group)| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: layout_desc.label,
                layout,
                entries: &itertools::zip_eq(layout_desc.entries, bind_group)
                    .map(|(entry_desc, resource)| wgpu::BindGroupEntry {
                        binding: entry_desc.binding,
                        resource: resource.clone(),
                    })
                    .collect_vec(),
            })
        })
        .collect_vec()
    }
}

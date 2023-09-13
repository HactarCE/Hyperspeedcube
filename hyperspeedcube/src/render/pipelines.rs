use std::fmt;

use anyhow::ensure;
use itertools::Itertools;

use super::CompositeVertex;

const MIN_NDIM: u8 = 2;
const MAX_NDIM: u8 = 8;

mod bindings {
    macro_rules! bindings {
        ($($name:ident = ($binding:expr, $binding_type:expr);)+) => {
            $(
                pub const $name: (u32, wgpu::BindingType) = ($binding, $binding_type);
            )+
        };
    }
    macro_rules! buffer_bindings {
        ($($name:ident = ($binding:expr, $buffer_binding_type:expr);)+) => {
            bindings! {
                $($name = ($binding, wgpu::BindingType::Buffer {
                    ty: $buffer_binding_type,
                    has_dynamic_offset: false,
                    min_binding_size: None,
            }   );)+
            }
        };
    }

    buffer_bindings! {
        // Static mesh data (per-vertex)
        VERTEX_POSITIONS       = (0, wgpu::BufferBindingType::Storage { read_only: true });
        U_TANGENTS             = (1, wgpu::BufferBindingType::Storage { read_only: true });
        V_TANGENTS             = (2, wgpu::BufferBindingType::Storage { read_only: true });
        STICKER_SHRINK_VECTORS = (3, wgpu::BufferBindingType::Storage { read_only: true });
        PIECE_IDS              = (4, wgpu::BufferBindingType::Storage { read_only: true });
        FACET_IDS              = (5, wgpu::BufferBindingType::Storage { read_only: true });

        // Static mesh data (other)
        PIECE_CENTROIDS        = (0, wgpu::BufferBindingType::Storage { read_only: true });
        FACET_CENTROIDS        = (1, wgpu::BufferBindingType::Storage { read_only: true });
        POLYGON_COLOR_IDS      = (2, wgpu::BufferBindingType::Storage { read_only: true });
        COLOR_VALUES           = (3, wgpu::BufferBindingType::Storage { read_only: true });
        // Computed data (per-vertex)
        VERTEX_3D_POSITIONS    = (4, wgpu::BufferBindingType::Storage { read_only: false });
        VERTEX_LIGHTINGS       = (5, wgpu::BufferBindingType::Storage { read_only: false });

        // View parameters and transforms
        PUZZLE_TRANSFORM       = (0, wgpu::BufferBindingType::Storage { read_only: true });
        PIECE_TRANSFORMS       = (1, wgpu::BufferBindingType::Storage { read_only: true });
        PROJECTION_PARAMS      = (2, wgpu::BufferBindingType::Uniform);
        LIGHTING_PARAMS        = (3, wgpu::BufferBindingType::Uniform);
        VIEW_PARAMS            = (4, wgpu::BufferBindingType::Uniform);

        // Composite parameters
        COMPOSITE_PARAMS       = (0, wgpu::BufferBindingType::Uniform);
        SPECIAL_COLORS         = (1, wgpu::BufferBindingType::Uniform);
    }
    bindings! {
        POLYGON_IDS_TEXTURE = (50, wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Sint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        });
    }
}

macro_rules! bind_groups {
    ($($index:literal => [$(pub($stages:ident) $name:ident),* $(,)?]),* $(,)?) => {{
        assert!(
            itertools::izip!([$($index),*], 0..).all(|(a, b)| a == b),
            "bind groups must be sequential",
        );
        &[$(
            &[$(
                wgpu::BindGroupLayoutEntry {
                    binding: bindings::$name.0,
                    visibility: wgpu::ShaderStages::$stages,
                    ty: bindings::$name.1,
                    count: None,
                },
            )*],
        )*]
    }};
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

    pub render_single_pass: Vec<wgpu::RenderPipeline>,
    pub render_single_pass_bind_groups: PipelineBindGroups,
}
impl Pipelines {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        fn make_compute_pipeline(
            device: &wgpu::Device,
            shader_module: &wgpu::ShaderModule,
            bind_groups: &PipelineBindGroups,
        ) -> wgpu::ComputePipeline {
            let label = &bind_groups.label;
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{label}_pipeline")),
                layout: Some(&bind_groups.pipeline_layout(device)),
                module: shader_module,
                entry_point: label,
            })
        }

        fn make_render_pipeline(
            device: &wgpu::Device,
            shader_module: &wgpu::ShaderModule,
            bind_groups: &PipelineBindGroups,
            desc: BasicRenderPipelineDescriptor<'_>,
        ) -> wgpu::RenderPipeline {
            let label = &bind_groups.label;
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{label}_pipeline")),
                layout: Some(&bind_groups.pipeline_layout(device)),
                vertex: wgpu::VertexState {
                    module: shader_module,
                    entry_point: &format!("{label}_vertex"),
                    buffers: desc.vertex_buffers,
                },
                primitive: desc.primitive,
                depth_stencil: desc.depth_stencil,
                multisample: desc.multisample,
                fragment: Some(wgpu::FragmentState {
                    module: shader_module,
                    entry_point: &format!("{label}_fragment"),
                    targets: &[desc.fragment_target],
                }),
                multiview: None,
            })
        }

        let shader_modules = (MIN_NDIM..=MAX_NDIM)
            .map(|ndim| device.create_shader_module(include_wgsl_with_params!("shader.wgsl", ndim)))
            .collect_vec();

        let compute_transform_points_bind_groups = PipelineBindGroups::new(
            "compute_transform_points",
            device,
            bind_groups![
                0 => [
                    pub(COMPUTE) VERTEX_POSITIONS,
                    pub(COMPUTE) U_TANGENTS,
                    pub(COMPUTE) V_TANGENTS,
                    pub(COMPUTE) STICKER_SHRINK_VECTORS,
                    pub(COMPUTE) PIECE_IDS,
                    pub(COMPUTE) FACET_IDS,
                ],
                1 => [
                    pub(COMPUTE) PIECE_CENTROIDS,
                    pub(COMPUTE) FACET_CENTROIDS,
                    pub(COMPUTE) VERTEX_3D_POSITIONS,
                    pub(COMPUTE) VERTEX_LIGHTINGS,
                ],
                2 => [
                    pub(COMPUTE) PUZZLE_TRANSFORM,
                    pub(COMPUTE) PIECE_TRANSFORMS,
                    pub(COMPUTE) PROJECTION_PARAMS,
                    pub(COMPUTE) LIGHTING_PARAMS,
                ],
            ],
        );
        let compute_transform_points = shader_modules
            .iter()
            .map(|shader_module| {
                make_compute_pipeline(device, shader_module, &compute_transform_points_bind_groups)
            })
            .collect_vec();

        let render_polygon_ids_bind_groups = PipelineBindGroups::new(
            "render_polygon_ids",
            device,
            bind_groups![0 => [], 1 => [], 2 => [pub(VERTEX) VIEW_PARAMS]],
        );
        let render_polygon_ids = make_render_pipeline(
            device,
            &shader_modules[0],
            &render_polygon_ids_bind_groups,
            BasicRenderPipelineDescriptor {
                vertex_buffers: &[
                    single_type_vertex_buffer![0 => Float32x4], // position
                    single_type_vertex_buffer![1 => Float32],   // lighting
                    single_type_vertex_buffer![2 => Sint32],    // polygon_id
                ],
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Greater,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                fragment_target: Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg32Sint,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                ..Default::default()
            },
        );

        let render_composite_puzzle_bind_groups = PipelineBindGroups::new(
            "render_composite_puzzle",
            device,
            bind_groups![
                0 => [],
                1 => [pub(FRAGMENT) POLYGON_COLOR_IDS, pub(FRAGMENT) COLOR_VALUES],
                2 => [pub(FRAGMENT) POLYGON_IDS_TEXTURE],
                3 => [pub(FRAGMENT) COMPOSITE_PARAMS, pub(FRAGMENT) SPECIAL_COLORS],

            ],
        );
        let render_composite_puzzle = make_render_pipeline(
            device,
            &shader_modules[0],
            &render_composite_puzzle_bind_groups,
            BasicRenderPipelineDescriptor {
                vertex_buffers: &[CompositeVertex::LAYOUT],
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                fragment_target: Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: blend_component!(Add(src * SrcAlpha, dst * One)),
                        alpha: blend_component!(Add(src * One, dst * One)),
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                ..Default::default()
            },
        );

        let render_single_pass_bind_groups = PipelineBindGroups::new(
            "render_single_pass",
            device,
            bind_groups![
                0 => [
                    pub(VERTEX) VERTEX_POSITIONS,
                    pub(VERTEX) U_TANGENTS,
                    pub(VERTEX) V_TANGENTS,
                    pub(VERTEX) STICKER_SHRINK_VECTORS,
                ],
                1 => [
                    pub(VERTEX) PIECE_CENTROIDS,
                    pub(VERTEX) FACET_CENTROIDS,
                    pub(FRAGMENT) POLYGON_COLOR_IDS,
                    pub(FRAGMENT) COLOR_VALUES,
                ],
                2 => [
                    pub(VERTEX) PUZZLE_TRANSFORM,
                    pub(VERTEX) PIECE_TRANSFORMS,
                    pub(VERTEX) PROJECTION_PARAMS,
                    pub(VERTEX) LIGHTING_PARAMS,
                    pub(VERTEX) VIEW_PARAMS,
                ],
            ],
        );
        let render_single_pass = shader_modules
            .iter()
            .map(|shader_module| {
                make_render_pipeline(
                    device,
                    shader_module,
                    &render_single_pass_bind_groups,
                    BasicRenderPipelineDescriptor {
                        vertex_buffers: &[
                            single_type_vertex_buffer![0 => Sint32], // piece_id
                            single_type_vertex_buffer![1 => Sint32], // facet_id
                            single_type_vertex_buffer![2 => Sint32], // polygon_id
                        ],
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: wgpu::TextureFormat::Depth24PlusStencil8,
                            depth_write_enabled: true,
                            depth_compare: wgpu::CompareFunction::Greater,
                            stencil: wgpu::StencilState::default(),
                            bias: wgpu::DepthBiasState::default(),
                        }),
                        fragment_target: Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8Unorm,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        ..Default::default()
                    },
                )
            })
            .collect_vec();

        // TODO: lazily create pipelines
        Self {
            compute_transform_points,
            compute_transform_points_bind_groups,

            render_polygon_ids,
            render_polygon_ids_bind_groups,

            render_composite_puzzle,
            render_composite_puzzle_bind_groups,

            render_single_pass,
            render_single_pass_bind_groups,
        }
    }

    pub(super) fn compute_transform_points(&self, ndim: u8) -> Option<&wgpu::ComputePipeline> {
        self.compute_transform_points
            .get(ndim.checked_sub(MIN_NDIM)? as usize)
    }

    pub(super) fn render_single_pass(&self, ndim: u8) -> Option<&wgpu::RenderPipeline> {
        self.render_single_pass
            .get(ndim.checked_sub(MIN_NDIM)? as usize)
    }
}

pub(super) struct PipelineBindGroups {
    label: String,
    bind_group_layouts: Vec<(
        wgpu::BindGroupLayout,
        wgpu::BindGroupLayoutDescriptor<'static>,
    )>,
}
impl PipelineBindGroups {
    fn new(
        label: impl fmt::Display,
        device: &wgpu::Device,
        bind_group_bindings: &'static [&'static [wgpu::BindGroupLayoutEntry]],
    ) -> Self {
        PipelineBindGroups {
            label: label.to_string(),
            bind_group_layouts: bind_group_bindings
                .iter()
                .map(|entries| {
                    let layout_descriptor = wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries,
                    };
                    (
                        device.create_bind_group_layout(&layout_descriptor),
                        layout_descriptor,
                    )
                })
                .collect(),
        }
    }

    pub fn pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        let empty_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[],
            });
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_pipeline_layout", self.label)),
            bind_group_layouts: &self
                .bind_group_layouts
                .iter()
                .map(|(layout, _desc)| layout)
                .collect_vec(),
            push_constant_ranges: &[],
        })
    }

    pub fn bind_groups(
        &self,
        device: &wgpu::Device,
        bind_groups: &[&[wgpu::BindingResource]],
    ) -> Vec<(u32, wgpu::BindGroup)> {
        assert_eq!(
            self.bind_group_layouts
                .iter()
                .map(|(_, desc)| desc.entries.len())
                .collect_vec(),
            bind_groups
                .iter()
                .map(|bind_group| bind_group.len())
                .collect_vec(),
            "bind groups layout mismatch",
        );

        let bind_groups = itertools::zip_eq(&self.bind_group_layouts, bind_groups).map(
            |((layout, layout_desc), bind_group)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: layout_desc.label,
                    layout,
                    entries: &itertools::zip_eq(layout_desc.entries, *bind_group)
                        .map(|(entry_desc, resource)| wgpu::BindGroupEntry {
                            binding: entry_desc.binding,
                            resource: resource.clone(),
                        })
                        .collect_vec(),
                })
            },
        );

        (0..).zip(bind_groups).collect()
    }
}

#[derive(Default)]
struct BasicRenderPipelineDescriptor<'a> {
    vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    fragment_target: Option<wgpu::ColorTargetState>,
}

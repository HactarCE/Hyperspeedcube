use itertools::Itertools;
use std::fmt;

use super::BasicVertex;

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
            pub($stages:ident) $binding:expr => $binding_type:expr,
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
            pub($stages:ident) $binding:expr => $buffer_binding_type:expr,
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

pub(super) struct Pipelines {
    /// Pipeline to render a basic cube.
    pub render_basic: wgpu::RenderPipeline,
    pub render_basic_bind_groups: PipelineBindGroups,

    /// Pipeline to populate `vertex_3d_position_buffer`.
    pub compute_transform_points: Vec<wgpu::ComputePipeline>,
    // /// Pipeline to populate `polygon_color_buffer`.
    // pub compute_polygon_colors: wgpu::ComputePipeline,
    // /// Pipeline to render to `polygon_ids_texture`.
    // pub render_polygon_ids: wgpu::RenderPipeline,
    // /// Pipeline to render to `out_texture`.
    // pub render_composite_puzzle: wgpu::RenderPipeline,
}
impl Pipelines {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
        const STORAGE_READ: wgpu::BufferBindingType =
            wgpu::BufferBindingType::Storage { read_only: true };
        const STORAGE_WRITE: wgpu::BufferBindingType =
            wgpu::BufferBindingType::Storage { read_only: false };

        let workgroup_size = device.limits().max_compute_workgroup_size_x;

        let render_basic_bind_groups = PipelineBindGroups::new(
            "render_basic",
            device,
            vec![buffer_bind_group_layout_descriptor![
                pub(VERTEX) 0 => wgpu::BufferBindingType::Uniform,
            ]],
        );

        // TODO: lazily create pipelines
        Self {
            render_basic: {
                let shader_module =
                    device.create_shader_module(wgpu::include_wgsl!("shaders/render_basic.wgsl"));

                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("render_basic"),
                    layout: Some(&render_basic_bind_groups.pipeline_layout(device)),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: "vs_main",
                        buffers: &[BasicVertex::LAYOUT],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24Plus,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Greater,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8Unorm,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                })
            },
            render_basic_bind_groups,

            compute_transform_points: vec![],
            // compute_transform_points: {
            //     (MIN_NDIM..=MAX_NDIM)
            //         .map(|ndim| {
            //             let shader = device.create_shader_module(include_wgsl_with_params!(
            //                 "shaders/compute_transform_points.wgsl",
            //                 ndim,
            //                 workgroup_size,
            //             ));
            //             let uniform_buffer_bindings = compute_buffer_binding_types([UNIFORM]);
            //             let storage_buffer_bindings = compute_buffer_binding_types([
            //                 STORAGE_READ,
            //                 STORAGE_READ,
            //                 STORAGE_READ,
            //                 STORAGE_WRITE,
            //             ]);
            //             let label = "compute_transform_points";
            //             let bind_groups = [&uniform_buffer_bindings, &storage_buffer_bindings];

            //             device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            //                 label: Some(&format!("{label}_pipeline")),
            //                 layout: Some(&create_pipeline_layout(device, label, &bind_groups, &[])),
            //                 module: &shader,
            //                 entry_point: "main",
            //             })
            //         })
            //         .collect()
            // },

            // compute_polygon_colors: todo!(),
            // render_polygon_ids: todo!(),
            // render_composite_puzzle: todo!(),
        }
    }

    pub(super) fn compute_transform_points(&self, ndim: u8) -> Option<&wgpu::ComputePipeline> {
        self.compute_transform_points
            .get(ndim.checked_sub(MIN_NDIM)? as usize)
    }
}

fn create_pipeline_layout(
    device: &wgpu::Device,
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

            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&format!("{label}_pipeline_bind_group_layout_{i}")),
                entries: &entries,
            })
        })
        .collect_vec();

    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label}_pipeline_layout")),
        bind_group_layouts: &bind_group_layouts.iter().collect_vec(),
        push_constant_ranges,
    })
}

fn compute_buffer_binding_types<const N: usize>(
    entries: [wgpu::BufferBindingType; N],
) -> [(wgpu::ShaderStages, wgpu::BindingType); N] {
    buffer_binding_types(entries.map(|ty| (wgpu::ShaderStages::COMPUTE, ty)))
}

fn buffer_binding_types<const N: usize>(
    entries: [(wgpu::ShaderStages, wgpu::BufferBindingType); N],
) -> [(wgpu::ShaderStages, wgpu::BindingType); N] {
    entries.map(|(visibility, ty)| {
        let binding_type = wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        (visibility, binding_type)
    })
}

fn get_vertex_fragment<'a>(
    module: &'a wgpu::ShaderModule,
    buffers: &'a [wgpu::VertexBufferLayout],
    targets: &'a [Option<wgpu::ColorTargetState>],
) -> (wgpu::VertexState<'a>, Option<wgpu::FragmentState<'a>>) {
    let vertex = wgpu::VertexState {
        module,
        entry_point: "vs_main",
        buffers,
    };
    let fragment = Some(wgpu::FragmentState {
        module,
        entry_point: "fs_main",
        targets,
    });
    (vertex, fragment)
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

    pub fn pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_pipeline_layout", self.label)),
            bind_group_layouts: &self.bind_group_layouts.iter().collect_vec(),
            push_constant_ranges: &[],
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

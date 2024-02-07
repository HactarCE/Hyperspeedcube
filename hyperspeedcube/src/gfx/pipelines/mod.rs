use eyre::{OptionExt, Result};
use std::sync::Arc;

use itertools::Itertools;

use crate::gfx::structs::UvVertex;

pub(in crate::gfx) mod blit;
pub(in crate::gfx) mod compute_transform_points;
pub(in crate::gfx) mod render_composite_puzzle;
pub(in crate::gfx) mod render_edge_ids;
pub(in crate::gfx) mod render_polygon_ids;
pub(in crate::gfx) mod render_single_pass;

#[rustfmt::skip]
mod bindings {
    use wgpu::BufferBindingType::{Storage, Uniform};
    use wgpu::SamplerBindingType::Filtering;
    use wgpu::TextureSampleType::{Float, Uint};
    use wgpu::TextureViewDimension::{D1, D2};

    use crate::gfx::bindings::{buffer, sampler, texture, BindingMetadata};

    // Static mesh data (per-vertex)
    pub const VERTEX_POSITIONS:             BindingMetadata = buffer(0, 0, Storage { read_only: true });
    pub const U_TANGENTS:                   BindingMetadata = buffer(0, 1, Storage { read_only: true });
    pub const V_TANGENTS:                   BindingMetadata = buffer(0, 2, Storage { read_only: true });
    pub const STICKER_SHRINK_VECTORS:       BindingMetadata = buffer(0, 3, Storage { read_only: true });
    pub const PIECE_IDS:                    BindingMetadata = buffer(0, 4, Storage { read_only: true });
    pub const FACET_IDS:                    BindingMetadata = buffer(0, 5, Storage { read_only: true });

    // Static mesh data (other)
    pub const PIECE_CENTROIDS:              BindingMetadata = buffer(1, 0, Storage { read_only: true });
    pub const FACET_CENTROIDS:              BindingMetadata = buffer(1, 1, Storage { read_only: true });
    pub const FACET_NORMALS:                BindingMetadata = buffer(1, 2, Storage { read_only: true });
    pub const POLYGON_COLOR_IDS:            BindingMetadata = buffer(1, 3, Storage { read_only: true });
    pub const EDGE_VERTS:                   BindingMetadata = buffer(1, 4, Storage { read_only: true });
    // Computed data (per-vertex)
    pub const VERTEX_3D_POSITIONS:          BindingMetadata = buffer(1, 5, Storage { read_only: false });
    pub const VERTEX_3D_POSITIONS_READONLY: BindingMetadata = buffer(1, 5, Storage { read_only: true });
    pub const VERTEX_LIGHTINGS:             BindingMetadata = buffer(1, 6, Storage { read_only: false });
    pub const VERTEX_CULLS:                 BindingMetadata = buffer(1, 7, Storage { read_only: false });
    pub const VERTEX_CULLS_READONLY:        BindingMetadata = buffer(1, 7, Storage { read_only: true });

    // View parameters and transforms
    pub const PUZZLE_TRANSFORM:             BindingMetadata = buffer(2, 0, Uniform);
    pub const PIECE_TRANSFORMS:             BindingMetadata = buffer(2, 1, Storage { read_only: true });
    pub const CAMERA_4D_POS:                BindingMetadata = buffer(2, 2, Storage { read_only: true });
    pub const PROJECTION_PARAMS:            BindingMetadata = buffer(2, 3, Uniform);
    pub const LIGHTING_PARAMS:              BindingMetadata = buffer(2, 4, Uniform);
    pub const VIEW_PARAMS:                  BindingMetadata = buffer(2, 5, Uniform);
    pub const TARGET_SIZE:                  BindingMetadata = buffer(2, 6, Uniform);

    // Composite parameters
    pub const COMPOSITE_PARAMS:             BindingMetadata = buffer(3, 0, Uniform);

    pub const STICKER_COLORS_TEXTURE:       BindingMetadata = texture(0, 50, D1, Float { filterable: false });
    pub const SPECIAL_COLORS_TEXTURE:       BindingMetadata = texture(0, 51, D1, Float { filterable: false });

    pub const IDS_TEXTURE:                  BindingMetadata = texture(0, 100, D2, Uint);
    pub const BLIT_SRC_TEXTURE:             BindingMetadata = texture(0, 102, D2, Float { filterable: true });
    pub const BLIT_SRC_SAMPLER:             BindingMetadata = sampler(0, 150, Filtering);
}

const MIN_NDIM: u8 = 2;
const MAX_NDIM: u8 = 8;

pub(in crate::gfx) struct Pipelines {
    /// Populate `vertex_3d_positions`, `vertex_lightings`, and `vertex_culls`.
    pub compute_transform_points: Vec<compute_transform_points::Pipeline>,
    /// Render polygon IDs.
    pub render_polygon_ids: render_polygon_ids::Pipeline,
    /// Render edge IDs.
    pub render_edge_ids: render_edge_ids::Pipeline,
    /// Render color from polygon IDs.
    pub render_composite_puzzle: render_composite_puzzle::Pipeline,
    /// Render the whole puzzle in a single pass.
    pub render_single_pass: Vec<render_single_pass::Pipeline>,
    /// Blit a texture onto another texture.
    pub blit: blit::Pipeline,
}
impl Pipelines {
    pub(super) fn new(device: &Arc<wgpu::Device>, target_format: wgpu::TextureFormat) -> Self {
        let shader_modules_by_dimension = (MIN_NDIM..=MAX_NDIM)
            .map(|ndim| device.create_shader_module(include_wgsl!("../shader.wgsl", ndim)))
            .collect_vec();
        // Most shader programs don't care about the number of dimensions.
        let shader_module = &shader_modules_by_dimension[0];

        // TODO: lazily create pipelines
        Self {
            compute_transform_points: shader_modules_by_dimension
                .iter()
                .map(|shader_module| compute_transform_points::Pipeline::new(device, shader_module))
                .collect(),
            render_polygon_ids: render_polygon_ids::Pipeline::new(device, shader_module),
            render_edge_ids: render_edge_ids::Pipeline::new(device, shader_module),
            render_composite_puzzle: render_composite_puzzle::Pipeline::new(device, shader_module),
            render_single_pass: shader_modules_by_dimension
                .iter()
                .map(|shader_module| render_single_pass::Pipeline::new(device, shader_module))
                .collect(),
            blit: blit::Pipeline::new(
                device,
                shader_module,
                blit::PipelineParams { target_format },
            ),
        }
    }

    pub(super) fn compute_transform_points(
        &self,
        ndim: u8,
    ) -> Result<&compute_transform_points::Pipeline> {
        ndim.checked_sub(MIN_NDIM)
            .and_then(|ndim| self.compute_transform_points.get(ndim as usize))
            .ok_or_eyre("error fetching transform points compute pipeline")
    }

    pub(super) fn render_single_pass(&self, ndim: u8) -> Result<&render_single_pass::Pipeline> {
        ndim.checked_sub(MIN_NDIM)
            .and_then(|ndim| self.render_single_pass.get(ndim as usize))
            .ok_or_eyre("error fetching single pass render pipeline")
    }
}

#[derive(Default)]
struct RenderPipelineDescriptor<'a> {
    label: &'a str,
    vertex_entry_point: &'a str,
    fragment_entry_point: &'a str,
    vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    fragment_target: Option<wgpu::ColorTargetState>,
}
impl RenderPipelineDescriptor<'_> {
    pub fn create_pipeline(
        self,
        device: &wgpu::Device,
        shader_module: &wgpu::ShaderModule,
        pipeline_layout: &wgpu::PipelineLayout,
    ) -> wgpu::RenderPipeline {
        let vertex_entry_point = match self.vertex_entry_point {
            "" => format!("{}_vertex", self.label),
            s => s.to_owned(),
        };
        let fragment_entry_point = match self.fragment_entry_point {
            "" => format!("{}_fragment", self.label),
            s => s.to_owned(),
        };

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{}_pipeline", self.label)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: &vertex_entry_point,
                buffers: self.vertex_buffers,
            },
            primitive: self.primitive,
            depth_stencil: self.depth_stencil,
            multisample: self.multisample,
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: &fragment_entry_point,
                targets: &[self.fragment_target],
            }),
            multiview: None,
        })
    }
}

pub struct ComputePipelineDescriptor<'a> {
    label: &'a str,
    entry_point: &'a str,
}
impl ComputePipelineDescriptor<'_> {
    pub fn create_pipeline(
        self,
        device: &wgpu::Device,
        shader_module: &wgpu::ShaderModule,
        pipeline_layout: &wgpu::PipelineLayout,
    ) -> wgpu::ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(&format!("{}_pipeline", self.label)),
            layout: Some(&pipeline_layout),
            module: shader_module,
            entry_point: &self.entry_point,
        })
    }
}

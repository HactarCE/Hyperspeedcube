use anyhow::{Context, Result};
use itertools::Itertools;
use ndpuzzle::math::VectorRef;
use ndpuzzle::puzzle::PuzzleType;
use std::fmt;

use super::structs::*;
use super::GraphicsState;

macro_rules! blend_component {
    ($op:ident(src * $src_factor:ident, dst * $dst_factor:ident)) => {
        wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::$src_factor,
            dst_factor: wgpu::BlendFactor::$dst_factor,
            operation: wgpu::BlendOperation::$op,
        }
    };
}

pub(crate) struct PuzzleRenderCache {
    /// For each sticker: indices into `vertex_buffer`
    pub(super) indices_per_sticker: Vec<Box<[u32]>>,

    /*
     * VERTEX AND INDEX BUFFERS
     */
    /// Every pair of polygon ID and vertex ID that appears in the puzzle model.
    pub(super) vertex_buffer: wgpu::Buffer,
    /// Indices into `vertex_buffer` for drawing triangles.
    pub(super) index_buffer: wgpu::Buffer,
    /// Full-screen quad vertices.
    pub(super) composite_quad_vertex_buffer: wgpu::Buffer,

    /*
     * SMALL UNIFORMS
     */
    /// Projection parameters.
    pub(super) projection_params_buffer: wgpu::Buffer,
    /// Lighting parameters.
    pub(super) lighting_params_buffer: wgpu::Buffer,
    /// 2D view parameters.
    pub(super) view_params_buffer: wgpu::Buffer,
    /// Compositing parameters.
    pub(super) composite_params_buffer: wgpu::Buffer,

    /*
     * OTHER BUFFERS
     */
    /// View transform from N-dimensional space to 4D space as an Nx4 matrix.
    pub(super) puzzle_transform_buffer: wgpu::Buffer,

    /// For each piece: its transform in N-dimensional space as an NxN matrix.
    ///
    /// TODO: consider changing this to an N-dimensional rotor.
    pub(super) piece_transform_buffer: wgpu::Buffer,

    /// For each facet: the point to shrink towards for facet spacing.
    pub(super) facet_center_buffer: wgpu::Buffer,

    /// For each sticker: the ID of its facet and the ID of its piece.
    pub(super) sticker_info_buffer: wgpu::Buffer,

    /// Number of polygons, which is the length of each "per-polygon" buffer.
    pub(super) polygon_count: usize,
    /// For each polygon: the ID of its facet and the three vertex IDs for one
    /// of its triangles (used to compute its normal in 3D space).
    pub(super) polygon_info_buffer: wgpu::Buffer,
    /// For each polygon: its color, which is computed from its normal and its
    /// facet's color.
    pub(super) polygon_color_buffer: wgpu::Buffer,

    /// Number of vertices, which is the length of each "per-vertex" buffer.
    pub(super) vertex_count: usize,
    /// For each vertex: the ID of its sticker.
    pub(super) vertex_sticker_id_buffer: wgpu::Buffer,
    /// For each vertex: its position in N-dimensional space.
    pub(super) vertex_position_buffer: wgpu::Buffer,
    /// For each vertex: the vector it shrinks along for sticker spacing.
    pub(super) vertex_shrink_vector_buffer: wgpu::Buffer,
    /// For each vertex: its position in 3D space, which is recomputed whenever
    /// the view angle or puzzle geometry changes (e.g., each frame of a twist
    /// animation).
    pub(super) vertex_3d_position_buffer: wgpu::Buffer,

    /*
     * PIPELINES
     */
    /// Pipeline to populate `vertex_3d_position_buffer`.
    pub(super) compute_transform_points_pipeline: wgpu::ComputePipeline,
    /// Pipeline to populate `polygon_color_buffer`.
    pub(super) compute_polygon_colors_pipeline: wgpu::ComputePipeline,
    /// Pipeline to render to `polygon_ids_texture`.
    pub(super) render_polygon_ids_pipeline: wgpu::RenderPipeline,
    /// Pipeline to render to `out_texture`.
    pub(super) render_composite_puzzle_pipeline: wgpu::RenderPipeline,

    /*
     * TEXTURES
     */
    pub(super) facet_colors_texture: CachedTexture, // TODO: doesn't need to change size
    pub(super) polygon_depth_texture: CachedTexture,
    pub(super) polygon_ids_texture: CachedTexture,
    pub(super) out_texture: CachedTexture,
}
impl fmt::Debug for PuzzleRenderCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleRenderCache").finish_non_exhaustive()
    }
}
impl PuzzleRenderCache {
    pub fn new(gfx: &mut GraphicsState, ty: &PuzzleType) -> Result<Self> {
        let ndim = ty.ndim();

        let facet_center_data = ty
            .shape
            .facets
            .iter()
            .flat_map(|facet| facet.pole.iter_ndim(ndim))
            .collect_vec();

        // Create static buffer data.
        let mut triangle_count = 0;
        let mut indices_per_sticker = vec![];
        let mut vertex_data = vec![];
        let mut sticker_info_data = vec![];
        let mut polygon_info_data = vec![];
        let mut vertex_sticker_id_data = vec![];
        let mut vertex_position_data = vec![];
        let mut vertex_shrink_vector_data = vec![];
        {
            let mut polygon_idx = 0;
            let mut degerate_polygons_count = 0;

            for (sticker_idx, sticker) in ty.stickers.iter().enumerate() {
                // For each sticker ...
                sticker_info_data.push(GfxStickerInfo {
                    piece: sticker.piece.0 as u32,
                    facet: sticker.color.0 as u32,
                });

                // For each polygon ...
                let vertex_list_base = vertex_sticker_id_data.len() as u32;
                let mut current_sticker_indices = vec![];
                for polygon in &sticker.polygons {
                    // Ignore degenerate polygons.
                    if polygon.len() < 3 {
                        degerate_polygons_count += 1;
                        continue;
                    }

                    // The first three vertices will determine the polygon's
                    // normal vector.
                    let v0 = vertex_list_base + polygon[0] as u32;
                    let v1 = vertex_list_base + polygon[1] as u32;
                    let v2 = vertex_list_base + polygon[2] as u32;
                    polygon_info_data.push(GfxPolygonInfo {
                        facet: sticker.color.0 as u32,
                        v0,
                        v1,
                        v2,
                    });

                    // For each vertex in each polygon ...
                    let vertex_data_list_base = vertex_data.len() as u32;
                    for &vertex_id in polygon {
                        vertex_data.push(PolygonVertex {
                            polygon: polygon_idx as i32,
                            vertex: vertex_list_base + vertex_id as u32,
                        });
                    }

                    // For each triangle in each polygon ...
                    for (b, c) in (1..polygon.len()).circular_tuple_windows() {
                        current_sticker_indices.extend([
                            vertex_data_list_base,
                            vertex_data_list_base + b as u32,
                            vertex_data_list_base + c as u32,
                        ]);
                        triangle_count += 1;
                    }

                    polygon_idx += 1;
                }
                indices_per_sticker.push(current_sticker_indices.into_boxed_slice());

                // For each vertex ...
                for (point, shrink_vector) in sticker.points.iter().zip(&sticker.shrink_vectors) {
                    vertex_sticker_id_data.push(sticker_idx as u32);
                    vertex_position_data.extend(point.iter_ndim(ndim));
                    vertex_shrink_vector_data.extend(shrink_vector.iter_ndim(ndim));
                }
            }

            if degerate_polygons_count != 0 {
                log::warn!(
                    "Removed {degerate_polygons_count} degenerate polygons from puzzle model"
                );
            }
        }

        // Create pipelines.
        let compute_transform_points_pipeline;
        let compute_polygon_colors_pipeline;
        let render_polygon_ids_pipeline;
        let render_composite_puzzle_pipeline;
        {
            const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
            const STORAGE_READ: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: true };
            const STORAGE_WRITE: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: false };

            compute_transform_points_pipeline = gfx.create_compute_pipeline(
                gfx.shaders
                    .compute_transform_points(ty.ndim())
                    .context(format!(
                        "cannot render puzzle with {} dimensions",
                        ty.ndim(),
                    ))?,
                "compute_transform_points",
                &[
                    &compute_buffer_binding_types([UNIFORM]),
                    &compute_buffer_binding_types([
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_READ,
                        STORAGE_WRITE,
                    ]),
                ],
                &[],
            );

            compute_polygon_colors_pipeline = gfx.create_compute_pipeline(
                &gfx.shaders.compute_colors,
                "compute_polygon_colors",
                &[
                    &compute_buffer_binding_types([UNIFORM]),
                    &compute_buffer_binding_types([STORAGE_READ, STORAGE_WRITE, STORAGE_READ]),
                    &[(
                        wgpu::ShaderStages::COMPUTE,
                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D1,
                            multisampled: false,
                        },
                    )],
                ],
                &[],
            );

            render_polygon_ids_pipeline = {
                let color_targets = [Some(wgpu::TextureFormat::R32Sint.into())];
                let (vertex, fragment) = get_vertex_fragment(
                    &gfx.shaders.render_polygon_ids,
                    &[PolygonVertex::LAYOUT],
                    &color_targets,
                );

                gfx.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("polygon_ids_render_pipeline"),
                        layout: Some(&gfx.device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("polygon_ids_render_pipeline_layout"),
                                bind_group_layouts: &[&gfx.create_bind_group_layout_of_buffers(
                                    "polygon_ids_render_pipeline_bindings_layout",
                                    &[
                                        (wgpu::ShaderStages::VERTEX, UNIFORM),
                                        (wgpu::ShaderStages::VERTEX, STORAGE_READ),
                                    ],
                                )],
                                push_constant_ranges: &[],
                            },
                        )),
                        vertex,
                        primitive: wgpu::PrimitiveState {
                            cull_mode: None,
                            ..Default::default()
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: wgpu::TextureFormat::Depth32Float,
                            depth_write_enabled: true,
                            depth_compare: wgpu::CompareFunction::Greater,
                            stencil: wgpu::StencilState::default(),
                            bias: wgpu::DepthBiasState::default(),
                        }),
                        multisample: wgpu::MultisampleState::default(),
                        fragment,
                        multiview: None,
                    })
            };

            render_composite_puzzle_pipeline = {
                let (vertex, fragment) = get_vertex_fragment(
                    &gfx.shaders.render_composite_puzzle,
                    &[CompositeVertex::LAYOUT],
                    &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: blend_component!(Add(src * SrcAlpha, dst * One)),
                            alpha: blend_component!(Add(src * SrcAlpha, dst * One)),
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                );

                gfx.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("puzzle_composite_render_pipeline"),
                        layout: Some(&gfx.create_pipeline_layout(
                            "puzzle_composite",
                            &[
                                &buffer_binding_types([(wgpu::ShaderStages::FRAGMENT, UNIFORM)]),
                                &buffer_binding_types([(
                                    wgpu::ShaderStages::FRAGMENT,
                                    STORAGE_READ,
                                )]),
                                &[(
                                    wgpu::ShaderStages::FRAGMENT,
                                    wgpu::BindingType::Texture {
                                        sample_type: wgpu::TextureSampleType::Sint,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        multisampled: false,
                                    },
                                )],
                            ],
                            &[],
                        )),
                        vertex,
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleStrip,
                            ..Default::default()
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState::default(),
                        fragment,
                        multiview: None,
                    })
            };
        }

        Ok(Self {
            indices_per_sticker,

            vertex_buffer: gfx.create_and_populate_buffer(
                "puzzle_geometry_vertex_buffer",
                wgpu::BufferUsages::VERTEX,
                vertex_data.as_slice(),
            ),
            index_buffer: gfx.create_buffer::<[u32; 3]>(
                "puzzle_geometry_index_buffer",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX,
                triangle_count * 3,
            ),
            composite_quad_vertex_buffer: gfx.create_and_populate_buffer(
                "composite_quad_vertex_buffer",
                wgpu::BufferUsages::VERTEX,
                &[
                    CompositeVertex {
                        pos: [-1.0, 1.0],
                        uv: [0.0, 0.0],
                    },
                    CompositeVertex {
                        pos: [1.0, 1.0],
                        uv: [1.0, 0.0],
                    },
                    CompositeVertex {
                        pos: [-1.0, -1.0],
                        uv: [0.0, 1.0],
                    },
                    CompositeVertex {
                        pos: [1.0, -1.0],
                        uv: [1.0, 1.0],
                    },
                ],
            ),

            projection_params_buffer: gfx
                .create_basic_uniform_buffer::<GfxProjectionParams>("projection_params_buffer"),
            lighting_params_buffer: gfx
                .create_basic_uniform_buffer::<GfxLightingParams>("lighting_params_buffer"),
            view_params_buffer: gfx
                .create_basic_uniform_buffer::<GfxViewParams>("view_params_buffer"),
            composite_params_buffer: gfx
                .create_basic_uniform_buffer::<GfxCompositeParams>("composite_params_buffer"),

            puzzle_transform_buffer: gfx.create_buffer::<f32>(
                "puzzle_transform_buffer",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ndim as usize * ndim as usize,
            ),

            piece_transform_buffer: gfx.create_buffer::<f32>(
                "piece_transform_buffer",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ty.pieces.len() * ndim as usize * ndim as usize,
            ),

            facet_center_buffer: gfx.create_and_populate_buffer(
                "facet_shrink_center_buffer",
                wgpu::BufferUsages::STORAGE,
                facet_center_data.as_slice(),
            ),

            sticker_info_buffer: gfx.create_and_populate_buffer(
                "sticker_info_buffer",
                wgpu::BufferUsages::STORAGE,
                sticker_info_data.as_slice(),
            ),

            polygon_count: polygon_info_data.len(),
            polygon_info_buffer: gfx.create_and_populate_buffer(
                "polygon_info_buffer",
                wgpu::BufferUsages::STORAGE,
                polygon_info_data.as_slice(),
            ),
            polygon_color_buffer: gfx.create_buffer::<[f32; 4]>(
                "polygon_color_buffer",
                wgpu::BufferUsages::STORAGE,
                polygon_info_data.len(),
            ),

            vertex_count: vertex_sticker_id_data.len(),
            vertex_sticker_id_buffer: gfx.create_and_populate_buffer(
                "vertex_sticker_id_buffer",
                wgpu::BufferUsages::STORAGE,
                vertex_sticker_id_data.as_slice(),
            ),
            vertex_position_buffer: gfx.create_and_populate_buffer(
                "vertex_position_buffer",
                wgpu::BufferUsages::STORAGE,
                vertex_position_data.as_slice(),
            ),
            vertex_shrink_vector_buffer: gfx.create_and_populate_buffer(
                "vertex_shrink_vector_buffer",
                wgpu::BufferUsages::STORAGE,
                vertex_shrink_vector_data.as_slice(),
            ),
            vertex_3d_position_buffer: gfx.create_buffer::<[f32; 4]>(
                "vertex_3d_position_buffer",
                wgpu::BufferUsages::STORAGE,
                vertex_position_data.len(),
            ),

            compute_transform_points_pipeline,
            compute_polygon_colors_pipeline,
            render_polygon_ids_pipeline,
            render_composite_puzzle_pipeline,

            facet_colors_texture: CachedTexture::new_1d(
                Some("facet_colors_texture"),
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            polygon_depth_texture: CachedTexture::new_2d(
                Some("puzzle_polyon_depth_texture"),
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            polygon_ids_texture: CachedTexture::new_2d(
                Some("puzzle_polygon_ids_texture"),
                wgpu::TextureFormat::R32Sint,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            out_texture: CachedTexture::new_2d(
                Some("puzzle_out_texture"),
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
        })
    }
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

pub(crate) struct CachedTexture {
    f: Box<dyn Fn(wgpu::Extent3d) -> wgpu::TextureDescriptor<'static>>,

    size: Option<wgpu::Extent3d>,
    texture: Option<(wgpu::Texture, wgpu::TextureView)>,
}
impl CachedTexture {
    pub(super) fn from_fn(
        f: impl 'static + Fn(wgpu::Extent3d) -> wgpu::TextureDescriptor<'static>,
    ) -> Self {
        Self {
            f: Box::new(f),

            size: None,
            texture: None,
        }
    }
    pub(super) fn new(
        label: Option<&'static str>,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::from_fn(move |size| wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            usage,
        })
    }
    pub(super) fn new_2d(
        label: Option<&'static str>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new(label, wgpu::TextureDimension::D2, format, usage)
    }
    pub(super) fn new_1d(
        label: Option<&'static str>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new(label, wgpu::TextureDimension::D1, format, usage)
    }

    pub(super) fn at_size(
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
            gfx.create_texture(&(self.f)(size))
        })
    }
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

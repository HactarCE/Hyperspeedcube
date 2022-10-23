//! Rendering logic.

use itertools::Itertools;
use ndpuzzle::math::VectorRef;
use ndpuzzle::puzzle::{Facet, Piece, PuzzleInfo, PuzzleType};
use ndpuzzle::util::IterCyclicPairsExt;
use std::fmt;

#[macro_use]
mod cache;
mod shaders;
mod structs;

use super::PuzzleController;
use crate::preferences::Preferences;
use crate::GraphicsState;
use cache::CachedTexture;
use structs::*;

/// Size of compute shader work group, defined in the WGSL shader source code.
const WORK_GROUP_SIZE: u32 = 64;

#[derive(Debug, Clone, PartialEq)]
struct PuzzleRenderParams {
    target_w: u32,
    target_h: u32,

    scale: f32,
    align_h: f32,
    align_v: f32,
}

pub(super) fn draw_puzzle(
    gfx: &mut GraphicsState,
    puzzle: &mut PuzzleController,
    cache: &mut PuzzleRenderCache,
    prefs: &Preferences,
    (width, height): (u32, u32),
) -> Option<wgpu::TextureView> {
    // Avoid divide-by-zero errors.
    if width == 0 || height == 0 {
        return None;
    }

    let size = cgmath::vec2(width as f32, height as f32);

    let view_prefs = puzzle.view_prefs(prefs);

    // Calculate scale.
    let scale = {
        let min_dimen = f32::min(size.x, size.y);
        let pixel_scale = min_dimen * view_prefs.scale;
        cgmath::vec2(pixel_scale / size.x, pixel_scale / size.y)
    };

    // Write the lighting parameters.
    let lighting_params = GfxLightingParams {
        // TODO: use actual light params
        dir: [3.0_f32.sqrt(), 3.0_f32.sqrt(), 3.0_f32.sqrt()],
        ambient: 1.0 - view_prefs.light_directional,
        directional: view_prefs.light_directional,
    };
    gfx.queue.write_buffer(
        &cache.lighting_params_buffer,
        0,
        bytemuck::bytes_of(&lighting_params),
    );

    // Write the puzzle transform.
    let puzzle_transform = puzzle
        .view_transform(&view_prefs)
        .at_ndim(puzzle.ty().ndim()); // TODO: use actual view transform
    gfx.queue.write_buffer(
        &cache.puzzle_transform_buffer,
        0,
        bytemuck::cast_slice(puzzle_transform.as_slice()),
    );

    // Write the view transform.
    let view_params = GfxViewParams {
        scale: scale.into(),
        align: [view_prefs.align_h, view_prefs.align_v],
    };
    gfx.queue.write_buffer(
        &cache.view_params_buffer,
        0,
        bytemuck::bytes_of(&view_params),
    );

    // Write the projection parameters.
    let projection_params = GfxProjectionParams {
        facet_scale: 1.0 - view_prefs.facet_spacing,
        sticker_scale: 1.0 - view_prefs.sticker_spacing,
        w_factor_4d: (view_prefs.fov_4d.to_radians() / 2.0).tan(),
        w_factor_3d: (view_prefs.fov_3d.to_radians() / 2.0).tan(),
        fov_signum: view_prefs.fov_3d.signum(),
        ndim: puzzle.ty().ndim() as u32,
    };
    gfx.queue.write_buffer(
        &cache.projection_params_buffer,
        0,
        bytemuck::bytes_of(&projection_params),
    );

    // Write the piece transforms.
    let mut offset = 0;
    for i in 0..puzzle.ty().pieces.len() {
        let m = puzzle.displayed().piece_transform(Piece(i as u16));
        gfx.queue.write_buffer(
            &cache.piece_transform_buffer,
            offset,
            bytemuck::cast_slice(m.as_slice()),
        );
        offset += std::mem::size_of_val(m.as_slice()) as u64;
    }

    // Write the facet colors.
    let texture_size = extent3d(puzzle.ty().shape.facets.len() as u32, 1);
    let (facet_colors_texture, facet_colors_texture_view) =
        cache.facet_colors_texture.at_size(gfx, texture_size);
    let facet_colors_data = (0..puzzle.ty().shape.facets.len())
        .map(|i| prefs.colors[(&**puzzle.ty(), Facet(i as u8))].to_array())
        .collect_vec();
    gfx.queue.write_texture(
        wgpu::ImageCopyTextureBase {
            texture: facet_colors_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&facet_colors_data),
        wgpu::ImageDataLayout::default(),
        texture_size,
    );

    const COMPUTE: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;

    const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
    const STORAGE_READ: wgpu::BufferBindingType =
        wgpu::BufferBindingType::Storage { read_only: true };
    const STORAGE_WRITE: wgpu::BufferBindingType =
        wgpu::BufferBindingType::Storage { read_only: false };

    // Compute 3D vertex positions on the GPU.
    {
        let bind_group_0 = gfx.create_bind_group_of_buffers(
            "compute_transforms_uniforms",
            &[
                (COMPUTE, UNIFORM, &cache.u32_offset_buffer), // binding 0
                (COMPUTE, UNIFORM, &cache.projection_params_buffer), // binding 1
            ],
        );

        let bind_group_1 = gfx.create_bind_group_of_buffers(
            "compute_transforms_storage",
            &[
                (COMPUTE, STORAGE_READ, &cache.puzzle_transform_buffer), // binding 0
                (COMPUTE, STORAGE_READ, &cache.piece_transform_buffer),  // binding 1
                (COMPUTE, STORAGE_READ, &cache.facet_shrink_center_buffer), // binding 2
                (COMPUTE, STORAGE_READ, &cache.sticker_info_buffer),     // binding 3
                (COMPUTE, STORAGE_READ, &cache.sticker_shrink_center_buffer), // binding 4
                (COMPUTE, STORAGE_READ, &cache.vertex_sticker_id_buffer), // binding 5
                (COMPUTE, STORAGE_READ, &cache.vertex_position_buffer),  // binding 6
                (COMPUTE, STORAGE_WRITE, &cache.vertex_3d_position_buffer), // binding 7
            ],
        );

        dispatch_work_groups_with_offsets(
            gfx,
            "compute_3d_vertex_positions",
            &cache.transform_points_compute_pipeline,
            &[&bind_group_0, &bind_group_1],
            &cache.u32_offset_buffer,
            cache.vertex_count as u32,
        );
    }

    // Compute polygon colors on the GPU.
    {
        let bind_group_0 = gfx.create_bind_group_of_buffers(
            "polygon_colors_compute_uniforms",
            &[
                (COMPUTE, UNIFORM, &cache.u32_offset_buffer), // binding 0
                (COMPUTE, UNIFORM, &cache.lighting_params_buffer), // binding 1
            ],
        );

        let bind_group_1 = gfx.create_bind_group_of_buffers(
            "polygon_colors_compute_storage",
            &[
                (COMPUTE, STORAGE_READ, &cache.polygon_info_buffer), // binding 0
                (COMPUTE, STORAGE_WRITE, &cache.polygon_color_buffer), // binding 1
                (COMPUTE, STORAGE_READ, &cache.vertex_3d_position_buffer), // binding 2
            ],
        );

        let (_, bind_group_2) = gfx.create_texture_bind_group(
            Some("polygon_colors_texture"),
            wgpu::ShaderStages::COMPUTE,
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D1,
                multisampled: false,
            },
            &facet_colors_texture_view,
        );

        dispatch_work_groups_with_offsets(
            gfx,
            "polygon_colors_compute",
            &cache.polygon_colors_compute_pipeline,
            &[&bind_group_0, &bind_group_1, &bind_group_2],
            &cache.u32_offset_buffer,
            cache.polygon_count as u32,
        );
    }

    // Sort pieces into buckets based on their opacity level. In practice,
    // there will only ever be a handful of unique opacity levels.
    let mut pieces_by_opacity = vec![vec![]; 256];
    for piece in puzzle.visible_pieces().iter_ones().map(|i| Piece(i as u16)) {
        let opacity = (puzzle.visual_piece_state(piece).opacity(prefs) * 255.0).round() as usize;
        // This `.clamp()` is just defensive programming.
        pieces_by_opacity[opacity.clamp(0, 255)].push(piece);
    }

    // Generate indices for each opacity bucket.
    #[derive(Debug)]
    struct OpacityBucket {
        index_buffer_range: std::ops::Range<u32>,
        alpha: f32,
    }
    let mut buckets = vec![];
    let mut index_data: Vec<u32> = vec![];
    for (i, bucket) in pieces_by_opacity.iter().enumerate() {
        // Skip the completely transparent bucket.
        if i == 0 {
            continue;
        }
        // Skip empty buckets (except the last one).
        if bucket.is_empty() && i != 255 {
            continue;
        }

        let start = index_data.len() as u32;
        for &piece in bucket {
            for sticker in &puzzle.ty().info(piece).stickers {
                index_data.extend_from_slice(&cache.indices_per_sticker[sticker.0 as usize]);
            }
        }
        let end = index_data.len() as u32;
        buckets.push(OpacityBucket {
            index_buffer_range: start..end,
            alpha: i as f32 / 255.0,
        })
    }

    // Write indices to the index buffer.
    gfx.queue
        .write_buffer(&cache.index_buffer, 0, bytemuck::cast_slice(&index_data));

    // Resize textures if necessary.
    let (_polygon_depth_texture, polygon_depth_texture_view) = cache
        .polygon_depth_texture
        .at_size(gfx, extent3d(width, height));
    let (_polygon_ids_texture, polygon_ids_texture_view) = cache
        .polygon_ids_texture
        .at_size(gfx, extent3d(width, height));
    let (out_texture, out_texture_view) = cache.out_texture.at_size(gfx, extent3d(width, height));

    // Finally render the polygons, starting with the most opaque.
    buckets.reverse();
    for (i, bucket) in buckets.iter().enumerate() {
        let next_alpha = match buckets.get(i + 1) {
            Some(b) => b.alpha,
            None => 0.0,
        };
        let first_bucket = i == 0;

        let mut encoder = gfx.device.create_command_encoder(&Default::default());

        // Pass 1: Render polygon IDs.
        {
            let bind_group_0 = gfx.create_bind_group_of_buffers(
                "polygon_ids_render_bindings",
                &[
                    (
                        wgpu::ShaderStages::VERTEX,
                        UNIFORM,
                        &cache.view_params_buffer,
                    ),
                    (
                        wgpu::ShaderStages::VERTEX,
                        STORAGE_READ,
                        &cache.vertex_3d_position_buffer,
                    ),
                ],
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("polygon_ids_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: polygon_ids_texture_view,
                    resolve_target: None,
                    ops: if first_bucket {
                        wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: -1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: true,
                        }
                    } else {
                        wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        }
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: polygon_depth_texture_view,
                    depth_ops: Some(if first_bucket {
                        wgpu::Operations::default()
                    } else {
                        wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        }
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&cache.polygon_ids_render_pipeline);
            render_pass.set_vertex_buffer(0, cache.vertex_buffer.slice(..));
            render_pass.set_index_buffer(cache.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &bind_group_0, &[]);

            render_pass.draw_indexed(bucket.index_buffer_range.clone(), 0, 0..1);
        }

        // Pass 2: Composite polygon colors.
        {
            gfx.queue.write_buffer(
                &cache.composite_params_buffer,
                0,
                bytemuck::bytes_of(&GfxCompositeParams {
                    background_color: [
                        prefs.colors.background.r() as f32 / 255.0,
                        prefs.colors.background.g() as f32 / 255.0,
                        prefs.colors.background.b() as f32 / 255.0,
                    ],
                    alpha: bucket.alpha - next_alpha,
                    outline_color: [
                        prefs.outlines.default_color.r() as f32 / 255.0,
                        prefs.outlines.default_color.g() as f32 / 255.0,
                        prefs.outlines.default_color.b() as f32 / 255.0,
                    ],
                    outline_radius: view_prefs.outline_thickness.round() as u32,
                }),
            );

            let bind_group_0 = gfx.create_bind_group_of_buffers(
                "puzzle_composite_render_uniforms",
                &[(
                    wgpu::ShaderStages::FRAGMENT,
                    UNIFORM,
                    &cache.composite_params_buffer,
                )],
            );
            let bind_group_1 = gfx.create_bind_group_of_buffers(
                "puzzle_composite_render_storage",
                &[(
                    wgpu::ShaderStages::FRAGMENT,
                    STORAGE_READ,
                    &cache.polygon_color_buffer,
                )],
            );
            // TODO: just return the bind group from this, not the layout.
            let (_, bind_group_2) = gfx.create_texture_bind_group(
                Some("puzzle_composite_render_texture"),
                wgpu::ShaderStages::FRAGMENT,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Sint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                polygon_ids_texture_view,
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("puzzle_composite_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: out_texture_view,
                    resolve_target: None,
                    ops: if first_bucket {
                        wgpu::Operations::default()
                    } else {
                        wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: true,
                        }
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&cache.puzzle_composite_render_pipeline);
            render_pass.set_vertex_buffer(0, cache.composite_quad_vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &bind_group_0, &[]);
            render_pass.set_bind_group(1, &bind_group_1, &[]);
            render_pass.set_bind_group(2, &bind_group_2, &[]);

            render_pass.draw(0..4, 0..1);
        }

        gfx.queue.submit(std::iter::once(encoder.finish()));
    }

    Some(out_texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

pub(super) struct PuzzleRenderCache {
    /// For each sticker: indices into `vertex_buffer`
    indices_per_sticker: Vec<Box<[u32]>>,

    /*
     * VERTEX AND INDEX BUFFERS
     */
    /// Every pair of polygon ID and vertex ID that appears in the puzzle model.
    vertex_buffer: wgpu::Buffer,
    /// Indices into `vertex_buffer` for drawing triangles.
    index_buffer: wgpu::Buffer,
    /// Full-screen quad vertices.
    composite_quad_vertex_buffer: wgpu::Buffer,

    /*
     * SMALL UNIFORMS
     */
    /// `u32` offset.
    u32_offset_buffer: wgpu::Buffer,
    /// Projection parameters.
    projection_params_buffer: wgpu::Buffer,
    /// Lighting parameters.
    lighting_params_buffer: wgpu::Buffer,
    /// 2D view parameters.
    view_params_buffer: wgpu::Buffer,
    /// Compositing parameters.
    composite_params_buffer: wgpu::Buffer,

    /*
     * OTHER BUFFERS
     */
    /// View transform from N-dimensional space to 4D space as an Nx4 matrix.
    puzzle_transform_buffer: wgpu::Buffer,

    /// For each piece: its transform in N-dimensional space as an NxN matrix.
    ///
    /// TODO: consider changing this to an N-dimensional rotor.
    piece_transform_buffer: wgpu::Buffer,

    /// For each facet: the point to shrink towards for facet spacing.
    facet_shrink_center_buffer: wgpu::Buffer,

    /// For each sticker: the ID of its facet and the ID of its piece.
    sticker_info_buffer: wgpu::Buffer,
    /// For each sticker: the point it shrinks towards for sticker spacing.
    sticker_shrink_center_buffer: wgpu::Buffer,

    /// Number of polygons, which is the length of each "per-polygon" buffer.
    polygon_count: usize,
    /// For each polygon: the ID of its facet and the three vertex IDs for one
    /// of its triangles (used to compute its normal in 3D space).
    polygon_info_buffer: wgpu::Buffer,
    /// For each polygon: its color, which is computed from its normal and its
    /// facet's color.
    polygon_color_buffer: wgpu::Buffer,

    /// Number of vertices, which is the length of each "per-vertex" buffer.
    vertex_count: usize,
    /// For each vertex: the ID of its sticker.
    vertex_sticker_id_buffer: wgpu::Buffer,
    /// For each vertex: its position in N-dimensional space.
    vertex_position_buffer: wgpu::Buffer,
    /// For each vertex: its position in 3D space, which is recomputed whenever
    /// the view angle or puzzle geometry changes (e.g., each frame of a twist
    /// animation).
    vertex_3d_position_buffer: wgpu::Buffer,

    /*
     * PIPELINES
     */
    /// Pipeline to populate `vertex_3d_position_buffer`.
    transform_points_compute_pipeline: wgpu::ComputePipeline,
    /// Pipeline to populate `polygon_color_buffer`.
    polygon_colors_compute_pipeline: wgpu::ComputePipeline,
    /// Pipeline to render to `polygon_ids_texture`.
    polygon_ids_render_pipeline: wgpu::RenderPipeline,
    /// Pipeline to render to `out_texture`.
    puzzle_composite_render_pipeline: wgpu::RenderPipeline,

    /*
     * TEXTURES
     */
    facet_colors_texture: CachedTexture, // TODO: doesn't need to change size
    polygon_depth_texture: CachedTexture,
    polygon_ids_texture: CachedTexture,
    out_texture: CachedTexture,
}
impl fmt::Debug for PuzzleRenderCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleRenderCache").finish_non_exhaustive()
    }
}
impl PuzzleRenderCache {
    pub fn new(gfx: &mut GraphicsState, ty: &PuzzleType) -> Self {
        let ndim = ty.ndim();

        let facet_shrink_center_data = ty
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
        let mut sticker_shrink_center_data = vec![];
        let mut polygon_info_data = vec![];
        let mut vertex_sticker_id_data = vec![];
        let mut vertex_position_data = vec![];
        {
            let mut polygon_idx = 0;
            let mut degerate_polygons_count = 0;

            for (sticker_idx, sticker) in ty.stickers.iter().enumerate() {
                // For each sticker ...
                sticker_info_data.push(GfxStickerInfo {
                    piece: sticker.piece.0 as u32,
                    facet: sticker.color.0 as u32,
                });
                sticker_shrink_center_data.extend(sticker.sticker_shrink_origin.iter_ndim(ndim));

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
                    for (b, c) in (1..polygon.len()).cyclic_pairs() {
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
                for point in &sticker.points {
                    vertex_sticker_id_data.push(sticker_idx as u32);
                    vertex_position_data.extend(point.iter_ndim(ndim));
                }
            }

            if degerate_polygons_count != 0 {
                log::warn!(
                    "Removed {degerate_polygons_count} degenerate polygons from puzzle model"
                );
            }
        }

        // Load shader modules.
        let compute_transforms_shader = gfx
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/compute_transforms.wgsl"));
        let compute_colors_shader = gfx
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/compute_colors.wgsl"));
        let polygon_ids_shader = gfx
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/polygon_ids.wgsl"));
        let puzzle_composite_shader = gfx
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/puzzle_composite.wgsl"));

        // Create pipelines.
        let transform_points_compute_pipeline;
        let polygon_colors_compute_pipeline;
        let polygon_ids_render_pipeline;
        let puzzle_composite_render_pipeline;
        {
            const COMPUTE: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;

            const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
            const STORAGE_READ: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: true };
            const STORAGE_WRITE: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: false };

            transform_points_compute_pipeline = {
                let bind_group_layout_0 = gfx.create_bind_group_layout_of_buffers(
                    "transform_points_compute_pipeline_uniforms_layout",
                    &[(COMPUTE, UNIFORM), (COMPUTE, UNIFORM)],
                );
                let bind_group_layout_1 = gfx.create_bind_group_layout_of_buffers(
                    "transform_points_compute_pipeline_storage_layout",
                    &[
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_WRITE),
                    ],
                );
                gfx.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("transform_points_compute_pipeline"),
                        layout: Some(&gfx.device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("transform_points_compute_pipeline_layout"),
                                bind_group_layouts: &[&bind_group_layout_0, &bind_group_layout_1],
                                push_constant_ranges: &[],
                            },
                        )),
                        module: &compute_transforms_shader,
                        entry_point: "main",
                    })
            };

            polygon_colors_compute_pipeline = {
                let bind_group_layout_0 = gfx.create_bind_group_layout_of_buffers(
                    "polygon_colors_compute_pipeline_uniforms_layout",
                    &[(COMPUTE, UNIFORM), (COMPUTE, UNIFORM)],
                );
                let bind_group_layout_1 = gfx.create_bind_group_layout_of_buffers(
                    "polygon_colors_compute_pipeline_storage_layout",
                    &[
                        (COMPUTE, STORAGE_READ),
                        (COMPUTE, STORAGE_WRITE),
                        (COMPUTE, STORAGE_READ),
                    ],
                );
                let bind_group_layout_2 =
                    gfx.device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some(
                                "puzzle_colors_compute_\
                                 facet_colors_texture_layout",
                            ),
                            entries: &[wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: false,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D1,
                                    multisampled: false,
                                },
                                count: None,
                            }],
                        });
                gfx.device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("polygon_colors_compute_pipeline"),
                        layout: Some(&gfx.device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("polygon_colors_compute_pipeline_layout"),
                                bind_group_layouts: &[
                                    &bind_group_layout_0,
                                    &bind_group_layout_1,
                                    &bind_group_layout_2,
                                ],
                                push_constant_ranges: &[],
                            },
                        )),
                        module: &compute_colors_shader,
                        entry_point: "main",
                    })
            };

            polygon_ids_render_pipeline =
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
                        vertex: wgpu::VertexState {
                            module: &polygon_ids_shader,
                            entry_point: "vs_main",
                            buffers: &[PolygonVertex::LAYOUT],
                        },
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
                        fragment: Some(wgpu::FragmentState {
                            module: &polygon_ids_shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::TextureFormat::R32Sint.into())],
                        }),
                        multiview: None,
                    });

            puzzle_composite_render_pipeline =
                gfx.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("puzzle_composite_render_pipeline"),
                        layout: Some(&gfx.device.create_pipeline_layout(
                            &wgpu::PipelineLayoutDescriptor {
                                label: Some("puzzle_composite_render_pipeline_layout"),
                                bind_group_layouts: &[
                                    &gfx.create_bind_group_layout_of_buffers(
                                        "puzzle_composite_render_pipeline_uniform_layout",
                                        &[(wgpu::ShaderStages::FRAGMENT, UNIFORM)],
                                    ),
                                    &gfx.create_bind_group_layout_of_buffers(
                                        "puzzle_composite_render_pipeline_storage_layout",
                                        &[(wgpu::ShaderStages::FRAGMENT, STORAGE_READ)],
                                    ),
                                    &gfx.device.create_bind_group_layout(
                                        &wgpu::BindGroupLayoutDescriptor {
                                            label: Some(
                                                "puzzle_composite_render_pipeline_\
                                                 polygon_ids_texture_layout",
                                            ),
                                            entries: &[wgpu::BindGroupLayoutEntry {
                                                binding: 0,
                                                visibility: wgpu::ShaderStages::FRAGMENT,
                                                ty: wgpu::BindingType::Texture {
                                                    sample_type: wgpu::TextureSampleType::Sint,
                                                    view_dimension: wgpu::TextureViewDimension::D2,
                                                    multisampled: false,
                                                },
                                                count: None,
                                            }],
                                        },
                                    ),
                                ],
                                push_constant_ranges: &[],
                            },
                        )),
                        vertex: wgpu::VertexState {
                            module: &puzzle_composite_shader,
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
                            module: &puzzle_composite_shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: wgpu::TextureFormat::Bgra8Unorm,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent {
                                        src_factor: wgpu::BlendFactor::SrcAlpha,
                                        dst_factor: wgpu::BlendFactor::One,
                                        operation: wgpu::BlendOperation::Add,
                                    },
                                    alpha: wgpu::BlendComponent {
                                        src_factor: wgpu::BlendFactor::SrcAlpha,
                                        dst_factor: wgpu::BlendFactor::One,
                                        operation: wgpu::BlendOperation::Add,
                                    },
                                }),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        multiview: None,
                    });
        }

        Self {
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

            u32_offset_buffer: gfx.create_basic_uniform_buffer::<u32>("u32_offset_buffer"),
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

            facet_shrink_center_buffer: gfx.create_and_populate_buffer(
                "facet_shrink_center_buffer",
                wgpu::BufferUsages::STORAGE,
                facet_shrink_center_data.as_slice(),
            ),

            sticker_info_buffer: gfx.create_and_populate_buffer(
                "sticker_info_buffer",
                wgpu::BufferUsages::STORAGE,
                sticker_info_data.as_slice(),
            ),
            sticker_shrink_center_buffer: gfx.create_and_populate_buffer(
                "sticker_shrink_center_buffer",
                wgpu::BufferUsages::STORAGE,
                sticker_shrink_center_data.as_slice(),
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
            vertex_3d_position_buffer: gfx.create_buffer::<[f32; 4]>(
                "vertex_3d_position_buffer",
                wgpu::BufferUsages::STORAGE,
                vertex_position_data.len(),
            ),

            transform_points_compute_pipeline,
            polygon_colors_compute_pipeline,
            polygon_ids_render_pipeline,
            puzzle_composite_render_pipeline,

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
        }
    }
}

fn extent3d(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

fn dispatch_work_groups_with_offsets(
    gfx: &mut GraphicsState,
    label: &str,
    pipeline: &wgpu::ComputePipeline,
    bind_groups: &[&wgpu::BindGroup],
    u32_offset_buffer: &wgpu::Buffer,
    count: u32,
) {
    let mut offset = 0;
    // TODO: read max group size and use that via push constant??
    // let group_size = gfx.device.limits().max_compute_workgroup_size_x;
    let group_size = WORK_GROUP_SIZE;
    while offset < count as u32 {
        gfx.queue
            .write_buffer(u32_offset_buffer, 0, bytemuck::bytes_of(&offset));

        let mut encoder = gfx.device.create_command_encoder(&Default::default());
        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label) });
            compute_pass.set_pipeline(pipeline);
            for (i, bind_group) in bind_groups.iter().enumerate() {
                compute_pass.set_bind_group(i as u32, bind_group, &[]);
            }
            compute_pass.dispatch_workgroups(std::cmp::min(group_size, count - offset), 1, 1);
        }
        gfx.queue.submit(std::iter::once(encoder.finish()));
        offset += group_size;
    }
}

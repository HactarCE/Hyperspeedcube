//! Rendering logic.

use itertools::Itertools;
use ndpuzzle::puzzle::{Facet, Piece, PuzzleInfo};

mod cache;
mod structs;

use super::PuzzleController;
use crate::preferences::Preferences;
use crate::GraphicsState;
pub(crate) use cache::PuzzleRenderCache;
use structs::*;

/// Size of compute shader work group, defined in the WGSL shader source code.
const WORK_GROUP_SIZE: u32 = 64;

// TODO: do not rerender literally everything every frame

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
            &cache.compute_transform_points_pipeline,
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
            &cache.compute_polygon_colors_pipeline,
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
            render_pass.set_pipeline(&cache.render_polygon_ids_pipeline);
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
            render_pass.set_pipeline(&cache.render_composite_puzzle_pipeline);
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

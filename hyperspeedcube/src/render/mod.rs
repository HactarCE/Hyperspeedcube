//! Rendering logic.

use std::sync::Arc;
use std::time::Instant;

mod cache;
mod mesh;
mod shaders;
mod state;
mod structs;

use crate::app::App;
use crate::puzzle::ProjectedStickerGeometry;
use cache::{CachedDynamicBuffer, CachedUniformBuffer};
pub(crate) use state::GraphicsState;
use structs::*;

#[derive(Debug, Clone, PartialEq)]
struct PuzzleRenderParams {
    target_w: u32,
    target_h: u32,
    sample_count: u32,

    scale: f32,
    align_h: f32,
    align_v: f32,
}

pub(crate) struct PuzzleRenderCache {
    last_render_time: Instant,
    last_params: Option<PuzzleRenderParams>,
    last_puzzle_geometry: Option<Arc<Vec<ProjectedStickerGeometry>>>,

    vertex_buffer: CachedDynamicBuffer,
    index_buffer: CachedDynamicBuffer,
    uniform_buffer: CachedUniformBuffer<BasicUniform>,

    multisample_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    out_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    depth_texture: Option<(wgpu::Texture, wgpu::TextureView)>,

    basic_pipeline: Option<wgpu::RenderPipeline>,
}
impl Default for PuzzleRenderCache {
    fn default() -> Self {
        Self {
            last_render_time: Instant::now(),
            last_params: None,
            last_puzzle_geometry: None,

            vertex_buffer: CachedDynamicBuffer::new::<RgbaVertex>(
                Some("puzzle_vertex_buffer"),
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            ),
            index_buffer: CachedDynamicBuffer::new::<u32>(
                Some("puzzle_index_buffer"),
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX,
            ),
            uniform_buffer: CachedUniformBuffer::new(Some("puzzle_uniform_buffer"), 0),

            multisample_texture: None,
            out_texture: None,
            depth_texture: None,

            basic_pipeline: None,
        }
    }
}
impl PuzzleRenderCache {
    fn set_params_and_invalidate(&mut self, new_params: PuzzleRenderParams) -> bool {
        let old = match self.last_params.take() {
            Some(p) => p,
            None => {
                self.last_params = Some(new_params);
                return true;
            }
        };
        let new = new_params;
        let ret = old != new;

        if new.target_w != old.target_w || new.target_h != old.target_h {
            self.multisample_texture = None;
            self.out_texture = None;
            self.depth_texture = None;
        }

        if new.sample_count != old.sample_count {
            self.multisample_texture = None;
            self.depth_texture = None;

            self.basic_pipeline = None;
        }

        self.last_params = Some(new);

        ret
    }
}

pub(crate) fn draw_puzzle(
    app: &mut App,
    gfx: &mut GraphicsState,
    mut force_redraw: bool,
) -> Option<wgpu::TextureView> {
    let (width, height) = app.puzzle_texture_size;
    let size = cgmath::vec2(width as f32, height as f32);

    // Avoid divide-by-zero errors.
    if width <= 0 || height <= 0 {
        return None;
    }

    let puzzle = &mut app.puzzle;
    let prefs = &app.prefs;
    let view_prefs = puzzle.view_prefs(prefs);
    let cache = &mut app.render_cache;

    let now = Instant::now();
    let delta = now - cache.last_render_time;
    cache.last_render_time = now;

    // Animate puzzle geometry.
    puzzle.update_geometry(delta, &prefs.interaction);

    // Invalidate cache if parameters changed.
    force_redraw |= cache.set_params_and_invalidate(PuzzleRenderParams {
        target_w: width,
        target_h: height,
        sample_count: prefs.gfx.sample_count(),

        scale: view_prefs.scale,
        align_h: view_prefs.align_h,
        align_v: view_prefs.align_v,
    });

    // Calculate scale.
    let scale = {
        let min_dimen = f32::min(size.x, size.y);
        let pixel_scale = min_dimen * view_prefs.scale;
        cgmath::vec2(pixel_scale / size.x, pixel_scale / size.y)
    };

    // If the puzzle geometry has changed, force a redraw.
    let puzzle_geometry = puzzle.geometry(prefs);
    if let Some(old_geom) = &cache.last_puzzle_geometry {
        if !Arc::ptr_eq(&puzzle_geometry, old_geom) {
            force_redraw = true;
        }
    } else {
        force_redraw = true;
    }
    cache.last_puzzle_geometry = Some(Arc::clone(&puzzle_geometry));

    // Determine which sticker(s) are at the mouse cursor, in order from front
    // to back.
    if let Some(cursor_pos) = app.cursor_pos {
        let transformed_cursor_pos = cgmath::point2(
            (cursor_pos.x - view_prefs.align_h) / scale.x,
            (cursor_pos.y - view_prefs.align_v) / scale.y,
        );
        let hovered_stickers = puzzle_geometry.iter().rev().filter_map(move |geom| {
            Some((geom.sticker, geom.twists_for_point(transformed_cursor_pos)?))
        });
        puzzle.update_hovered_sticker(hovered_stickers);
    } else {
        puzzle.update_hovered_sticker([]);
    }

    // Animate puzzle decorations (colors, opacity, and outlines). Do this after
    // generating the puzzle geometry so that we get the most up-to-date
    // information about which sticker is hovered.
    force_redraw |= puzzle.update_decorations(delta, &prefs);

    if !force_redraw && cache.out_texture.is_some() {
        return None; // No repaint needed.
    }

    // Generate the mesh.
    let (mut verts, mut indices) = mesh::make_puzzle_mesh(puzzle, prefs, &puzzle_geometry);

    // Create "out" texture that will ultimately be returned.
    let (out_texture, out_texture_view) = cache.out_texture.get_or_insert_with(|| {
        gfx.create_texture(&wgpu::TextureDescriptor {
            label: Some("puzzle_texture"),
            size: extent3d(width, height),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gfx.config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        })
    });

    // Create depth texture.
    let (_depth_texture, depth_texture_view) = cache.depth_texture.get_or_insert_with(|| {
        gfx.create_texture(&wgpu::TextureDescriptor {
            label: Some("puzzle_texture"),
            size: extent3d(width, height),
            mip_level_count: 1,
            sample_count: prefs.gfx.sample_count(),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        })
    });

    // Create command encoder.
    let mut encoder = gfx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("puzzle_command_encoder"),
        });

    // Create render pass color attachment.
    let mut multisample_texture_view = None;
    let render_pass_color_attachment = {
        let clear_color = egui::Rgba::from(prefs.colors.background).to_tuple();
        let ops = wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: clear_color.0 as f64,
                g: clear_color.1 as f64,
                b: clear_color.2 as f64,
                a: 1.0,
            }),
            store: true,
        };

        if prefs.gfx.msaa {
            // Create multisample texture.
            let (_, msaa_tex_view) = cache.multisample_texture.get_or_insert_with(|| {
                gfx.create_texture(&wgpu::TextureDescriptor {
                    label: Some("puzzle_texture_multisample"),
                    size: extent3d(width, height),
                    mip_level_count: 1,
                    sample_count: prefs.gfx.sample_count(),
                    dimension: wgpu::TextureDimension::D2,
                    format: gfx.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                })
            });

            // Draw to the multisample texture, then resolve it to the "out"
            // texture.
            wgpu::RenderPassColorAttachment {
                view: multisample_texture_view.insert(msaa_tex_view),
                resolve_target: Some(out_texture_view),
                ops,
            }
        } else {
            // Draw directly to the "out" texture.
            wgpu::RenderPassColorAttachment {
                view: out_texture_view,
                resolve_target: None,
                ops,
            }
        }
    };

    // Begin the render pass.
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("puzzle_stickers_render_pass"),
        color_attachments: &[Some(render_pass_color_attachment)],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    });

    // Draw stickers, if there's anything to draw.
    if !indices.is_empty() {
        // Set pipeline.
        render_pass.set_pipeline(cache.basic_pipeline.get_or_insert_with(|| {
            gfx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("basic_pipeline"),
                    layout: Some(&gfx.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("basic_pipeline_layout"),
                            bind_group_layouts: &[cache.uniform_buffer.bind_group_layout(gfx)],
                            push_constant_ranges: &[],
                        },
                    )),
                    vertex: wgpu::VertexState {
                        module: gfx.shaders.basic.get(gfx),
                        entry_point: "vs_main",
                        buffers: &[RgbaVertex::LAYOUT],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Greater,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: prefs.gfx.sample_count(),
                        ..Default::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: gfx.shaders.basic.get(gfx),
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gfx.config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview: None,
                })
        }));

        // Populate vertex buffer.
        let vertex_buffer = cache.vertex_buffer.write_all(gfx, &mut verts);
        render_pass.set_vertex_buffer(0, vertex_buffer);

        // Populate index buffer.
        let index_buffer = cache.index_buffer.write_all(gfx, &mut indices);
        render_pass.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint32);

        // Populate and bind uniform.
        let uniform = BasicUniform {
            scale: scale.into(),
            align: [view_prefs.align_h, view_prefs.align_v],
        };
        cache.uniform_buffer.write(gfx, &uniform);
        render_pass.set_bind_group(0, cache.uniform_buffer.bind_group(gfx), &[]);

        // Draw stickers.
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    drop(render_pass);

    gfx.queue.submit(std::iter::once(encoder.finish()));

    Some(out_texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

fn extent3d(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

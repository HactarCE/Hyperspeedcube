//! Rendering logic.

use cgmath::Point2;
use itertools::Itertools;
use smallvec::SmallVec;

mod cache;
mod geometry;
mod shaders;
mod sort;
mod state;
mod structs;
mod util;

use crate::app::App;
use crate::puzzle::{traits::*, PuzzleType, Sticker};
use cache::{CachedDynamicBuffer, CachedUniformBuffer};
pub(crate) use state::GraphicsState;
use structs::*;
use util::{f32_total_cmp, IterCyclicPairsExt};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PuzzleRenderParams {
    pub(super) target_w: u32,
    pub(super) target_h: u32,
    pub(super) sample_count: u32,
}
impl Default for PuzzleRenderParams {
    fn default() -> Self {
        Self {
            target_w: 1,
            target_h: 1,
            sample_count: 1,
        }
    }
}

pub(crate) struct PuzzleRenderCache {
    params: PuzzleRenderParams,

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
            params: PuzzleRenderParams::default(),

            vertex_buffer: CachedDynamicBuffer::new::<RgbaVertex>(
                Some("puzzle_vertex_buffer"),
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            ),
            index_buffer: CachedDynamicBuffer::new::<u16>(
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
    fn set_params_and_invalidate(&mut self, new_params: PuzzleRenderParams) {
        let new = new_params;
        let old = self.params;
        self.params = new_params;

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
    }
}

pub(crate) struct PuzzleRenderResult {
    pub texture: wgpu::TextureView,

    puzzle_type: PuzzleType,
    sticker_geometries: Vec<ProjectedStickerGeometry>,
    scale: [f32; 2],
}
impl PuzzleRenderResult {
    pub(crate) fn get_stickers_at_pixel<'a>(
        &'a self,
        pixel: Point2<f32>,
    ) -> impl 'a + Iterator<Item = Sticker> {
        let point = cgmath::point2(pixel.x / self.scale[0], pixel.y / self.scale[1]);
        self.sticker_geometries
            .iter()
            .rev()
            .filter(move |projected_sticker_geometry| {
                projected_sticker_geometry.contains_point(point)
            })
            .map(|projected_sticker_geometry| {
                Sticker::from_id(self.puzzle_type, projected_sticker_geometry.sticker_id)
            })
            .filter_map(|result| result.ok())
    }
}

pub(crate) fn draw_puzzle(
    app: &mut App,
    gfx: &mut GraphicsState,
    width: u32,
    height: u32,
) -> PuzzleRenderResult {
    let puzzle_type = app.puzzle.ty();

    // Avoid divide-by-zero errors.
    if width == 0 || height == 0 {
        return PuzzleRenderResult {
            texture: gfx.dummy_texture_view(),

            puzzle_type,
            sticker_geometries: vec![],
            scale: [1.0, 1.0],
        };
    }

    // Invalidate cache if parameters changed.
    app.render_cache
        .set_params_and_invalidate(PuzzleRenderParams {
            target_w: width,
            target_h: height,
            sample_count: app.prefs.gfx.sample_count(),
        });

    // Generate puzzle geometry.
    let sticker_geometries = geometry::generate_puzzle_geometry(app);
    let (mut verts, mut indices) = geometry::triangulate_puzzle_geometry(&sticker_geometries);

    let prefs = &app.prefs;
    let cache = &mut app.render_cache;

    // Calculate scale.
    let scale = {
        let min_dimen = f32::min(width as f32, height as f32);
        let pixel_scale = min_dimen * prefs.view[app.puzzle.ty()].scale;
        [pixel_scale / width as f32, pixel_scale / height as f32]
    };

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
                resolve_target: Some(&out_texture_view),
                ops,
            }
        } else {
            // Draw directly to the "out" texture.
            wgpu::RenderPassColorAttachment {
                view: &out_texture_view,
                resolve_target: None,
                ops,
            }
        }
    };

    // Begin the render pass.
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("puzzle_stickers_render_pass"),
        color_attachments: &[render_pass_color_attachment],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_texture_view,
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
                        module: gfx.shaders.basic.get(&gfx),
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
                        module: gfx.shaders.basic.get(&gfx),
                        entry_point: "fs_main",
                        targets: &[wgpu::ColorTargetState {
                            format: gfx.config.format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        }],
                    }),
                    multiview: None,
                })
        }));

        // Populate vertex buffer.
        let vertex_buffer = cache.vertex_buffer.write_all(gfx, &mut verts);
        render_pass.set_vertex_buffer(0, vertex_buffer);

        // Populate index buffer.
        let index_buffer = cache.index_buffer.write_all(gfx, &mut indices);
        render_pass.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint16);

        // Populate and bind uniform.
        cache.uniform_buffer.write(gfx, &BasicUniform { scale });
        render_pass.set_bind_group(0, &cache.uniform_buffer.bind_group(gfx), &[]);

        // Draw stickers.
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    drop(render_pass);

    gfx.queue.submit(std::iter::once(encoder.finish()));

    PuzzleRenderResult {
        texture: out_texture.create_view(&wgpu::TextureViewDescriptor::default()),

        puzzle_type,
        sticker_geometries,
        scale,
    }
}

fn extent3d(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

//! Rendering logic.

use cgmath::{Deg, Matrix3, Matrix4, Vector3};
use egui::Rgba;
use std::cmp::Ordering;

mod cache;
mod shaders;
mod state;
mod uniforms;
mod verts;

use crate::app::App;
use crate::puzzle::traits::*;
use cache::CachedBuffer;
pub(crate) use state::GraphicsState;
use uniforms::PuzzleUniform;
pub(crate) use verts::RgbaVertex;

const CLIPPING_RADIUS: f32 = 2.0;

/// Matrix to convert from OpenGL clip space (where Z ranges from -1 to +1) to
/// WGPU clip space (where Z ranges from 0 to +1). Shamelessly stolen from:
/// https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/
#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

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

    stickers_vbo: CachedBuffer,
    uniform_buffer: Option<(wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup)>,
    multisample_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    out_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    basic_pipeline: Option<wgpu::RenderPipeline>,
}
impl Default for PuzzleRenderCache {
    fn default() -> Self {
        Self {
            params: PuzzleRenderParams::default(),

            stickers_vbo: CachedBuffer::new(|gfx, len| {
                gfx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("puzzle_stickers_vbo"),
                    size: (len * std::mem::size_of::<RgbaVertex>()) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                    mapped_at_creation: false,
                })
            }),
            uniform_buffer: None,
            multisample_texture: None,
            out_texture: None,
            basic_pipeline: None,
        }
    }
}
impl PuzzleRenderCache {
    fn set_params(&mut self, new_params: PuzzleRenderParams) {
        let new = new_params;
        let old = self.params;
        self.params = new_params;

        if new.target_w != old.target_w || new.target_h != old.target_h {
            self.multisample_texture = None;
            self.out_texture = None;
        }
        if new.sample_count != old.sample_count {
            self.multisample_texture = None;
            self.basic_pipeline = None;
        }
    }
}

pub(crate) fn draw_puzzle(
    app: &mut App,
    gfx: &mut GraphicsState,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    // We run into
    if width == 0 || height == 0 {
        return gfx.dummy_texture_view();
    }

    let prefs = &app.prefs;
    let puzzle = &app.puzzle;
    let view_prefs = &prefs.view[puzzle.ty()];
    let puzzle_highlight = app.puzzle_selection();
    let cache = &mut app.render_cache;
    cache.set_params(PuzzleRenderParams {
        target_w: width,
        target_h: height,
        sample_count: prefs.gfx.sample_count(),
    });

    // Invalidate cache if parameters changed.

    // Create uniform buffer and bind group.
    let (uniform_buffer, uniform_bind_group_layout, uniform_bind_group) = cache
        .uniform_buffer
        .get_or_insert_with(|| gfx.create_uniform::<PuzzleUniform>(Some("puzzle_uniform"), 0));

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

    let mut encoder = gfx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("puzzle_command_encoder"),
        });

    let clear_color = Rgba::from(prefs.colors.background).to_tuple();
    let ops = wgpu::Operations {
        load: wgpu::LoadOp::Clear(wgpu::Color {
            r: clear_color.0 as f64,
            g: clear_color.1 as f64,
            b: clear_color.2 as f64,
            a: 1.0,
        }),
        store: true,
    };

    let mut multisample_texture_view = None;
    let render_pass_color_attachment = if prefs.gfx.msaa {
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
    };

    // Begin the render pass.
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("puzzle_stickers_render_pass"),
        color_attachments: &[render_pass_color_attachment],
        depth_stencil_attachment: None,
    });
    render_pass.set_pipeline(cache.basic_pipeline.get_or_insert_with(|| {
        let module = gfx.shaders.basic(&gfx.device);

        gfx.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("basic_render_pipeline"),
                layout: Some(
                    &gfx.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("basic_render_pipeline_layout"),
                            bind_group_layouts: &[&uniform_bind_group_layout],
                            push_constant_ranges: &[],
                        }),
                ),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: "vs_main",
                    buffers: &[RgbaVertex::LAYOUT],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: prefs.gfx.sample_count(),
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module,
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

    // Populate VBO and IBO.
    let vbo_data: Vec<RgbaVertex>;
    let vbo_slice: wgpu::BufferSlice;
    {
        // Compute the model transform, which must be applied here on the CPU so
        // that we can do proper depth sorting.
        let view_transform = Matrix3::from_angle_x(Deg(view_prefs.pitch))
            * Matrix3::from_angle_y(Deg(view_prefs.yaw))
            / CLIPPING_RADIUS;
        // Compute the perspective transform, which we will apply on the GPU.
        let perspective_transform = {
            let min_dimen = f32::min(width as f32, height as f32);
            let scale = min_dimen * view_prefs.scale;

            let xx = scale / width as f32;
            let yy = scale / height as f32;

            let fov = view_prefs.fov_3d;
            let zw = (fov.to_radians() / 2.0).tan(); // `tan(fov/2)` is the factor of how much the Z coordinate affects the XY coordinates.
            let ww = 1.0 + fov.signum() * zw;

            // We've already normalized all puzzle coordinates, so the near and
            // far planes are z=-1 and z=+1 respectively. This makes
            // constructing the perspective transformation matrix relatively
            // easy.
            //
            // NOTE: This call constructs a matrix from **columns**, so it
            // appears transposed in code.
            Matrix4::from_cols(
                cgmath::vec4(xx, 0.0, 0.0, 0.0),
                cgmath::vec4(0.0, yy, 0.0, 0.0),
                cgmath::vec4(0.0, 0.0, -1.0, -zw),
                cgmath::vec4(0.0, 0.0, 0.0, ww),
            )
        };

        let mut sticker_geometry_params = StickerGeometryParams {
            sticker_spacing: view_prefs.sticker_spacing,
            face_spacing: view_prefs.face_spacing,
            fov_4d: view_prefs.fov_4d,

            view_transform,

            ..StickerGeometryParams::default()
        };
        sticker_geometry_params.color = Rgba::from(prefs.colors.outline).to_array();

        let light_direction = Matrix3::from_angle_y(Deg(view_prefs.light_yaw))
            * Matrix3::from_angle_x(Deg(-view_prefs.light_pitch)) // positive number = above
            * Vector3::unit_z();
        let light_direction: [f32; 3] = light_direction.into();

        // Write uniform data.
        gfx.queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::bytes_of(&PuzzleUniform {
                transform: (OPENGL_TO_WGPU_MATRIX * perspective_transform).into(),
                light_direction,
                min_light: 1.0 - view_prefs.light_intensity,
            }),
        );

        // Write sticker vertex data.
        {
            struct StickerVerts {
                verts: Vec<RgbaVertex>,
                avg_z: f32,
            }

            // Each sticker has a `Vec<RgbaVertex>` with all of its vertices and
            // a single `f32` containing the average Z value.
            let mut verts_by_sticker: Vec<StickerVerts> = vec![];
            for piece in puzzle.pieces() {
                sticker_geometry_params.model_transform = puzzle.model_transform_for_piece(*piece);

                for sticker in piece.stickers() {
                    let alpha = if puzzle_highlight.has_sticker(sticker) {
                        1.0
                    } else {
                        prefs.colors.hidden_opacity
                    } * prefs.colors.sticker_opacity;

                    let sticker_color = match prefs.colors.blindfold {
                        false => prefs.colors[puzzle.get_sticker_color(sticker)],
                        true => prefs.colors.blind_face,
                    };
                    sticker_geometry_params.color = Rgba::from(sticker_color).to_array();
                    sticker_geometry_params.color[3] = alpha;
                    sticker_geometry_params.color[3] = alpha;

                    if let Some(verts) = sticker.verts(sticker_geometry_params) {
                        let avg_z =
                            verts.iter().map(|v| v.pos[2]).sum::<f32>() / verts.len() as f32;
                        verts_by_sticker.push(StickerVerts { verts, avg_z });
                    }
                }
            }
            // Sort by average Z position to approximate proper transparency.
            verts_by_sticker
                .sort_by(|s1, s2| s1.avg_z.partial_cmp(&s2.avg_z).unwrap_or(Ordering::Equal));

            vbo_data = verts_by_sticker.into_iter().flat_map(|s| s.verts).collect();
        }

        // Write sticker vertices to the VBO.
        let (vbo, slice) = cache
            .stickers_vbo
            .slice(gfx, vbo_data.len() * std::mem::size_of::<RgbaVertex>());
        vbo_slice = slice;
        gfx.queue
            .write_buffer(vbo, 0, bytemuck::cast_slice(&vbo_data));
    }

    render_pass.set_vertex_buffer(0, vbo_slice);
    render_pass.set_bind_group(0, &uniform_bind_group, &[]);
    render_pass.draw(0..vbo_data.len() as u32, 0..1);

    drop(render_pass);

    gfx.queue.submit(std::iter::once(encoder.finish()));

    out_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn extent3d(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

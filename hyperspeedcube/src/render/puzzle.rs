//! Mesh rendering.
//!
//! 1. Render polygon IDs and lighting amounts to texture
//! 2. Render result in full color.

use itertools::Itertools;
use ndpuzzle::{
    math::{cga::Isometry, Matrix},
    puzzle::{Mesh, PerPiece, PerSticker},
};
use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;

use super::{structs::*, GraphicsState};

macro_rules! struct_with_constructor {
    (
        $(#[$struct_attr:meta])*
        $struct_vis:vis struct $struct_name:ident { ... }
        impl $impl_struct_name:ty {
            $fn_vis:vis fn $method_name:ident($($param_tok:tt)*) -> $ret_type:ty {
                $({ $($init_tok:tt)* })?
                $init_struct_name:ident {
                    $(
                        $(#[$field_attr:meta])*
                        $field:ident: $type:ty = $default_value:expr
                    ),* $(,)?
                }
            }
        }
    ) => {
        $(#[$struct_attr])*
        $struct_vis struct $struct_name {
            $(
                $(#[$field_attr])*
                $field: $type,
            )*
        }
        impl $impl_struct_name {
            $fn_vis fn $method_name($($param_tok)*) -> $ret_type {
                $($($init_tok)*)?
                $init_struct_name {
                    $(
                        $field: $default_value,
                    )*
                }
            }
        }
    };
}

#[derive(Debug)]
pub(crate) struct PuzzleViewRenderState {
    /// GPU static buffers.
    model: StaticPuzzleModel,
    /// GPU dynamic buffers.
    buffers: PuzzleViewDynamicBuffers,

    pub rot: Isometry,
    pub zoom: f32,
}

impl PuzzleViewRenderState {
    pub fn new(gfx: &GraphicsState, mesh: &Mesh) -> Self {
        // Increment buffer IDs so each buffer has a different label in graphics
        // debuggers.
        static ID: AtomicUsize = AtomicUsize::new(0);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        PuzzleViewRenderState {
            model: StaticPuzzleModel::new(gfx, mesh, id),
            buffers: PuzzleViewDynamicBuffers::new(gfx, mesh, id),

            rot: Isometry::ident(),
            zoom: 1.0,
        }
    }

    pub fn draw_puzzle(
        &mut self,
        gfx: &GraphicsState,
        encoder: &mut wgpu::CommandEncoder,
        (width, height): (u32, u32),
    ) -> Option<&wgpu::TextureView> {
        // Avoid divide-by-zero errors.
        if width == 0 || height == 0 {
            return None;
        }

        let size = cgmath::vec2(width as f32, height as f32);
        let tex_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        // Make the textures the right size.
        let (first_pass_texture, first_pass_texture_view) =
            self.buffers.first_pass_texture.at_size(gfx, tex_size);
        let (depth_texture, depth_texture_view) = self.buffers.depth_texture.at_size(gfx, tex_size);
        let (color_texture, color_texture_view) = self.buffers.out_texture.at_size(gfx, tex_size);

        struct ViewPrefs {
            scale: f32,
        }
        let view_prefs = ViewPrefs { scale: 1.0 };

        // Calculate scale.
        let scale = {
            let min_dimen = f32::min(size.x, size.y);
            let pixel_scale = min_dimen * view_prefs.scale;
            cgmath::vec2(pixel_scale / size.x, pixel_scale / size.y) * self.zoom
        };

        // Write the projection parameters.
        let data = GfxProjectionParams {
            facet_shrink: 0.0,
            sticker_shrink: 0.0,
            piece_explode: 0.0,

            w_factor_4d: 0.0,
            w_factor_3d: 0.0,
            fov_signum: 1.0,
        };
        gfx.queue.write_buffer(
            &self.buffers.projection_params,
            0,
            bytemuck::bytes_of(&data),
        );

        // Write the lighting parameters.
        let data = GfxLightingParams {
            dir: [1.0, 0.0, 0.0],
            ambient: 0.0,
            _padding1: [0.0; 3],
            directional: 1.0,
        };
        gfx.queue
            .write_buffer(&self.buffers.lighting_params, 0, bytemuck::bytes_of(&data));

        // Write the puzzle transform. TODO: make this only a 4xN matrix
        let puzzle_transform =
            Matrix::ident(self.model.ndim) * self.rot.euclidean_rotation_matrix();
        gfx.queue.write_buffer(
            &self.buffers.puzzle_transform,
            0,
            bytemuck::cast_slice(puzzle_transform.as_slice()),
        );

        // Write the piece transforms.
        let piece_transforms = vec![Matrix::ident(self.model.ndim); self.model.piece_count];
        let piece_transforms_data: Vec<f32> = piece_transforms
            .iter()
            .flat_map(|m| m.as_slice())
            .copied()
            .collect();
        gfx.queue.write_buffer(
            &self.buffers.piece_transforms,
            0,
            bytemuck::cast_slice(&piece_transforms_data),
        );

        // Write the facet colors.
        let mut colors = vec![[0.5, 0.5, 0.5, 1.0]];
        colors.extend(
            (0..self.model.facet_count)
                .map(|i| colorous::RAINBOW.eval_rational(i, self.model.facet_count))
                .map(|c| c.into_array().map(|x| x as f32 / 255.0))
                .map(|[r, g, b]| [r, g, b, 1.0]),
        );
        gfx.queue
            .write_buffer(&self.buffers.facet_colors, 0, bytemuck::cast_slice(&colors));

        // Write the view parameters.
        let data = GfxViewParams {
            scale: [scale.x, scale.y],
            align: [0.0, 0.0],
        };
        gfx.queue
            .write_buffer(&self.buffers.view_params, 0, bytemuck::bytes_of(&data));

        // Compute 3D vertex positions on the GPU.
        {
            let bind_groups = gfx
                .pipelines
                .compute_transform_points_bind_groups
                .bind_groups(
                    &gfx.device,
                    &[
                        &[
                            self.buffers.projection_params.as_entire_binding(),
                            self.buffers.lighting_params.as_entire_binding(),
                            self.buffers.puzzle_transform.as_entire_binding(),
                            self.buffers.piece_transforms.as_entire_binding(),
                        ],
                        &[
                            self.model.vertex_positions.as_entire_binding(),
                            self.model.u_tangents.as_entire_binding(),
                            self.model.v_tangents.as_entire_binding(),
                            self.model.sticker_shrink_vectors.as_entire_binding(),
                            self.model.facet_ids.as_entire_binding(),
                            self.model.piece_ids.as_entire_binding(),
                        ],
                        &[
                            self.model.facet_centroids.as_entire_binding(),
                            self.model.piece_centroids.as_entire_binding(),
                        ],
                        &[
                            self.buffers.vertex_3d_positions.as_entire_binding(),
                            self.buffers.vertex_lightings.as_entire_binding(),
                        ],
                    ],
                );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_3d_vertex_positions"),
            });
            compute_pass.set_pipeline(gfx.pipelines.compute_transform_points(self.model.ndim)?);
            compute_pass.set_bind_group(0, &bind_groups[0], &[]);
            compute_pass.set_bind_group(1, &bind_groups[1], &[]);
            compute_pass.set_bind_group(2, &bind_groups[2], &[]);
            compute_pass.set_bind_group(3, &bind_groups[3], &[]);

            dispatch_work_groups_with_offsets(
                &mut compute_pass,
                self.model.vertex_count as u32,
                &gfx.device.limits(),
            );
        }

        // Render first pass.
        {
            let bind_groups = gfx.pipelines.render_polygon_ids_bind_groups.bind_groups(
                &gfx.device,
                &[&[self.buffers.view_params.as_entire_binding()]],
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_polygon_ids"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: first_pass_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&gfx.pipelines.render_polygon_ids);
            render_pass.set_bind_group(0, &bind_groups[0], &[]);
            render_pass.set_vertex_buffer(0, self.buffers.vertex_3d_positions.slice(..));
            render_pass.set_vertex_buffer(1, self.buffers.vertex_lightings.slice(..));
            render_pass.set_vertex_buffer(2, self.model.facet_ids.slice(..));
            render_pass.set_vertex_buffer(3, self.model.polygon_ids.slice(..));
            render_pass.set_index_buffer(self.model.triangles.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.model.triangle_count as u32 * 3, 0, 0..1);
            drop(render_pass);
        }

        // Write the composite parameters. TODO: use push constants
        let data = GfxCompositeParams {
            alpha: 1.0,
            outline_radius: 1,
        };
        gfx.queue
            .write_buffer(&self.buffers.composite_params, 0, bytemuck::bytes_of(&data));

        // Write the special colors.
        let data = GfxSpecialColors {
            background: [0.6, 0.7, 0.8],
            _padding1: 0,
            outline: [0.0, 0.0, 0.0],
            _padding2: 0,
        };
        gfx.queue
            .write_buffer(&self.buffers.special_colors, 0, bytemuck::bytes_of(&data));

        // Render second pass.
        {
            let bind_groups = gfx
                .pipelines
                .render_composite_puzzle_bind_groups
                .bind_groups(
                    &gfx.device,
                    &[
                        &[
                            self.buffers.composite_params.as_entire_binding(),
                            self.buffers.special_colors.as_entire_binding(),
                        ],
                        &[wgpu::BindingResource::TextureView(first_pass_texture_view)],
                        &[self.buffers.facet_colors.as_entire_binding()],
                    ],
                );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_composite_puzzle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&gfx.pipelines.render_composite_puzzle);
            render_pass.set_bind_group(0, &bind_groups[0], &[]);
            render_pass.set_bind_group(1, &bind_groups[1], &[]);
            render_pass.set_bind_group(2, &bind_groups[2], &[]);
            render_pass.set_vertex_buffer(0, self.buffers.composite_vertices.slice(..));
            render_pass.draw(0..4, 0..1);
            drop(render_pass);
        }

        Some(color_texture_view)
    }
}

struct_with_constructor! {
    /// Static buffers for a puzzle type.
    struct StaticPuzzleModel { ... }
    impl StaticPuzzleModel {
        fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize) -> Self {
            {
                macro_rules! buffer {
                    ($mesh:ident.$name:ident, $usage:expr) => {{
                        let label = format!("puzzle{}_{}", id, stringify!($name));
                        gfx.create_buffer_init(label, &$mesh.$name, $usage)
                    }};
                    ($name:ident, $usage:expr) => {{
                        let label = format!("puzzle{}_{}", id, stringify!($name));
                        gfx.create_buffer_init(label, &$name, $usage)
                    }};
                }

                const COPY_SRC: wgpu::BufferUsages = wgpu::BufferUsages::COPY_SRC;
                const INDEX: wgpu::BufferUsages = wgpu::BufferUsages::INDEX;
                const VERTEX: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX;
                const STORAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE;

                // Convert to i32 because WGSL doesn't support 16-bit integers yet.
                let facet_ids = mesh.facet_ids.iter().map(|&i| i.0 as u32).collect_vec();
                let piece_ids = mesh.facet_ids.iter().map(|&i| i.0 as u32).collect_vec();
            }

            StaticPuzzleModel {
                ndim: u8 = mesh.ndim(),
                piece_count: usize = mesh.piece_count(),
                facet_count: usize = mesh.facet_count(),
                vertex_count: usize = mesh.vertex_count(),
                triangle_count: usize = mesh.triangle_count(),

                /*
                 * PER-VERTEX STORAGE BUFFERS
                 */
                /// Vertex locations in N-dimensional space.
                vertex_positions:       wgpu::Buffer = buffer!(mesh.vertex_positions,       STORAGE),
                /// First tangent vectors.
                u_tangents:             wgpu::Buffer = buffer!(mesh.u_tangents,             STORAGE),
                /// Second tangent vectors.
                v_tangents:             wgpu::Buffer = buffer!(mesh.v_tangents,             STORAGE),
                /// Vector along which to apply sticker shrink for each vertex.
                sticker_shrink_vectors: wgpu::Buffer = buffer!(mesh.sticker_shrink_vectors, STORAGE),
                /// Facet ID for each vertex.
                facet_ids:              wgpu::Buffer = buffer!(facet_ids,          VERTEX | STORAGE),
                /// Piece ID for each vertex.
                piece_ids:              wgpu::Buffer = buffer!(piece_ids,                   STORAGE),
                /// Polygon ID for each vertex.
                polygon_ids:            wgpu::Buffer = buffer!(mesh.polygon_ids,             VERTEX),

                /*
                 * OTHER STORAGE BUFFERS
                 */
                /// Centroid for each piece.
                piece_centroids:        wgpu::Buffer = buffer!(mesh.piece_centroids,        STORAGE),
                /// Centroid for each facet.
                facet_centroids:        wgpu::Buffer = buffer!(mesh.facet_centroids,        STORAGE),
                /// Vertex IDs for each triangle in the whole mesh.
                triangles:              wgpu::Buffer = buffer!(mesh.triangles,     COPY_SRC | INDEX), // TODO: this isn't index; sorted is

                sticker_index_ranges: PerSticker<Range<u32>> = mesh.sticker_index_ranges.clone(),
                piece_internals_index_ranges: PerPiece<Range<u32>> = mesh.piece_internals_index_ranges.clone(),
            }
        }
    }
}
impl fmt::Debug for StaticPuzzleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticPuzzleModel")
            .field("ndim", &self.ndim)
            .field("piece_count", &self.piece_count)
            .field("vertex_count", &self.vertex_count)
            .field("triangle_count", &self.triangle_count)
            .field("sticker_index_ranges", &self.sticker_index_ranges)
            .field(
                "piece_internals_index_ranges",
                &self.piece_internals_index_ranges,
            )
            .finish_non_exhaustive()
    }
}

struct_with_constructor! {
    /// Dynamic buffers and textures for a puzzle view.
    struct PuzzleViewDynamicBuffers { ... }
    impl PuzzleViewDynamicBuffers {
        fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize) -> Self {
            {
                let ndim = mesh.ndim();
                let label = |s| format!("puzzle{id}_{s}");
            }

            PuzzleViewDynamicBuffers {
                /*
                 * VIEW PARAMETERS AND TRANSFORMS
                 */
                /// Projection parameters uniform.
                projection_params: wgpu::Buffer = gfx.create_buffer::<GfxProjectionParams>(
                    label("projection_params"),
                    1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),
                /// Lighting parameters uniform.
                lighting_params: wgpu::Buffer = gfx.create_buffer::<GfxLightingParams>(
                    label("lighting_params"),
                    1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),
                /// NxN transformation matrix for the whole puzzle.
                puzzle_transform: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("puzzle_transform"),
                    ndim as usize * ndim as usize,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// NxN transformation matrix for each piece.
                piece_transforms: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("piece_transforms"),
                    ndim as usize * ndim as usize * mesh.piece_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),

                view_params: wgpu::Buffer = gfx.create_buffer::<GfxViewParams>(
                    label("view_params"),
                    1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),
                composite_params: wgpu::Buffer = gfx.create_buffer::<GfxCompositeParams>(
                    label("composite_params"),
                    1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),

                /*
                 * VERTEX BUFFERS
                 */
                /// 3D position for each vertex.
                vertex_3d_positions: wgpu::Buffer = gfx.create_buffer::<[f32; 4]>(
                    label("vertex_3d_positions"),
                    mesh.vertex_count(),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                ),
                /// Lighting amount for each vertex.
                vertex_lightings: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("vertex_lightings"),
                    mesh.vertex_count(),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                ),
                /// Composite vertices. TODO: globally cache this
                composite_vertices: wgpu::Buffer = gfx.create_buffer_init::<CompositeVertex>(
                    label("composite_vertices"),
                    &CompositeVertex::SQUARE,
                    wgpu::BufferUsages::VERTEX,
                ),

                /*
                 * INDEX BUFFERS
                 */
                /// Indices of triangles to draw, sorted by opacity.
                sorted_triangles: wgpu::Buffer = gfx.create_buffer::<[i32; 3]>(
                    label("sorted_triangles"),
                    mesh.triangle_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX,
                ),

                /*
                 * COLORS
                 */
                /// Special colors.
                special_colors: wgpu::Buffer = gfx.create_buffer::<GfxSpecialColors>(
                    label("special_colors"),
                    1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),
                /// Color for each facet.
                facet_colors: wgpu::Buffer = gfx.create_buffer::<[f32; 4]>(
                    label("facet_colors"),
                    mesh.facet_count() + 1,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),

                /*
                 * TEXTURES
                 */
                /// First pass texture, which includes lighting, facet ID, and
                /// polygon ID for each pixel.
                first_pass_texture: CachedTexture = CachedTexture::new_2d(
                    label("first_pass_texture"),
                    wgpu::TextureFormat::Rg32Sint,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Depth texture for use in the first pass.
                depth_texture: CachedTexture = CachedTexture::new_2d(
                    label("depth_texture"),
                    wgpu::TextureFormat::Depth24PlusStencil8,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Output color texture.
                out_texture: CachedTexture = CachedTexture::new_2d(
                    label("color_texture"),
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
            }
        }
    }
}

impl fmt::Debug for PuzzleViewDynamicBuffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleViewDynamicBuffers")
            .finish_non_exhaustive()
    }
}

pub(crate) struct CachedTexture {
    label: String,
    dimension: wgpu::TextureDimension,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,

    size: Option<wgpu::Extent3d>,
    texture: Option<(wgpu::Texture, wgpu::TextureView)>,
}
impl CachedTexture {
    pub(super) fn new(
        label: String,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        CachedTexture {
            label,
            dimension,
            format,
            usage,

            size: None,
            texture: None,
        }
    }
    pub(super) fn new_2d(
        label: String,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new(label, wgpu::TextureDimension::D2, format, usage)
    }
    pub(super) fn new_1d(
        label: String,
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
            gfx.create_texture(wgpu::TextureDescriptor {
                label: Some(&self.label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: self.dimension,
                format: self.format,
                usage: self.usage,
                view_formats: &[],
            })
        })
    }
}

fn dispatch_work_groups_with_offsets(
    compute_pass: &mut wgpu::ComputePass,
    count: u32,
    limits: &wgpu::Limits,
) {
    let group_size = limits.max_compute_workgroup_size_x;
    let mut offset: u32 = 0;
    while offset < count {
        compute_pass.set_push_constants(0, bytemuck::bytes_of(&offset));
        compute_pass.dispatch_workgroups(std::cmp::min(group_size, count - offset), 1, 1);
        offset += group_size;
    }
}

//! Mesh rendering.
//!
//! 1. Render polygon IDs and lighting amounts to texture
//! 2. Render result in full color.

use itertools::Itertools;
use ndpuzzle::{
    math::{cga::Isometry, Float, Matrix, ToConformalPoint, VectorRef},
    puzzle::{Mesh, PerPiece, PerSticker},
};
use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use super::{structs::*, CachedTexture, GraphicsState};

// Increment buffer IDs so each buffer has a different label in graphics
// debuggers.
fn next_buffer_id() -> usize {
    static ID: AtomicUsize = AtomicUsize::new(0);
    ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewParams {
    pub width: u32,
    pub height: u32,

    pub rot: Isometry,
    pub zoom: f32,

    pub facet_shrink: f32,
    pub sticker_shrink: f32,
    pub piece_explode: f32,

    pub fov_3d: f32,
    pub fov_4d: f32,
}
impl Default for ViewParams {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,

            rot: Isometry::ident(),
            zoom: 0.3,

            facet_shrink: 0.0,
            sticker_shrink: 0.0,
            piece_explode: 0.25,

            fov_3d: 0.0,
            fov_4d: 30.0,
        }
    }
}
impl ViewParams {
    /// Returns the X and Y scale factors to use in the view matrix. Returns
    /// `Err` if either the width or height is smaller than one pixel.
    pub fn xy_scale(&self) -> Result<cgmath::Vector2<f32>, ()> {
        if self.width == 0 || self.height == 0 {
            return Err(());
        }
        let w = self.width as f32;
        let h = self.height as f32;

        let min_dimen = f32::min(w as f32, h as f32);
        Ok(cgmath::vec2(min_dimen / w, min_dimen / h) * self.zoom)
    }

    pub fn w_factor_4d(&self) -> f32 {
        (self.fov_4d.to_radians() * 0.5).tan()
    }
    pub fn w_factor_3d(&self) -> f32 {
        (self.fov_3d.to_radians() * 0.5).tan()
    }
    pub fn project_point(&self, p: impl ToConformalPoint) -> Option<cgmath::Point2<f32>> {
        let mut p = self.rot.transform_point(p).to_finite().ok()?;

        // Apply 4D perspective transformation.
        let w = p.get(3) as f32;
        p.resize(3);
        let mut p = p / (1.0 + w * self.w_factor_4d()) as Float;

        // Apply 3D perspective transformation.
        let z = p.get(2) as f32;
        p.resize(2);
        let p = p / (1.0 + (self.fov_3d.signum() - z) * self.w_factor_3d()) as Float;

        // Apply scaling.
        let xy_scale = self.xy_scale().ok()?;
        let x = p[0] as f32 * xy_scale.x;
        let y = p[1] as f32 * xy_scale.y;

        Some(cgmath::point2(x, y))
    }

    /// Returns the projection parameters to send to the GPU.
    fn gfx_projection_params(&self) -> GfxProjectionParams {
        GfxProjectionParams {
            facet_shrink: self.facet_shrink,
            sticker_shrink: self.sticker_shrink,
            piece_explode: self.piece_explode,

            w_factor_4d: self.w_factor_4d(),
            w_factor_3d: self.w_factor_3d(),
            fov_signum: self.fov_3d.signum(),
        }
    }
}

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
pub(crate) struct PuzzleRenderer {
    /// Static model data, which does not change and so can be shared among all
    /// renderers of the same type of puzzle (hence `Arc`).
    model: Arc<StaticPuzzleModel>,
    /// GPU dynamic buffers, whose contents do change.
    buffers: DynamicPuzzleBuffers,
}

impl PuzzleRenderer {
    pub fn new(gfx: &GraphicsState, mesh: &Mesh) -> Self {
        let id = next_buffer_id();
        PuzzleRenderer {
            model: Arc::new(StaticPuzzleModel::new(gfx, mesh, id)),
            buffers: DynamicPuzzleBuffers::new(gfx, mesh, id),
        }
    }

    pub fn clone(&self, gfx: &GraphicsState) -> Self {
        Self {
            model: Arc::clone(&self.model),
            buffers: self.buffers.clone(gfx),
        }
    }

    pub fn draw_puzzle_single_pass(
        &mut self,
        gfx: &GraphicsState,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &ViewParams,
    ) -> Result<&wgpu::TextureView, ()> {
        self.init_buffers(gfx, view_params)?;

        let tex_size = wgpu::Extent3d {
            width: view_params.width,
            height: view_params.height,
            depth_or_array_layers: 1,
        };

        // Make the textures the right size.
        let (depth_texture, depth_texture_view) = self.buffers.depth_texture.at_size(gfx, tex_size);
        let (color_texture, color_texture_view) = self.buffers.out_texture.at_size(gfx, tex_size);

        // Render in a single pass.
        {
            let bind_groups = gfx.pipelines.render_single_pass_bind_groups.bind_groups(
                &gfx.device,
                &[
                    &[
                        self.model.vertex_positions.as_entire_binding(),
                        self.model.u_tangents.as_entire_binding(),
                        self.model.v_tangents.as_entire_binding(),
                        self.model.sticker_shrink_vectors.as_entire_binding(),
                    ],
                    &[
                        self.model.facet_centroids.as_entire_binding(),
                        self.model.piece_centroids.as_entire_binding(),
                        self.buffers.facet_colors.as_entire_binding(),
                    ],
                    &[
                        self.buffers.puzzle_transform.as_entire_binding(),
                        self.buffers.piece_transforms.as_entire_binding(),
                        self.buffers.projection_params.as_entire_binding(),
                        self.buffers.lighting_params.as_entire_binding(),
                        self.buffers.view_params.as_entire_binding(),
                    ],
                ],
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_puzzle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.6,
                            g: 0.7,
                            b: 0.8,
                            a: 1.0,
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

            render_pass.set_pipeline(
                gfx.pipelines
                    .render_single_pass(self.model.ndim)
                    .ok_or(())?,
            );
            for (index, bind_group) in &bind_groups {
                render_pass.set_bind_group(*index, bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.model.facet_ids.slice(..));
            render_pass.set_vertex_buffer(1, self.model.piece_ids.slice(..));
            render_pass.set_index_buffer(self.model.triangles.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.model.triangle_count as u32 * 3, 0, 0..1);
            drop(render_pass);
        }

        Ok(color_texture_view)
    }

    pub fn draw_puzzle(
        &mut self,
        gfx: &GraphicsState,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &ViewParams,
    ) -> Result<&wgpu::TextureView, ()> {
        self.init_buffers(gfx, view_params)?;

        let tex_size = wgpu::Extent3d {
            width: view_params.width,
            height: view_params.height,
            depth_or_array_layers: 1,
        };

        // Make the textures the right size.
        let (first_pass_texture, first_pass_texture_view) =
            self.buffers.first_pass_texture.at_size(gfx, tex_size);
        let (depth_texture, depth_texture_view) = self.buffers.depth_texture.at_size(gfx, tex_size);
        let (color_texture, color_texture_view) = self.buffers.out_texture.at_size(gfx, tex_size);

        // Compute 3D vertex positions on the GPU.
        {
            let bind_groups = gfx
                .pipelines
                .compute_transform_points_bind_groups
                .bind_groups(
                    &gfx.device,
                    &[
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
                            self.buffers.vertex_3d_positions.as_entire_binding(),
                            self.buffers.vertex_lightings.as_entire_binding(),
                        ],
                        &[
                            self.buffers.puzzle_transform.as_entire_binding(),
                            self.buffers.piece_transforms.as_entire_binding(),
                            self.buffers.projection_params.as_entire_binding(),
                            self.buffers.lighting_params.as_entire_binding(),
                        ],
                    ],
                );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_3d_vertex_positions"),
            });
            compute_pass.set_pipeline(
                gfx.pipelines
                    .compute_transform_points(self.model.ndim)
                    .ok_or(())?,
            );
            for (index, bind_group) in &bind_groups {
                compute_pass.set_bind_group(*index, bind_group, &[]);
            }

            dispatch_work_groups(&mut compute_pass, self.model.vertex_count as u32);
        }

        // Render first pass.
        {
            let bind_groups = gfx.pipelines.render_polygon_ids_bind_groups.bind_groups(
                &gfx.device,
                &[&[], &[], &[self.buffers.view_params.as_entire_binding()]],
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
            for (index, bind_group) in &bind_groups {
                render_pass.set_bind_group(*index, bind_group, &[]);
            }
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
                        &[],
                        &[self.buffers.facet_colors.as_entire_binding()],
                        &[wgpu::BindingResource::TextureView(first_pass_texture_view)],
                        &[
                            self.buffers.composite_params.as_entire_binding(),
                            self.buffers.special_colors.as_entire_binding(),
                        ],
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
            for (index, bind_group) in &bind_groups {
                render_pass.set_bind_group(*index, bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.buffers.composite_vertices.slice(..));
            render_pass.draw(0..4, 0..1);
            drop(render_pass);
        }

        Ok(color_texture_view)
    }

    fn init_buffers(&mut self, gfx: &GraphicsState, view_params: &ViewParams) -> Result<(), ()> {
        let scale = view_params.xy_scale()?;

        // Write the projection parameters.
        let data = view_params.gfx_projection_params();
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
            Matrix::ident(self.model.ndim) * view_params.rot.euclidean_rotation_matrix();
        let puzzle_transform = puzzle_transform
            .as_slice()
            .iter()
            .map(|&x| x as f32)
            .collect_vec();
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
            .map(|&x| x as f32)
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

        Ok(())
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
                let piece_ids = mesh.piece_ids.iter().map(|&i| i.0 as u32).collect_vec();
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
                piece_ids:              wgpu::Buffer = buffer!(piece_ids,          VERTEX | STORAGE),
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
    struct DynamicPuzzleBuffers { ... }
    impl DynamicPuzzleBuffers {
        fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize) -> Self {
            {
                let ndim = mesh.ndim();
                let label = |s| format!("puzzle{id}_{s}");
            }

            DynamicPuzzleBuffers {
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

impl fmt::Debug for DynamicPuzzleBuffers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleViewDynamicBuffers")
            .finish_non_exhaustive()
    }
}

impl DynamicPuzzleBuffers {
    fn clone(&self, gfx: &GraphicsState) -> Self {
        let id = next_buffer_id();

        macro_rules! clone_buffer {
            ($gfx:ident, $id:ident, $self:ident.$field:ident) => {
                gfx.create_buffer::<u8>(
                    format!("puzzle{}_{}", $id, stringify!($field)),
                    $self.$field.size() as usize,
                    $self.$field.usage(),
                )
            };
        }
        macro_rules! clone_texture {
            ($gfx:ident, $id:ident, $self:ident.$field:ident) => {
                $self
                    .$field
                    .clone(format!("puzzle{}_{}", $id, stringify!($field)))
            };
        }

        Self {
            projection_params: clone_buffer!(gfx, id, self.projection_params),
            lighting_params: clone_buffer!(gfx, id, self.lighting_params),
            puzzle_transform: clone_buffer!(gfx, id, self.puzzle_transform),
            piece_transforms: clone_buffer!(gfx, id, self.piece_transforms),
            view_params: clone_buffer!(gfx, id, self.view_params),
            composite_params: clone_buffer!(gfx, id, self.composite_params),
            vertex_3d_positions: clone_buffer!(gfx, id, self.vertex_3d_positions),
            vertex_lightings: clone_buffer!(gfx, id, self.vertex_lightings),
            composite_vertices: clone_buffer!(gfx, id, self.composite_vertices),
            sorted_triangles: clone_buffer!(gfx, id, self.sorted_triangles),
            special_colors: clone_buffer!(gfx, id, self.special_colors),
            facet_colors: clone_buffer!(gfx, id, self.facet_colors),

            first_pass_texture: clone_texture!(gfx, id, self.first_pass_texture),
            depth_texture: clone_texture!(gfx, id, self.depth_texture),
            out_texture: clone_texture!(gfx, id, self.out_texture),
        }
    }
}

fn dispatch_work_groups(compute_pass: &mut wgpu::ComputePass, count: u32) {
    const WORKGROUP_SIZE: u32 = 256;
    // Divide, rounding up
    let group_count = (count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
    compute_pass.dispatch_workgroups(group_count, 1, 1);
}

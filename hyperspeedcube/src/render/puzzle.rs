//! Mesh rendering.
//!
//! 1. Render polygon IDs and lighting amounts to texture
//! 2. Render result in full color.

use itertools::Itertools;
use ndpuzzle::{
    math::{cga::Isometry, Matrix, Sign, Vector, VectorRef},
    puzzle::{Mesh, PerPiece, PerSticker},
    vector,
};
use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;

use crate::render::structs::{BasicVertex, ViewParams};

use super::{GfxProjectionParams, GraphicsState};

pub(crate) struct PuzzleViewRenderState {
    ndim: u8,

    piece_count: usize,

    model: PuzzleModel,

    pub rot: Isometry,

    /// Projection parameters uniform buffer.
    projection_params_buffer: wgpu::Buffer,

    /// Puzzle transform buffer.
    puzzle_transform: wgpu::Buffer,

    vertex_3d_positions: wgpu::Buffer,
    sorted_triangles: wgpu::Buffer,

    facet_colors: wgpu::Buffer,

    /// Output color texture.
    color_texture: CachedTexture,
    depth_texture: CachedTexture,
}

impl fmt::Debug for PuzzleViewRenderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleViewRenderState")
            .field("ndim", &self.ndim)
            .field("piece_count", &self.piece_count)
            .finish_non_exhaustive()
    }
}

impl PuzzleViewRenderState {
    pub fn new(gfx: &GraphicsState, mesh: &Mesh) -> Self {
        // Increment buffer IDs so each buffer has a different label in graphics
        // debuggers.
        static ID: AtomicUsize = AtomicUsize::new(0);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        PuzzleViewRenderState {
            ndim: mesh.ndim(),

            piece_count: mesh.piece_count(),

            model: PuzzleModel::new(gfx, mesh, id),

            rot: Isometry::ident(),

            projection_params_buffer: gfx.create_uniform_buffer::<GfxProjectionParams>(format!(
                "puzzle{id}_projection_params",
            )),

            puzzle_transform: gfx.create_buffer::<[f32; 3]>(
                format!("puzzle{id}_transform"),
                mesh.ndim() as usize,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            ),

            vertex_3d_positions: gfx.create_buffer::<[f32; 4]>(
                format!("puzzle{id}_projected_points"),
                mesh.vertex_count(),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            ),
            sorted_triangles: gfx.create_buffer::<[u32; 3]>(
                format!("puzzle{id}_triangles_sorted"),
                mesh.triangles.len(),
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX,
            ),

            facet_colors: gfx.create_buffer::<[f32; 3]>(
                format!("puzzle{id}_facet_colors"),
                3,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            ),

            color_texture: CachedTexture::new_2d(
                format!("puzzle{id}_color_texture"),
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
            depth_texture: CachedTexture::new_2d(
                format!("puzzle{id}_depth_texture"),
                wgpu::TextureFormat::Depth24Plus,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
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
            dbg!("nope");
            return None;
        }

        let size = cgmath::vec2(width as f32, height as f32);
        let tex_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        // Make the texture the right size.
        let (color_texture, color_texture_view) = self.color_texture.at_size(gfx, tex_size);

        struct ViewPrefs {
            scale: f32,
        }
        let view_prefs = ViewPrefs { scale: 1.0 };

        // Calculate scale.
        let scale = {
            let min_dimen = f32::min(size.x, size.y);
            let pixel_scale = min_dimen * view_prefs.scale;
            cgmath::vec2(pixel_scale / size.x, pixel_scale / size.y)
        };

        // Write the puzzle transform. TODO: make this only a 4xN matrix
        let puzzle_transform = Matrix::ident(self.ndim);
        gfx.queue.write_buffer(
            &self.puzzle_transform,
            0,
            bytemuck::cast_slice(puzzle_transform.as_slice()),
        );

        // Write the piece transforms.
        let piece_transforms = vec![Matrix::ident(self.ndim); self.piece_count];
        let piece_transforms_data: Vec<f32> = piece_transforms
            .iter()
            .flat_map(|m| m.as_slice())
            .copied()
            .collect();
        gfx.queue.write_buffer(
            &self.model.piece_transforms,
            0,
            bytemuck::cast_slice(&piece_transforms_data),
        );

        // Write the facet colors.
        gfx.queue.write_buffer(
            &self.facet_colors,
            0,
            bytemuck::cast_slice(&[[1.0, 0.3, 0.6], [0.0, 1.0, 0.3], [0.0, 0.6, 1.0_f32]]),
        );

        // Compute 3D vertex positions on the GPU.
        {
            const COMPUTE: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;

            const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
            const STORAGE_READ: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: true };
            const STORAGE_WRITE: wgpu::BufferBindingType =
                wgpu::BufferBindingType::Storage { read_only: false };

            // let bind_group_0 = || {
            //     gfx.create_bind_group_of_buffers(
            //         "compute_transforms_uniforms",
            //         &[
            //             (COMPUTE, UNIFORM, &self.projection_params_buffer), // binding 0
            //         ],
            //     )
            // };

            // let bind_group_1 = |i| {
            //     let o4 = i * std::mem::size_of::<u32>() as u64;
            //     let o56 = i * std::mem::size_of::<f32>() as u64 * self.ndim as u64;
            //     let o7 = i * std::mem::size_of::<[f32; 4]>() as u64;
            //     gfx.create_bind_group_of_buffers_with_offsets(
            //         "compute_transforms_storage",
            //         &[
            //             (COMPUTE, STORAGE_READ, &self.puzzle_transform, 0), // binding 0
            //             (COMPUTE, STORAGE_READ, &self.model.piece_transforms, 0), // binding 1
            //             // (COMPUTE, STORAGE_READ, &self.facet_center, 0),     // binding 2
            //             // (COMPUTE, STORAGE_READ, &self.sticker_info, 0),     // binding 3
            //             // (COMPUTE, STORAGE_READ, &self.vertex_sticker_id, o4), // binding 4
            //             (
            //                 COMPUTE,
            //                 STORAGE_READ,
            //                 &self.model.static_buffers.vertex_positions,
            //                 o56,
            //             ), // binding 5
            //             // (COMPUTE, STORAGE_READ, &self.vertex_shrink_vector, o56), // binding 6
            //             (COMPUTE, STORAGE_WRITE, &self.vertex_3d_positions, o7), // binding 7
            //         ],
            //     )
            // };

            // dispatch_work_groups_with_offsets(
            //     encoder,
            //     "compute_3d_vertex_positions",
            //     &cache.compute_transform_points_pipeline,
            //     |i| vec![bind_group_0(), bind_group_1(i)],
            //     cache.vertex_count as u32,
            //     &gfx.device.limits(),
            // );
        }

        let colors = [
            [1.0, 0.0, 0.0],
            [1.0, 0.3, 0.0],
            [0.8, 0.8, 0.8],
            [0.8, 0.8, 0.0],
            [0.0, 0.7, 0.2],
            [0.0, 0.0, 0.7],
        ];
        let vertex_data =
            itertools::iproduct!([[0, 1, 2], [1, 2, 0], [2, 0, 1]], [Sign::Pos, Sign::Neg])
                .zip(colors)
                .flat_map(|((axes, sign), color)| {
                    let [a, u, v] = axes.map(Vector::unit);
                    let a = a * sign.to_f32();
                    let vert = |pos: Vector| BasicVertex {
                        pos: [0, 1, 2].map(|i| pos.get(i) / 3.0),
                        color,
                    };
                    [
                        vert(&a - &u - &v),
                        vert(&a + &u - &v),
                        vert(&a - &u + &v),
                        vert(&a + &u - &v),
                        vert(&a - &u + &v),
                        vert(&a + &u + &v),
                    ]
                })
                .collect_vec();

        let vertex_buffer =
            gfx.create_buffer_init("cube", &vertex_data, wgpu::BufferUsages::VERTEX);
        let uniform_buffer = gfx.create_uniform_buffer::<ViewParams>("view_uniform");
        let mat = Matrix::from_nonuniform_scaling(vector![scale.x, scale.y])
            * self.rot.euclidean_rotation_matrix();
        gfx.queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::bytes_of(&ViewParams {
                mat: [0, 1, 2, 3].map(|i| [0, 1, 2, 3].map(|j| mat.get(i, j))),
            }),
        );
        let uniform_bind_group = gfx.create_bind_group_of_buffers(
            "uniforms",
            &[(
                wgpu::ShaderStages::VERTEX,
                wgpu::BufferBindingType::Uniform,
                &uniform_buffer,
            )],
        );

        let (depth_texture, depth_texture_view) = self.depth_texture.at_size(gfx, tex_size);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_test"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.5,
                            b: 1.0,
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

            render_pass.set_pipeline(&gfx.pipelines.render_basic);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &uniform_bind_group, &[]);

            render_pass.draw(0..vertex_data.len() as u32, 0..1);
        }

        Some(color_texture_view)
    }
}

/// Data corresponding to the current puzzle state.
struct PuzzleModel {
    /// Static data about the puzzle.
    static_buffers: StaticPuzzleModel,

    /// Dynamic buffer containing a transformation matrix for each piece.
    piece_transforms: wgpu::Buffer,
}
impl PuzzleModel {
    pub fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize) -> Self {
        let matrix_size = mesh.ndim() as usize * mesh.ndim() as usize;

        PuzzleModel {
            static_buffers: StaticPuzzleModel::new(gfx, mesh, id),

            piece_transforms: gfx.create_buffer::<f32>(
                format!("puzzle{id}_piece_transforms"),
                mesh.piece_count() * matrix_size as usize,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            ),
        }
    }
}

struct StaticPuzzleModel {
    /// Static per-vertex buffer containing vertex location in N-dimensional
    /// space.
    vertex_positions: wgpu::Buffer,
    /// Static per-vertex buffer containing the first tangent vector.
    u_tangents: wgpu::Buffer,
    /// Static per-vertex buffer containing the second tangent vector.
    v_tangents: wgpu::Buffer,
    /// Static per-vertex buffer containing the vector along which to apply
    /// sticker shrink.
    sticker_shrink_vectors: wgpu::Buffer,
    /// Static per-vertex buffer containing facet ID.
    facet_ids: wgpu::Buffer,
    /// Static per-vertex buffer containing piece ID.
    piece_ids: wgpu::Buffer,

    /// Static per-piece buffer containing the centroid of a piece.
    piece_centroids: wgpu::Buffer,
    /// Static per-facet buffer containing the centroid of a facet.
    facet_centroids: wgpu::Buffer,

    /// Static buffer containing the vertex IDs of each triangle in the whole
    /// mesh.
    triangles: wgpu::Buffer,

    sticker_index_ranges: PerSticker<Range<u32>>,
    piece_internals_index_ranges: PerPiece<Range<u32>>,
}
impl StaticPuzzleModel {
    fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize) -> Self {
        StaticPuzzleModel {
            vertex_positions: gfx.create_buffer_init(
                format!("puzzle{id}_vertex_positions"),
                &mesh.vertex_positions,
                wgpu::BufferUsages::STORAGE,
            ),
            u_tangents: gfx.create_buffer_init(
                format!("puzzle{id}_u_tangents"),
                &mesh.u_tangents,
                wgpu::BufferUsages::STORAGE,
            ),
            v_tangents: gfx.create_buffer_init(
                format!("puzzle{id}_v_tangents"),
                &mesh.v_tangents,
                wgpu::BufferUsages::STORAGE,
            ),
            sticker_shrink_vectors: gfx.create_buffer_init(
                format!("puzzle{id}_sticker_shrink_vectors"),
                &mesh.sticker_shrink_vectors,
                wgpu::BufferUsages::STORAGE,
            ),
            facet_ids: gfx.create_buffer_init(
                format!("puzzle{id}_facet_ids"),
                &mesh.facet_ids,
                wgpu::BufferUsages::STORAGE,
            ),
            piece_ids: gfx.create_buffer_init(
                format!("puzzle{id}_piece_ids"),
                &mesh.piece_ids,
                wgpu::BufferUsages::STORAGE,
            ),

            piece_centroids: gfx.create_buffer_init(
                format!("puzzle{id}_piece_centroids"),
                &mesh.piece_centroids,
                wgpu::BufferUsages::STORAGE,
            ),
            facet_centroids: gfx.create_buffer_init(
                format!("puzzle{id}_facet_centroids"),
                &mesh.facet_centroids,
                wgpu::BufferUsages::STORAGE,
            ),

            triangles: gfx.create_buffer_init(
                format!("puzzle{id}_triangles"),
                &mesh.triangles,
                wgpu::BufferUsages::COPY_SRC,
            ),

            sticker_index_ranges: mesh.sticker_index_ranges.clone(),
            piece_internals_index_ranges: mesh.piece_internals_index_ranges.clone(),
        }
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

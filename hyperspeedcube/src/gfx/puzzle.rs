//! Puzzle mesh rendering.
//!
//! 1. Render polygon ID, depth, and lighting textures.
//! 2. Render result in full color.

use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use eyre::{bail, eyre, Result};
use hypermath::prelude::*;
use hyperpuzzle::{Mesh, PerPiece, PerSticker, Piece, Puzzle};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::preferences::ViewPreferences;

use super::bindings::{BindGroups, WgpuPassExt};
use super::pipelines;
use super::structs::*;
use super::{CachedTexture1d, CachedTexture2d, GraphicsState};

/// Near and far plane distance (assuming no FOV). Larger number means less
/// clipping, but also less Z buffer precision.
const Z_CLIP: f32 = 1024.0;

pub struct PuzzleRenderResources {
    pub gfx: Arc<GraphicsState>,
    pub renderer: Arc<Mutex<PuzzleRenderer>>,
    pub render_engine: RenderEngine,
    pub view_params: DrawParams,
}

impl eframe::egui_wgpu::CallbackTrait for PuzzleRenderResources {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let mut renderer = self.renderer.lock();
        let result = match self.render_engine {
            RenderEngine::SinglePass => {
                renderer.draw_puzzle_single_pass(egui_encoder, &self.view_params)
            }
            RenderEngine::MultiPass => renderer.draw_puzzle(egui_encoder, &self.view_params),
        };
        if let Err(e) = result {
            log::error!("{e}");
        }

        // egui expects sRGB colors in the shader, so we have to read the sRGB
        // texture as though it were linear to prevent the GPU from doing gamma
        // conversion.
        let src_format = renderer.buffers.composite_texture.format();
        let src_format_with_no_conversion = Some(src_format.remove_srgb_suffix());
        let texture = &renderer.buffers.composite_texture.texture;
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: src_format_with_no_conversion,
            ..Default::default()
        });

        let pipeline = &self.gfx.pipelines.blit;
        let bind_groups = pipeline.bind_groups(pipelines::blit::Bindings {
            src_texture: &texture_view,
            src_sampler: match self.view_params.prefs.downscale_interpolate {
                true => &self.gfx.bilinear_sampler,
                false => &self.gfx.nearest_neighbor_sampler,
            },
        });

        callback_resources.insert(bind_groups);

        vec![]
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a eframe::egui_wgpu::CallbackResources,
    ) {
        let Some(bind_groups) = callback_resources.get::<BindGroups>() else {
            log::error!("lost bind groups for blitting puzzle view");
            return;
        };

        render_pass.set_pipeline(&self.gfx.pipelines.blit.pipeline);
        render_pass.set_bind_groups(&bind_groups);
        render_pass.set_vertex_buffer(0, self.gfx.uv_vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..1);
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RenderEngine {
    SinglePass,
    #[default]
    MultiPass,
}
impl fmt::Display for RenderEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderEngine::SinglePass => write!(f, "Fast"),
            RenderEngine::MultiPass => write!(f, "Fancy"),
        }
    }
}

// Increment buffer IDs so each buffer has a different label in graphics
// debuggers.
fn next_buffer_id() -> usize {
    static ID: AtomicUsize = AtomicUsize::new(0);
    ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DrawParams {
    pub prefs: ViewPreferences,

    /// Width and height of the target in pixels.
    pub target_size: [u32; 2],
    /// Mouse position in NDC (normalized device coordinates).
    pub mouse_pos: [f32; 2],

    pub rot: Isometry,
    pub zoom: f32,

    pub background_color: egui::Color32,
    pub outlines_color: egui::Color32,

    pub piece_opacities: PerPiece<f32>,
}
impl Default for DrawParams {
    fn default() -> Self {
        Self {
            prefs: ViewPreferences::default(),

            target_size: [0, 0],
            mouse_pos: [0.0, 0.0],

            rot: Isometry::ident(),
            zoom: 0.5,

            background_color: egui::Color32::BLACK,
            outlines_color: egui::Color32::BLACK,

            piece_opacities: PerPiece::default(),
        }
    }
}
impl DrawParams {
    /// Returns the X and Y scale factors to use in the view matrix. Returns
    /// `Err` if either the width or height is smaller than one pixel.
    pub fn xy_scale(&self) -> Result<cgmath::Vector2<f32>> {
        if self.target_size.contains(&0) {
            bail!("puzzle view has zero size");
        }
        let w = self.target_size[0] as f32;
        let h = self.target_size[1] as f32;

        let min_dimen = f32::min(w as f32, h as f32);
        Ok(cgmath::vec2(min_dimen / w, min_dimen / h) * self.zoom)
    }

    /// Returns the factor by which the W coordinate affects the XYZ coordinates
    /// during 4D projection.
    pub fn w_factor_4d(&self) -> f32 {
        (self.prefs.fov_4d.to_radians() * 0.5).tan()
    }
    /// Returns the factor by which the Z coordinate affects the XY coordinates
    /// during 3D projection.
    pub fn w_factor_3d(&self) -> f32 {
        (self.prefs.fov_3d.to_radians() * 0.5).tan()
    }
    /// Projects an N-dimensional point to a 2D point on the screen.
    pub fn project_point(&self, p: impl ToConformalPoint) -> Option<cgmath::Point2<f32>> {
        let mut p = self.rot.transform_point(p).to_finite().ok()?;

        // Apply 4D perspective transformation.
        let w = p.get(3) as f32;
        p.resize(3);
        let mut p = p / (1.0 + w * self.w_factor_4d()) as Float;

        // Apply 3D perspective transformation.
        let z = p.get(2) as f32;
        p.resize(2);
        let p = p / (1.0 + (self.prefs.fov_3d.signum() - z) * self.w_factor_3d()) as Float;

        // Apply scaling.
        let xy_scale = self.xy_scale().ok()?;
        let x = p[0] as f32 * xy_scale.x;
        let y = p[1] as f32 * xy_scale.y;

        Some(cgmath::point2(x, y))
    }

    /// Returns a vector indicating the direction that light is shining from.
    fn light_dir(&self) -> cgmath::Vector3<f32> {
        use cgmath::{Deg, Matrix3, Vector3};

        Matrix3::from_angle_y(Deg(self.prefs.light_yaw))
            * Matrix3::from_angle_x(Deg(-self.prefs.light_pitch)) // pitch>0 means light comes from above
            * Vector3::unit_z()
    }
}

/// Define a struct with fields, doc comments, and initial values all at once.
/// This is useful in cases like defining a struct of GPU buffers, where the
/// usage of a buffer is conceptually part of its type even though it's defined
/// at runtime.
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
    /// Graphics state.
    pub gfx: Arc<GraphicsState>,
    /// Static model data, which does not change and so can be shared among all
    /// renderers of the same type of puzzle (hence `Arc`).
    model: Arc<StaticPuzzleModel>,
    /// GPU dynamic buffers, whose contents do change.
    buffers: DynamicPuzzleBuffers,
    /// Puzzle info.
    puzzle: Arc<Puzzle>,
}

impl Clone for PuzzleRenderer {
    fn clone(&self) -> Self {
        Self {
            gfx: Arc::clone(&self.gfx),
            puzzle: Arc::clone(&self.puzzle),
            model: Arc::clone(&self.model),
            buffers: self.buffers.clone(&self.gfx),
        }
    }
}

impl PuzzleRenderer {
    pub fn new(gfx: &Arc<GraphicsState>, puzzle: Arc<Puzzle>) -> Self {
        let id = next_buffer_id();
        PuzzleRenderer {
            gfx: Arc::clone(gfx),
            model: Arc::new(StaticPuzzleModel::new(&gfx, &puzzle.mesh, id)),
            buffers: DynamicPuzzleBuffers::new(Arc::clone(gfx), &puzzle.mesh, id),
            puzzle,
        }
    }

    pub fn draw_puzzle_single_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &DrawParams,
    ) -> Result<()> {
        let opacity_buckets = self.init_buffers(encoder, view_params)?;
        if opacity_buckets.is_empty() {
            return Ok(());
        }
        let index_range = opacity_buckets[0].triangles_range.start
            ..opacity_buckets.last().unwrap().triangles_range.end;

        let pipeline = &self.gfx.pipelines.render_single_pass(self.model.ndim)?;

        // Render in a single pass.
        {
            let bind_groups = pipeline.bind_groups(pipelines::render_single_pass::Bindings {
                vertex_positions: &self.model.vertex_positions,
                u_tangents: &self.model.u_tangents,
                v_tangents: &self.model.v_tangents,
                sticker_shrink_vectors: &self.model.sticker_shrink_vectors,

                piece_centroids: &self.model.piece_centroids,
                facet_centroids: &self.model.facet_centroids,
                facet_normals: &self.model.facet_normals,
                polygon_color_ids: &self.model.polygon_color_ids,

                puzzle_transform: &self.buffers.puzzle_transform,
                piece_transforms: &self.buffers.piece_transforms,
                camera_4d_pos: &self.buffers.camera_4d_pos,
                draw_params: &self.buffers.draw_params,

                sticker_colors_texture: &self.buffers.sticker_colors_texture.view,
                special_colors_texture: &self.buffers.special_colors_texture.view,
            });

            let [r, g, b, _] = egui::Rgba::from(view_params.background_color).to_array();

            let mut render_pass = pipelines::render_single_pass::PassParams {
                clear_color: [r as f64, g as f64, b as f64],
                color_texture: &self.buffers.composite_texture.view,
                depth_texture: &self.buffers.polygon_ids_depth_texture.view,
            }
            .begin_pass(encoder);

            render_pass.set_pipeline(&pipeline.pipeline);
            render_pass.set_bind_groups(&bind_groups);
            render_pass.set_vertex_buffer(0, self.model.piece_ids.slice(..));
            render_pass.set_vertex_buffer(1, self.model.facet_ids.slice(..));
            render_pass.set_vertex_buffer(2, self.model.polygon_ids.slice(..));
            render_pass.set_index_buffer(
                self.buffers.sorted_triangles.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(index_range, 0, 0..1);
            drop(render_pass);
        }

        Ok(())
    }

    pub fn draw_puzzle(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &DrawParams,
    ) -> Result<()> {
        let opacity_buckets = self.init_buffers(encoder, view_params)?;
        if opacity_buckets.is_empty() {
            return Ok(());
        }

        // Compute 3D vertex positions on the GPU.
        self.compute_3d_vertex_positions(encoder)?;

        // Render each bucket.
        let mut is_first = true;
        for bucket in opacity_buckets {
            self.render_polygon_ids(encoder, &bucket, is_first)?;
            self.render_edge_ids(encoder, &bucket, is_first)?;
            self.render_composite_puzzle(encoder, bucket.opacity, is_first)?;

            is_first = false;
        }

        Ok(())
    }

    /// Initializes buffers and returns the number of triangles to draw.
    fn init_buffers(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &DrawParams,
    ) -> Result<Vec<GeometryBucket>> {
        // Make the textures the right size.
        let size = view_params.target_size;
        self.buffers.polygon_ids_texture.set_size(size);
        self.buffers.polygon_ids_depth_texture.set_size(size);
        self.buffers.edge_ids_texture.set_size(size);
        self.buffers.edge_ids_depth_texture.set_size(size);
        self.buffers.composite_texture.set_size(size);

        if self.model.is_empty() {
            return Ok(vec![]);
        }

        // Compute the Z coordinate of the 3D camera; i.e., where the projection
        // rays converge (which may be behind the puzzle). This gives us either
        // the near plane or the far plane, depending on the sign of the 3D FOV.
        let fov_signum = view_params.prefs.fov_3d.signum();
        let camera_z = (fov_signum + 1.0 / view_params.w_factor_3d()).clamp(-Z_CLIP, Z_CLIP);

        // Write the draw parameters.
        let data = GfxDrawParams {
            light_dir: view_params.light_dir().into(),
            light_amt: view_params.prefs.light_amt,

            mouse_pos: view_params.mouse_pos,

            target_size: view_params.target_size.map(|x| x as f32),
            xy_scale: view_params.xy_scale()?.into(),

            facet_shrink: if view_params.prefs.show_internals && self.model.ndim == 3 {
                0.0
            } else {
                view_params.prefs.facet_shrink
            },
            sticker_shrink: if view_params.prefs.show_internals && self.model.ndim == 3 {
                0.0
            } else {
                view_params.prefs.sticker_shrink
            },
            piece_explode: view_params.prefs.piece_explode,

            w_factor_4d: view_params.w_factor_4d(),
            w_factor_3d: view_params.w_factor_3d(),
            fov_signum,
            near_plane_z: if fov_signum > 0.0 { camera_z } else { Z_CLIP },
            far_plane_z: if fov_signum < 0.0 { camera_z } else { -Z_CLIP },
            clip_4d_backfaces: view_params.prefs.clip_4d_backfaces as i32,
            clip_4d_behind_camera: view_params.prefs.clip_4d_behind_camera as i32,
        };
        self.gfx
            .queue
            .write_buffer(&self.buffers.draw_params, 0, bytemuck::bytes_of(&data));

        // Write the puzzle transform.
        let puzzle_transform = view_params.rot.euclidean_rotation_matrix();
        let puzzle_transform: Vec<f32> = puzzle_transform
            .cols_ndim(self.model.ndim)
            .flat_map(|column| column.iter_ndim(4).collect_vec())
            .map(|x| x as f32)
            .collect();
        self.gfx.queue.write_buffer(
            &self.buffers.puzzle_transform,
            0,
            bytemuck::cast_slice(puzzle_transform.as_slice()),
        );

        // Write the piece transforms.
        let piece_transforms = vec![Matrix::ident(self.model.ndim); self.puzzle.pieces.len()];
        let piece_transforms_data: Vec<f32> = piece_transforms
            .iter()
            .flat_map(|m| m.as_slice())
            .map(|&x| x as f32)
            .collect();
        self.gfx.queue.write_buffer(
            &self.buffers.piece_transforms,
            0,
            bytemuck::cast_slice(&piece_transforms_data),
        );

        // Write the position of the 4D camera.
        let camera_w = -1.0 - 1.0 / view_params.w_factor_4d() as Float;
        let camera_4d_pos = view_params
            .rot
            .reverse()
            .transform_point(vector![0.0, 0.0, 0.0, camera_w]);
        let camera_4d_pos_data: Vec<f32> = camera_4d_pos
            .to_finite()
            .map_err(|_| eyre!("camera 4D position is not finite"))?
            .iter_ndim(self.model.ndim)
            .map(|x| x as f32)
            .collect();
        self.gfx.queue.write_buffer(
            &self.buffers.camera_4d_pos,
            0,
            bytemuck::cast_slice(&camera_4d_pos_data),
        );

        // Write the sticker colors.
        let mut colors_data = vec![[127, 127, 127, 255]];
        colors_data.extend(
            (0..self.model.color_count)
                .map(|i| colorous::RAINBOW.eval_rational(i, self.model.color_count))
                .map(|c| c.into_array())
                .map(|[r, g, b]| [r, g, b, 255]),
        );
        self.buffers.sticker_colors_texture.write(&colors_data);

        // Write the special colors.
        let colors_data = [
            view_params.background_color.to_array(),
            view_params.outlines_color.to_array(),
        ];
        self.buffers.special_colors_texture.write(&colors_data);

        // Sort pieces into buckets by opacity and outlines and write triangle
        // indices.
        let mut triangles_buffer_index = 0;
        let mut edges_buffer_index = 0;
        let mut buckets: Vec<GeometryBucket> = vec![];
        let mut new_bucket = GeometryBucket {
            opacity: 1.0,
            outline_width: 1.0,
            triangles_range: 0..0,
            edges_range: 0..0,
        };
        for (piece, &opacity) in view_params
            .piece_opacities
            .iter()
            .sorted_by(|a, b| f32::total_cmp(&a.1, &b.1))
            .rev()
        {
            if opacity == 0.0 {
                break;
            }
            if opacity != new_bucket.opacity {
                buckets.push(new_bucket);
                new_bucket = GeometryBucket {
                    opacity,
                    outline_width: 1.0,
                    triangles_range: triangles_buffer_index..triangles_buffer_index,
                    edges_range: edges_buffer_index..edges_buffer_index,
                };
            }
            self.write_triangles_and_edges_for_piece(
                encoder,
                piece,
                &mut triangles_buffer_index,
                &mut edges_buffer_index,
                view_params,
            );
            new_bucket.triangles_range.end = triangles_buffer_index;
            new_bucket.edges_range.end = edges_buffer_index;
        }
        buckets.push(new_bucket);

        // Replace absolute opacity with incrmental opacity (difference between
        // opacities of the current bucket and the next bucket).
        for i in 0..buckets.len() - 1 {
            buckets[i].opacity -= buckets[i + 1].opacity;
        }

        Ok(buckets)
    }
    fn write_triangles_and_edges_for_piece(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        piece: Piece,
        triangles_destination_offset: &mut u32,
        edges_destination_offset: &mut u32,
        view_params: &DrawParams,
    ) {
        if view_params.prefs.show_internals {
            self.write_triangles_and_edges(
                encoder,
                &self.model.piece_internals_triangle_ranges[piece],
                triangles_destination_offset,
                &self.model.piece_internals_edge_ranges[piece],
                edges_destination_offset,
            );
        }
        for &sticker in &self.puzzle.pieces[piece].stickers {
            self.write_triangles_and_edges(
                encoder,
                &self.model.sticker_triangle_ranges[sticker],
                triangles_destination_offset,
                &self.model.sticker_edge_ranges[sticker],
                edges_destination_offset,
            );
        }
    }
    fn write_triangles_and_edges(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        triangles_index_range: &Range<u32>,
        triangles_destination_offset: &mut u32,
        edges_index_range: &Range<u32>,
        edges_destination_offset: &mut u32,
    ) {
        const INDEX_SIZE: u64 = std::mem::size_of::<u32>() as u64;

        let start = triangles_index_range.start * 3;
        let len = triangles_index_range.len() * 3;
        encoder.copy_buffer_to_buffer(
            &self.model.triangles,
            start as u64 * INDEX_SIZE,
            &self.buffers.sorted_triangles,
            *triangles_destination_offset as u64 * INDEX_SIZE,
            len as u64 * INDEX_SIZE,
        );
        *triangles_destination_offset += len as u32;

        let start = edges_index_range.start;
        let len = edges_index_range.len();
        encoder.copy_buffer_to_buffer(
            &self.model.edge_ids,
            start as u64 * INDEX_SIZE,
            &self.buffers.sorted_edges,
            *edges_destination_offset as u64 * INDEX_SIZE,
            len as u64 * INDEX_SIZE,
        );
        *edges_destination_offset += len as u32;
    }

    fn compute_3d_vertex_positions(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        let pipeline = self
            .gfx
            .pipelines
            .compute_transform_points(self.model.ndim)?;

        let bind_groups = pipeline.bind_groups(pipelines::compute_transform_points::Bindings {
            vertex_positions: &self.model.vertex_positions,
            u_tangents: &self.model.u_tangents,
            v_tangents: &self.model.v_tangents,
            sticker_shrink_vectors: &self.model.sticker_shrink_vectors,
            piece_ids: &self.model.piece_ids,
            facet_ids: &self.model.facet_ids,

            piece_centroids: &self.model.piece_centroids,
            facet_centroids: &self.model.facet_centroids,
            facet_normals: &self.model.facet_normals,
            vertex_3d_positions: &self.buffers.vertex_3d_positions,
            vertex_3d_normals: &self.buffers.vertex_3d_normals,
            vertex_culls: &self.buffers.vertex_culls,

            puzzle_transform: &self.buffers.puzzle_transform,
            piece_transforms: &self.buffers.piece_transforms,
            camera_4d_pos: &self.buffers.camera_4d_pos,
            draw_params: &self.buffers.draw_params,
        });

        let mut compute_pass =
            encoder.begin_compute_pass(&pipelines::compute_transform_points::PASS_DESCRIPTOR);

        compute_pass.set_pipeline(&pipeline.pipeline);
        compute_pass.set_bind_groups(&bind_groups);

        dispatch_work_groups(&mut compute_pass, self.model.vertex_count as u32);
        Ok(())
    }

    fn render_polygon_ids(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        bucket: &GeometryBucket,
        clear: bool,
    ) -> Result<()> {
        let pipeline = &self.gfx.pipelines.render_polygon_ids;

        let bind_groups = pipeline.bind_groups(pipelines::render_polygon_ids::Bindings {
            draw_params: &self.buffers.draw_params,
        });

        let mut render_pass = pipelines::render_polygon_ids::PassParams {
            clear,
            ids_texture: &self.buffers.polygon_ids_texture.view,
            ids_depth_texture: &self.buffers.polygon_ids_depth_texture.view,
        }
        .begin_pass(encoder);

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_groups(&bind_groups);
        render_pass.set_vertex_buffer(0, self.buffers.vertex_3d_positions.slice(..));
        render_pass.set_vertex_buffer(1, self.buffers.vertex_3d_normals.slice(..));
        render_pass.set_vertex_buffer(2, self.model.polygon_ids.slice(..));
        render_pass.set_vertex_buffer(3, self.buffers.vertex_culls.slice(..));
        render_pass.set_index_buffer(
            self.buffers.sorted_triangles.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(bucket.triangles_range.clone(), 0, 0..1);

        Ok(())
    }

    fn render_edge_ids(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        bucket: &GeometryBucket,
        clear: bool,
    ) -> Result<()> {
        let pipeline = &self.gfx.pipelines.render_edge_ids;

        let bind_groups = pipeline.bind_groups(pipelines::render_edge_ids::Bindings {
            edge_verts: &self.model.edges,
            vertex_3d_positions: &self.buffers.vertex_3d_positions,
            vertex_culls: &self.buffers.vertex_culls,

            draw_params: &self.buffers.draw_params,
        });

        let mut render_pass = pipelines::render_edge_ids::PassParams {
            clear,
            ids_texture: &self.buffers.edge_ids_texture.view,
            ids_depth_texture: &self.buffers.edge_ids_depth_texture.view,
        }
        .begin_pass(encoder);

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_groups(&bind_groups);
        render_pass.set_vertex_buffer(0, self.buffers.sorted_edges.slice(..));
        render_pass.draw(0..4, bucket.edges_range.clone());

        Ok(())
    }

    fn render_composite_puzzle(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        alpha: f32,
        clear: bool,
    ) -> Result<()> {
        let pipeline = &self.gfx.pipelines.render_composite_puzzle;

        let bind_groups = pipeline.bind_groups(pipelines::render_composite_puzzle::Bindings {
            polygon_color_ids: &self.model.polygon_color_ids,
            edges: &self.model.edges,
            vertex_3d_positions: &self.buffers.vertex_3d_positions,

            draw_params: &self.buffers.draw_params,

            sticker_colors_texture: &self.buffers.sticker_colors_texture.view,
            special_colors_texture: &self.buffers.special_colors_texture.view,

            polygon_ids_texture: &self.buffers.polygon_ids_texture.view,
            polygon_ids_depth_texture: &self.buffers.polygon_ids_depth_texture.view,
            edge_ids_texture: &self.buffers.edge_ids_texture.view,
            edge_ids_depth_texture: &self.buffers.edge_ids_depth_texture.view,
        });

        let mut render_pass = pipelines::render_composite_puzzle::PassParams {
            clear,
            target: &self.buffers.composite_texture.view,
        }
        .begin_pass(encoder);

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_groups(&bind_groups);
        render_pass.set_vertex_buffer(0, self.gfx.uv_vertex_buffer.slice(..));
        render_pass.set_blend_constant(wgpu::Color {
            r: alpha as f64,
            g: alpha as f64,
            b: alpha as f64,
            a: alpha as f64,
        });

        render_pass.draw(0..4, 0..1);
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
                const VERTEX: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX;
                const STORAGE: wgpu::BufferUsages = wgpu::BufferUsages::STORAGE;

                // Convert to i32 because WGSL doesn't support 16-bit integers yet.
                let piece_ids = mesh.piece_ids.iter().map(|&i| i.0 as u32).collect_vec();
                let facet_ids = mesh.facet_ids.iter().map(|&i| i.0 as u32).collect_vec();
                let polygon_color_ids = mesh.polygon_color_ids.iter().map(|&i| i.0 as u32).collect_vec();

                // This is just a buffer full of sequential integers so that we
                // don't have to send that data to the GPU each frame.
                let edge_ids = (0..mesh.edges.len() as u32).collect_vec();
            }

            StaticPuzzleModel {
                ndim: u8 = mesh.ndim(),
                color_count: usize = mesh.color_count(),
                vertex_count: usize = mesh.vertex_count(),
                edge_count: usize = mesh.edge_count(),
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
                /// Piece ID for each vertex.
                piece_ids:              wgpu::Buffer = buffer!(piece_ids,          VERTEX | STORAGE),
                /// Facet ID for each vertex.
                facet_ids:              wgpu::Buffer = buffer!(facet_ids,          VERTEX | STORAGE),
                /// Polygon ID for each vertex.
                polygon_ids:            wgpu::Buffer = buffer!(mesh.polygon_ids,             VERTEX),

                /*
                 * OTHER STORAGE BUFFERS
                 */
                /// Color ID for each polygon.
                polygon_color_ids:      wgpu::Buffer = buffer!(polygon_color_ids,           STORAGE),
                /// Centroid for each piece.
                piece_centroids:        wgpu::Buffer = buffer!(mesh.piece_centroids,        STORAGE),
                /// Centroid for each facet.
                facet_centroids:        wgpu::Buffer = buffer!(mesh.facet_centroids,        STORAGE),
                /// Normal vector for each facet.
                facet_normals:          wgpu::Buffer = buffer!(mesh.facet_normals,          STORAGE),
                /// Vertex IDs for each triangle in the whole mesh.
                triangles:              wgpu::Buffer = buffer!(mesh.triangles,             COPY_SRC),
                /// Vertex IDs for each edge in the whole mesh.
                edges:                  wgpu::Buffer = buffer!(mesh.edges,                  STORAGE),
                /// Sequential edge IDs.
                edge_ids:               wgpu::Buffer = buffer!(edge_ids,                   COPY_SRC),

                sticker_triangle_ranges: PerSticker<Range<u32>> = mesh.sticker_triangle_ranges.clone(),
                piece_internals_triangle_ranges: PerPiece<Range<u32>> = mesh.piece_internals_triangle_ranges.clone(),

                sticker_edge_ranges: PerSticker<Range<u32>> = mesh.sticker_edge_ranges.clone(),
                piece_internals_edge_ranges: PerPiece<Range<u32>> = mesh.piece_internals_edge_ranges.clone(),
            }
        }
    }
}
impl fmt::Debug for StaticPuzzleModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticPuzzleModel")
            .field("ndim", &self.ndim)
            .field("vertex_count", &self.vertex_count)
            .field("edge_count", &self.edge_count)
            .field("triangle_count", &self.triangle_count)
            .finish_non_exhaustive()
    }
}

impl StaticPuzzleModel {
    fn is_empty(&self) -> bool {
        // TODO: what if internals are hidden? this isn't really surefire
        self.vertex_count == 0
    }
}

struct_with_constructor! {
    /// Dynamic buffers and textures for a puzzle view.
    struct DynamicPuzzleBuffers { ... }
    impl DynamicPuzzleBuffers {
        fn new(gfx: Arc<GraphicsState>, mesh: &Mesh, id: usize) -> Self {
            {
                let ndim = mesh.ndim();
                let label = |s| format!("puzzle{id}_{s}");
            }

            DynamicPuzzleBuffers {
                /*
                 * VIEW PARAMETERS AND TRANSFORMS
                 */
                /// NxN transformation matrix for the whole puzzle.
                puzzle_transform: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("puzzle_transform"),
                    ndim as usize * 4,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                ),
                /// NxN transformation matrix for each piece.
                piece_transforms: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("piece_transforms"),
                    ndim as usize * ndim as usize * mesh.piece_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Position of the 4D camera in N-dimensional space.
                camera_4d_pos: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("camera_4d_pos"),
                    ndim as usize,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Draw parameters uniform. (constant for a given puzzle view)
                draw_params: wgpu::Buffer = gfx.create_uniform_buffer::<GfxDrawParams>(
                    label("draw_params"),
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
                /// 3D normal vector for each vertex.
                vertex_3d_normals: wgpu::Buffer = gfx.create_buffer::<[f32; 4]>(
                    label("vertex_3d_normals"),
                    mesh.vertex_count(),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                ),
                /// Vertex cull flag for each vertex.
                vertex_culls: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("vertex_culls"),
                    mesh.vertex_count(),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
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
                /// Indices of edges to draw, sorted by opacity.
                sorted_edges: wgpu::Buffer = gfx.create_buffer::<i32>(
                    label("sorted_edges"),
                    mesh.edge_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                ),

                /*
                 * TEXTURES
                 */
                /// Sticker colors texture.
                sticker_colors_texture: CachedTexture1d = CachedTexture1d::new(
                    Arc::clone(&gfx),
                    label("sticker_colors_texture"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                /// Special colors texture.
                special_colors_texture: CachedTexture1d = CachedTexture1d::new(
                    Arc::clone(&gfx),
                    label("special_colors_texture"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),

                /// Polygon ID and normal vector for each pixel.
                polygon_ids_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("polygon_ids_texture"),
                    wgpu::TextureFormat::Rg32Uint,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Depth texture for use with `polygon_ids_texture`.
                polygon_ids_depth_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("polygon_ids_depth_texture"),
                    wgpu::TextureFormat::Depth32Float,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),

                /// Edge ID for each pixel.
                edge_ids_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("edge_ids_texture"),
                    wgpu::TextureFormat::R32Uint,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Depth texture for use with `edge_ids_texture`.
                edge_ids_depth_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("edge_ids_depth_texture"),
                    wgpu::TextureFormat::Depth32Float,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),

                /// Output color texture.
                composite_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("composite_texture"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
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
            ($id:ident, $self:ident.$field:ident) => {
                $self
                    .$field
                    .clone(format!("puzzle{}_{}", $id, stringify!($field)))
            };
        }

        Self {
            puzzle_transform: clone_buffer!(gfx, id, self.puzzle_transform),
            piece_transforms: clone_buffer!(gfx, id, self.piece_transforms),
            camera_4d_pos: clone_buffer!(gfx, id, self.camera_4d_pos),
            draw_params: clone_buffer!(gfx, id, self.draw_params),

            vertex_3d_positions: clone_buffer!(gfx, id, self.vertex_3d_positions),
            vertex_3d_normals: clone_buffer!(gfx, id, self.vertex_3d_normals),
            vertex_culls: clone_buffer!(gfx, id, self.vertex_culls),
            sorted_triangles: clone_buffer!(gfx, id, self.sorted_triangles),
            sorted_edges: clone_buffer!(gfx, id, self.sorted_edges),

            sticker_colors_texture: clone_texture!(id, self.sticker_colors_texture),
            special_colors_texture: clone_texture!(id, self.special_colors_texture),
            polygon_ids_texture: clone_texture!(id, self.polygon_ids_texture),
            polygon_ids_depth_texture: clone_texture!(id, self.polygon_ids_depth_texture),
            edge_ids_texture: clone_texture!(id, self.edge_ids_texture),
            edge_ids_depth_texture: clone_texture!(id, self.edge_ids_depth_texture),
            composite_texture: clone_texture!(id, self.composite_texture),
        }
    }
}

fn dispatch_work_groups(compute_pass: &mut wgpu::ComputePass<'_>, count: u32) {
    const WORKGROUP_SIZE: u32 = 256;
    // Divide, rounding up
    let group_count = (count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
    compute_pass.dispatch_workgroups(group_count, 1, 1);
}

#[derive(Debug, Clone, PartialEq)]
struct GeometryBucket {
    opacity: f32,
    outline_width: f32,
    triangles_range: Range<u32>,
    edges_range: Range<u32>,
}

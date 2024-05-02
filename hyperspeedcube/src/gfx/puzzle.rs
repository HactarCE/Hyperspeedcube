//! Puzzle mesh rendering.
//!
//! 1. Render polygons to a texture: color ID, normal vector, and depth.
//! 2. Render edges to a texture: edge ID and depth.
//! 3. Composite results and antialias.
//! 4. Repeat all three steps for each opacity level.

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use bitvec::bitbox;
use bitvec::order::Lsb0;
use egui::NumExt;
use eyre::{bail, Result};
use hypermath::prelude::*;
use hyperpuzzle::{Mesh, PerPiece, PerSticker, Piece, Puzzle};
use itertools::Itertools;
use parking_lot::Mutex;

use super::bindings::{BindGroups, WgpuPassExt};
use super::draw_params::GeometryCacheKey;
use super::structs::*;
use super::{pipelines, CachedTexture1d, CachedTexture2d, DrawParams, GraphicsState};

/// Near and far plane distance (assuming no FOV). Larger number means less
/// clipping far from the camera, but also less Z buffer precision.
const Z_CLIP: f32 = 8.0;

/// Minimum distance of the near/far clipping plane from the camera Z
/// coordinate. Larger number means more clipping near the camera, but also more
/// Z buffer precision.
const Z_EPSILON: f32 = 0.01;

/// Whether to send the mouse position to the GPU. This is useful for debugging
/// purposes, but causes the puzzle to redraw every frame that the mouse moves,
/// even when not necessary.
const SEND_MOUSE_POS: bool = false;

/// Color ID for the background.
const BACKGROUND_COLOR_ID: u32 = 0;
/// Color ID for the internals.
const INTERNALS_COLOR_ID: u32 = 1;
/// First color ID for stickers.
const FACES_BASE_COLOR_ID: u32 = 2;

/// How much to scale outline radius values compared to size of one 3D unit.
const OUTLINE_RADIUS_SCALE_FACTOR: f32 = 0.005;

pub struct PuzzleRenderResources {
    pub gfx: Arc<GraphicsState>,
    pub renderer: Arc<Mutex<PuzzleRenderer>>,
}

impl PuzzleRenderResources {
    fn unique_key(&self) -> usize {
        Arc::as_ptr(&self.renderer) as usize
    }
}

impl eframe::egui_wgpu::CallbackTrait for PuzzleRenderResources {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let mut renderer = self.renderer.lock();

        let result = renderer.draw_puzzle(egui_encoder);
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

        let Some(draw_params) = renderer.draw_params() else {
            return vec![];
        };

        let pipeline = &self.gfx.pipelines.blit;
        let bind_groups = pipeline.bind_groups(pipelines::blit::Bindings {
            src_texture: &texture_view,
            src_sampler: match draw_params.cam.prefs.downscale_interpolate {
                true => &self.gfx.bilinear_sampler,
                false => &self.gfx.nearest_neighbor_sampler,
            },
        });

        callback_resources
            .entry()
            .or_insert(HashMap::new())
            .insert(self.unique_key(), bind_groups);

        vec![]
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a eframe::egui_wgpu::CallbackResources,
    ) {
        let Some(bind_groups) = callback_resources
            .get::<HashMap<usize, BindGroups>>()
            .and_then(|map| map.get(&self.unique_key()))
        else {
            log::error!("lost bind groups for blitting puzzle view");
            return;
        };

        render_pass.set_pipeline(&self.gfx.pipelines.blit.pipeline);
        render_pass.set_bind_groups(bind_groups);
        render_pass.set_vertex_buffer(0, self.gfx.uv_vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..1);
    }
}

// Increment buffer IDs so each buffer has a different label in graphics
// debuggers.
fn next_buffer_id() -> usize {
    static ID: AtomicUsize = AtomicUsize::new(0);
    ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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

    prep: Option<DrawPrepResponse>,
}

impl Clone for PuzzleRenderer {
    fn clone(&self) -> Self {
        Self {
            gfx: Arc::clone(&self.gfx),
            puzzle: Arc::clone(&self.puzzle),
            model: Arc::clone(&self.model),
            buffers: self.buffers.clone(&self.gfx),

            prep: None,
        }
    }
}

impl PuzzleRenderer {
    pub fn new(gfx: &Arc<GraphicsState>, puzzle: &Arc<Puzzle>) -> Self {
        let id = next_buffer_id();
        PuzzleRenderer {
            gfx: Arc::clone(gfx),
            model: Arc::new(StaticPuzzleModel::new(&gfx, &puzzle.mesh, id, &puzzle)),
            buffers: DynamicPuzzleBuffers::new(Arc::clone(gfx), &puzzle.mesh, id),
            puzzle: Arc::clone(puzzle),

            prep: None,
        }
    }

    /// Sets the draw parameters to be used for the next
    pub fn prepare_draw(&mut self, mut draw_params: DrawParams) -> DrawPrepResponse {
        if !SEND_MOUSE_POS {
            draw_params.mouse_pos = [0.0; 2];
        }

        let geometry_cache_key = draw_params.geometry_cache_key(self.puzzle.ndim());

        let needs_recompute_vertex_3d_positions =
            self.geometry_cache_key() != Some(&geometry_cache_key);
        let needs_redraw =
            needs_recompute_vertex_3d_positions || self.draw_params() != Some(&draw_params);

        let vertex_3d_positions = {
            let existing_output = self
                .prep
                .as_ref()
                .and_then(|prep| prep.vertex_3d_positions.as_ref());
            if needs_recompute_vertex_3d_positions {
                None
            } else if existing_output.is_some() {
                existing_output.map(Arc::clone)
            } else {
                // If we are drawing two frames in a row with the same geometry
                // cache key, then the 3D vertex positions have stabilized, so
                // fetch them from the GPU.
                let output = Arc::new(Mutex::new(None));

                let output_ref = Arc::clone(&output);
                // Save the 3D vertex positions.
                wgpu::util::DownloadBuffer::read_buffer(
                    &self.gfx.device,
                    &self.gfx.queue,
                    &self.buffers.vertex_3d_positions.slice(..),
                    move |result| match result {
                        Ok(buffer) => {
                            *output_ref.lock() = Some(Arc::new(
                                bytemuck::cast_slice::<u8, f32>(&buffer)
                                    .chunks_exact(4)
                                    .map(|a| cgmath::vec4(a[0], a[1], a[2], a[3]))
                                    .collect(),
                            ));
                        }
                        Err(wgpu::BufferAsyncError) => {
                            log::error!("Error mapping 3D vertex positions buffer")
                        }
                    },
                );

                Some(output)
            }
        };

        let r = DrawPrepResponse {
            geometry_cache_key,
            needs_recompute_vertex_3d_positions,
            vertex_3d_positions,

            draw_params,
            needs_redraw,
        };

        self.prep = Some(r.clone());

        r
    }

    pub fn draw_params(&self) -> Option<&DrawParams> {
        Some(&self.prep.as_ref()?.draw_params)
    }
    fn geometry_cache_key(&self) -> Option<&GeometryCacheKey> {
        Some(&self.prep.as_ref()?.geometry_cache_key)
    }
    pub fn vertex_3d_positions(&self) -> Option<Arc<Vec<cgmath::Vector4<f32>>>> {
        self.prep
            .as_ref()?
            .vertex_3d_positions
            .as_ref()?
            .lock()
            .as_ref()
            .map(Arc::clone)
    }

    pub fn draw_puzzle(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        let Some(prep) = self.prep.clone() else {
            bail!("cannot draw without call to prepare_draw()");
        };

        if prep.needs_recompute_vertex_3d_positions {
            log::trace!(
                "Recomputing 3D vertex positions for puzzle {:?}",
                self.puzzle.name,
            );
            // Compute 3D vertex positions on the GPU.
            self.compute_3d_vertex_positions(encoder)?;
        }

        if prep.needs_redraw {
            log::trace!("Redrawing puzzle {:?}", self.puzzle.name);
            let opacity_buckets = self.init_buffers(encoder, &prep.draw_params)?;

            // Render each bucket. Use `is_first` to clear the texture only on the
            // first pass.
            let mut is_first = true;
            for bucket in opacity_buckets {
                self.render_polygons(encoder, &bucket, is_first)?;
                self.render_edge_ids(encoder, &bucket, is_first)?;
                self.render_composite_puzzle(encoder, bucket.opacity, is_first)?;

                is_first = false;
            }
        }

        Ok(())
    }

    /// Initializes buffers and returns the number of triangles to draw.
    fn init_buffers(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        draw_params: &DrawParams,
    ) -> Result<Vec<GeometryBucket>> {
        // Make the textures the right size.
        let size = draw_params.cam.target_size;
        self.buffers.polygons_texture.set_size(size);
        self.buffers.polygons_depth_texture.set_size(size);
        self.buffers.edge_ids_texture.set_size(size);
        self.buffers.edge_ids_depth_texture.set_size(size);
        self.buffers.composite_texture.set_size(size);

        if self.model.is_empty() {
            return Ok(vec![]);
        }

        // Compute the Z coordinate of the 3D camera; i.e., where the projection
        // rays converge (which may be behind the puzzle). This gives us either
        // the near plane or the far plane, depending on the sign of the 3D FOV.
        let fov_signum = draw_params.cam.prefs.fov_3d.signum();
        let camera_z = (fov_signum + 1.0 / draw_params.cam.w_factor_3d()).clamp(-Z_CLIP, Z_CLIP);

        // Write the draw parameters.
        {
            let near_plane_z = if fov_signum > 0.0 {
                (camera_z - Z_EPSILON).at_most(Z_CLIP)
            } else {
                Z_CLIP
            };
            let far_plane_z = if fov_signum < 0.0 {
                (camera_z + Z_EPSILON).at_least(-Z_CLIP)
            } else {
                -Z_CLIP
            };

            let w_factor_4d = draw_params.cam.w_factor_4d();
            let w_factor_3d = draw_params.cam.w_factor_3d();

            let data = GfxDrawParams {
                pre: GfxPrecomputedValues::new(w_factor_3d, near_plane_z, far_plane_z),

                light_dir: draw_params.light_dir().into(),
                face_light_intensity: draw_params.cam.prefs.face_light_intensity,
                outline_light_intensity: draw_params.cam.prefs.outline_light_intensity,

                pixel_size: draw_params.cam.pixel_size()?,
                target_size: draw_params.cam.target_size_f32().into(),
                xy_scale: draw_params.cam.xy_scale()?.into(),

                mouse_pos: draw_params.mouse_pos,

                facet_shrink: draw_params.facet_shrink(self.puzzle.ndim()),
                sticker_shrink: draw_params.sticker_shrink(self.puzzle.ndim()),
                piece_explode: draw_params.cam.prefs.piece_explode,

                w_factor_4d,
                w_factor_3d,
                fov_signum,
                near_plane_z,
                far_plane_z,
                clip_4d_backfaces: draw_params.cam.prefs.clip_4d_backfaces as i32,
                clip_4d_behind_camera: draw_params.cam.prefs.clip_4d_behind_camera as i32,

                _padding: [0.0; 2],
            };
            self.gfx
                .queue
                .write_buffer(&self.buffers.draw_params, 0, bytemuck::bytes_of(&data));
        }

        // Write the puzzle transform.
        {
            let puzzle_transform = draw_params.cam.rot.euclidean_rotation_matrix();
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
        }

        // Write the piece transforms.
        {
            let piece_transforms_data: Vec<f32> = draw_params
                .piece_transforms
                .iter_values()
                .flat_map(|m| m.as_slice())
                .map(|&x| x as f32)
                .collect();
            self.gfx.queue.write_buffer(
                &self.buffers.piece_transforms,
                0,
                bytemuck::cast_slice(&piece_transforms_data),
            );
        }

        // Write the position of the 4D camera.
        {
            let camera_w = -1.0 - 1.0 / draw_params.cam.w_factor_4d() as Float;
            let camera_4d_pos = draw_params
                .cam
                .rot
                .reverse()
                .transform_point(vector![0.0, 0.0, 0.0, camera_w]);
            let camera_4d_pos_data: Vec<f32> = camera_4d_pos
                .iter_ndim(self.model.ndim)
                .map(|x| x as f32)
                .collect();
            self.gfx.queue.write_buffer(
                &self.buffers.camera_4d_pos,
                0,
                bytemuck::cast_slice(&camera_4d_pos_data),
            );
        }

        // Assign each unique color an identifying index.
        // 0 = background
        // 1 = internals
        // 2+N = sticker color N
        // others = other colors (such as outlines)
        let mut color_palette = vec![draw_params.background_color, draw_params.internals_color];
        color_palette.extend((0..self.model.color_count).map(|i| {
            colorous::RAINBOW
                .eval_rational(i, self.model.color_count)
                .into_array()
        }));
        let mut color_ids: HashMap<[u8; 3], u32> = color_palette
            .iter()
            .enumerate()
            .map(|(i, &color)| (color, i as u32))
            .collect();
        for new_color in draw_params
            .piece_styles
            .iter()
            .flat_map(|(style_values, _)| [style_values.face_color, style_values.outline_color])
        {
            color_ids.entry(new_color).or_insert_with(|| {
                let idx = color_palette.len() as u32;
                color_palette.push(new_color);
                idx
            });
        }
        let mut color_palette_size = color_palette.len() as u32;
        let max_color_palette_size = self.gfx.device.limits().max_texture_dimension_1d;
        if color_palette_size > max_color_palette_size {
            log::warn!(
                "Color palette size ({color_palette_size}) exceeds \
                 maximum 1D texture size ({max_color_palette_size})"
            );
            color_palette_size = max_color_palette_size;
        }
        self.buffers
            .color_palette_texture
            .set_size(color_palette_size);

        // Write color palette. TOOD: only write to buffer when it changes
        let color_palette_data = color_palette
            .into_iter()
            .map(|[r, g, b]| [r, g, b, 255])
            .collect_vec();
        self.buffers
            .color_palette_texture
            .write(&color_palette_data);

        if draw_params.piece_styles.is_empty() {
            log::error!("no piece styles");
            return Ok(vec![]);
        }

        // Create a map from pieces to styles.
        let mut piece_style_indices = self.puzzle.pieces.map_ref(|_, _| 0);
        for (i, (_style, piece_set)) in draw_params.piece_styles.iter().enumerate() {
            for piece in piece_set.iter_ones() {
                piece_style_indices[Piece(piece as _)] = i;
            }
        }

        let mut polygon_color_ids_data = vec![0; self.model.polygon_count];
        let mut outline_color_ids_data = vec![0; self.model.edge_count];
        let mut outline_radii_data = vec![0.0; self.model.edge_count];

        fn u32_range_to_usize(r: &Range<u32>) -> Range<usize> {
            r.start as usize..r.end as usize
        }

        for (piece, piece_info) in &self.puzzle.pieces {
            let style = draw_params.piece_styles[piece_style_indices[piece]].0;
            let fallback_face_color = color_ids[&style.face_color];
            let fallback_outline_color = color_ids[&style.outline_color];

            // Should the faces/outlines use the sticker color?
            let face_sticker_color = style.face_sticker_color;
            let outline_sticker_color = style.outline_sticker_color
                && draw_params.outlines_may_use_sticker_color(self.puzzle.ndim());

            for &sticker in &piece_info.stickers {
                let sticker_color =
                    FACES_BASE_COLOR_ID + self.puzzle.stickers[sticker].color.0 as u32;

                // Write sticker face colors.
                let face_color_id = match face_sticker_color {
                    true => sticker_color,
                    false => fallback_face_color,
                };
                let polygon_range = &self.model.sticker_polygon_ranges[sticker];
                polygon_color_ids_data[polygon_range.clone()].fill(face_color_id);

                // Write sticker outline colors.
                let outline_color_id = match outline_sticker_color {
                    true => sticker_color,
                    false => fallback_outline_color,
                };
                let edge_range = u32_range_to_usize(&self.model.sticker_edge_ranges[sticker]);
                outline_color_ids_data[edge_range.clone()].fill(outline_color_id);
                // Write sticker outline radii.
                outline_radii_data[edge_range]
                    .fill(style.outline_size * OUTLINE_RADIUS_SCALE_FACTOR);
            }

            // Write internals face colors.
            let face_color_id = match face_sticker_color {
                true => INTERNALS_COLOR_ID,
                false => fallback_face_color,
            };
            let polygon_range = &self.model.piece_internals_polygon_ranges[piece];
            polygon_color_ids_data[polygon_range.clone()].fill(face_color_id);

            // Write internals outline colors.
            let outline_color_id = match outline_sticker_color {
                true => INTERNALS_COLOR_ID,
                false => fallback_outline_color,
            };
            let edge_range = u32_range_to_usize(&self.model.piece_internals_edge_ranges[piece]);
            outline_color_ids_data[edge_range.clone()].fill(outline_color_id);
            // Write internals outline radii.
            outline_radii_data[edge_range].fill(style.outline_size * OUTLINE_RADIUS_SCALE_FACTOR);
        }

        // Ok but now actually write that data.
        // TODO: only write to buffer when it changes
        self.gfx.queue.write_buffer(
            &self.buffers.polygon_color_ids,
            0,
            bytemuck::cast_slice(&polygon_color_ids_data),
        );
        self.gfx.queue.write_buffer(
            &self.buffers.outline_color_ids,
            0,
            bytemuck::cast_slice(&outline_color_ids_data),
        );
        self.gfx.queue.write_buffer(
            &self.buffers.outline_radii,
            0,
            bytemuck::cast_slice(&outline_radii_data),
        );

        // Make a list of unique opacity values in sorted order.
        let mut bucket_opacities: Vec<u8> = draw_params
            .piece_styles
            .iter()
            .flat_map(|(style, _)| [style.face_opacity, style.outline_opacity])
            .sorted_unstable()
            .rev()
            .dedup()
            .collect();
        let mut bucket_id_by_opacity = vec![0; 256];
        for (i, &opacity) in bucket_opacities.iter().enumerate() {
            bucket_id_by_opacity[opacity as usize] = i as u8;
        }
        // For each bucket, get the set of pieces whose faces/edges are in that
        // bucket.
        let empty_piece_set = bitbox![u64, Lsb0; 0; self.puzzle.pieces.len()];
        let mut bucket_face_pieces = vec![empty_piece_set; bucket_opacities.len()];
        let mut bucket_edge_pieces = bucket_face_pieces.clone();
        for (style, piece_set) in &draw_params.piece_styles {
            let triangles_bucket_id = bucket_id_by_opacity[style.face_opacity as usize];
            bucket_face_pieces[triangles_bucket_id as usize] |= piece_set;

            let edges_bucket_id = bucket_id_by_opacity[style.outline_opacity as usize];
            bucket_edge_pieces[edges_bucket_id as usize] |= piece_set;
        }

        // If the first bucket is totally transparent, then skip it.
        if bucket_opacities.first() == Some(&0) {
            bucket_opacities.remove(0);
            bucket_face_pieces.remove(0);
            bucket_edge_pieces.remove(0);
        }
        if bucket_opacities.is_empty() {
            // TODO: still draw background color somehow!
            return Ok(vec![]);
        }

        let mut triangles_buffer_index = 0;
        let mut edges_buffer_index = 0;

        let mut buckets: Vec<GeometryBucket> =
            itertools::izip!(bucket_opacities, bucket_face_pieces, bucket_edge_pieces)
                .map(|(opacity, face_pieces, outline_pieces)| {
                    let triangles_buffer_start = triangles_buffer_index;
                    for piece_id in face_pieces.iter_ones() {
                        self.write_geometry_for_piece(
                            encoder,
                            Piece(piece_id as _),
                            GeometryType::Faces,
                            draw_params.cam.prefs.show_internals,
                            &mut triangles_buffer_index,
                        );
                    }
                    let triangles_range = triangles_buffer_start..triangles_buffer_index;

                    let edges_buffer_start = edges_buffer_index;
                    for piece_id in outline_pieces.iter_ones() {
                        self.write_geometry_for_piece(
                            encoder,
                            Piece(piece_id as _),
                            GeometryType::Edges,
                            draw_params.cam.prefs.show_internals,
                            &mut edges_buffer_index,
                        );
                    }
                    let edges_range = edges_buffer_start..edges_buffer_index;

                    GeometryBucket {
                        opacity: opacity as f32 / 255.0,
                        triangles_range,
                        edges_range,
                    }
                })
                .collect();

        // Replace absolute opacity with incrmental opacity (difference between
        // opacities of the current bucket and the next bucket).
        for i in 0..buckets.len() - 1 {
            buckets[i].opacity -= buckets[i + 1].opacity;
        }

        Ok(buckets)
    }
    fn write_geometry_for_piece(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        piece: Piece,
        geometry_type: GeometryType,
        include_internals: bool,
        dst_offset: &mut u32,
    ) {
        let src = match geometry_type {
            GeometryType::Faces => &self.model.triangles,
            GeometryType::Edges => &self.model.edge_ids,
        };
        let dst = match geometry_type {
            GeometryType::Faces => &self.buffers.sorted_triangles,
            GeometryType::Edges => &self.buffers.sorted_edges,
        };

        if include_internals {
            let index_range = match geometry_type {
                GeometryType::Faces => &self.model.piece_internals_triangle_ranges[piece],
                GeometryType::Edges => &self.model.piece_internals_edge_ranges[piece],
            };
            self.write_geometry(encoder, geometry_type, src, dst, index_range, dst_offset);
        }

        for &sticker in &self.puzzle.pieces[piece].stickers {
            let index_range = match geometry_type {
                GeometryType::Faces => &self.model.sticker_triangle_ranges[sticker],
                GeometryType::Edges => &self.model.sticker_edge_ranges[sticker],
            };
            self.write_geometry(encoder, geometry_type, src, dst, index_range, dst_offset);
        }
    }
    fn write_geometry(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        geometry_type: GeometryType,
        src: &wgpu::Buffer,
        dst: &wgpu::Buffer,
        index_range: &Range<u32>,
        destination_offset: &mut u32,
    ) {
        const INDEX_SIZE: u64 = std::mem::size_of::<u32>() as u64;
        let numbers_per_entry = match geometry_type {
            GeometryType::Faces => 3,
            GeometryType::Edges => 1,
        };

        let start = index_range.start * numbers_per_entry;
        let len = index_range.len() as u32 * numbers_per_entry;
        encoder.copy_buffer_to_buffer(
            src,
            start as u64 * INDEX_SIZE,
            dst,
            *destination_offset as u64 * INDEX_SIZE,
            len as u64 * INDEX_SIZE,
        );
        *destination_offset += len;
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

    fn render_polygons(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        bucket: &GeometryBucket,
        clear: bool,
    ) -> Result<()> {
        let pipeline = &self.gfx.pipelines.render_polygons;

        let bind_groups = pipeline.bind_groups(pipelines::render_polygons::Bindings {
            polygon_color_ids: &self.buffers.polygon_color_ids,
            draw_params: &self.buffers.draw_params,
        });

        let mut render_pass = pipelines::render_polygons::PassParams {
            clear,
            ids_texture: &self.buffers.polygons_texture.view,
            ids_depth_texture: &self.buffers.polygons_depth_texture.view,
        }
        .begin_pass(encoder);

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_groups(&bind_groups);
        render_pass.set_vertex_buffer(0, self.buffers.vertex_3d_positions.slice(..));
        render_pass.set_vertex_buffer(1, self.buffers.vertex_3d_normals.slice(..));
        render_pass.set_vertex_buffer(2, self.model.polygon_ids.slice(..));
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

            outline_radii: &self.buffers.outline_radii,
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
            edges: &self.model.edges,
            vertex_3d_positions: &self.buffers.vertex_3d_positions,

            outline_color_ids: &self.buffers.outline_color_ids,
            outline_radii: &self.buffers.outline_radii,
            draw_params: &self.buffers.draw_params,

            color_palette_texture: &self.buffers.color_palette_texture.view,

            polygons_texture: &self.buffers.polygons_texture.view,
            polygons_depth_texture: &self.buffers.polygons_depth_texture.view,
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
        fn new(gfx: &GraphicsState, mesh: &Mesh, id: usize, puzzle: &Puzzle) -> Self {
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

                // This is just a buffer full of sequential integers so that we
                // don't have to send that data to the GPU each frame.
                let edge_ids = (0..mesh.edges.len() as u32).collect_vec();
            }

            StaticPuzzleModel {
                ndim: u8 = mesh.ndim(),
                color_count: usize = mesh.color_count,
                vertex_count: usize = mesh.vertex_count(),
                polygon_count: usize = mesh.polygon_count,
                edge_count: usize = mesh.edge_count(),
                triangle_count: usize = mesh.triangle_count(),
                sticker_polygon_ranges: PerSticker<Range<usize>> = mesh.sticker_polygon_ranges.clone(),
                piece_internals_polygon_ranges: PerPiece<Range<usize>> = mesh.piece_internals_polygon_ranges.clone(),

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
                piece_ids:              wgpu::Buffer = buffer!(piece_ids,          VERTEX | STORAGE), // TODO: only VERTEX for single-pass pipeline
                /// Facet ID for each vertex.
                facet_ids:              wgpu::Buffer = buffer!(facet_ids,          VERTEX | STORAGE), // TODO: only VERTEX for single-pass pipeline
                /// Polygon ID for each vertex.
                polygon_ids:            wgpu::Buffer = buffer!(mesh.polygon_ids,   VERTEX | STORAGE), // TODO: only VERTEX for single-pass pipeline

                /*
                 * OTHER STORAGE BUFFERS
                 */
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
                    ndim as usize * ndim as usize * mesh.piece_count,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Position of the 4D camera in N-dimensional space.
                camera_4d_pos: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("camera_4d_pos"),
                    ndim as usize,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Polygon color IDs.
                polygon_color_ids: wgpu::Buffer = gfx.create_buffer::<u32>(
                    label("polygon_color_ids"),
                    mesh.edge_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Outline color IDs.
                outline_color_ids: wgpu::Buffer = gfx.create_buffer::<u32>(
                    label("outline_color_ids"),
                    mesh.edge_count(),
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                ),
                /// Outline radii.
                outline_radii: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("outline_radii"),
                    mesh.edge_count(),
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
                    wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                ),
                /// 3D normal vector for each vertex.
                vertex_3d_normals: wgpu::Buffer = gfx.create_buffer::<[f32; 4]>(
                    label("vertex_3d_normals"),
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
                /// Color palette texture.
                color_palette_texture: CachedTexture1d = CachedTexture1d::new(
                    Arc::clone(&gfx),
                    label("color_palette_texture"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),

                /// Polygon color ID and normal vector for each pixel.
                polygons_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("polygons_texture"),
                    wgpu::TextureFormat::Rg32Uint,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Depth texture for use with `polygons_texture`.
                polygons_depth_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("polygons_depth_texture"),
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
            polygon_color_ids: clone_buffer!(gfx, id, self.polygon_color_ids),
            outline_color_ids: clone_buffer!(gfx, id, self.outline_color_ids),
            outline_radii: clone_buffer!(gfx, id, self.outline_radii),
            draw_params: clone_buffer!(gfx, id, self.draw_params),

            vertex_3d_positions: clone_buffer!(gfx, id, self.vertex_3d_positions),
            vertex_3d_normals: clone_buffer!(gfx, id, self.vertex_3d_normals),
            sorted_triangles: clone_buffer!(gfx, id, self.sorted_triangles),
            sorted_edges: clone_buffer!(gfx, id, self.sorted_edges),

            color_palette_texture: clone_texture!(id, self.color_palette_texture),

            polygons_texture: clone_texture!(id, self.polygons_texture),
            polygons_depth_texture: clone_texture!(id, self.polygons_depth_texture),
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
    triangles_range: Range<u32>,
    edges_range: Range<u32>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum GeometryType {
    Faces,
    Edges,
}

#[derive(Debug, Clone)]
pub(crate) struct DrawPrepResponse {
    /// Cache key for `vertex_3d_positions`.
    geometry_cache_key: GeometryCacheKey,
    /// Whether the 3D vertex positions for the puzzle should be recomputed.
    pub needs_recompute_vertex_3d_positions: bool,
    /// Output location for 3D vertex positions, which will be fetched from the
    /// GPU at some point.
    ///
    /// If the outer `Option` is `None`, then there is no active request so
    /// vertex positions will not be fetched.
    ///
    /// If the inner `Option` is `None`, then we have fetched vertex positions
    /// from the GPU and they should be delivered by the next frame.
    pub vertex_3d_positions: Option<Arc<Mutex<Option<Arc<Vec<cgmath::Vector4<f32>>>>>>>,

    /// Cache key for redrawing the whole puzzle.
    pub draw_params: DrawParams,
    /// Whether the puzzle should be redrawn.
    pub needs_redraw: bool,
}

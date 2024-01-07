//! Puzzle mesh rendering.
//!
//! 1. Render polygon ID, depth, and lighting textures.
//! 2. Render result in full color.

use std::fmt;
use std::ops::Range;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use eyre::{bail, eyre, OptionExt, Result};
use hypermath::prelude::*;
use hyperpuzzle::{Mesh, PerPiece, PerSticker, Piece, Puzzle};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::preferences::ViewPreferences;

use super::structs::*;
use super::{CachedTexture1d, CachedTexture2d, GraphicsState};

pub struct PuzzleRenderResources {
    pub gfx: Arc<GraphicsState>,
    pub renderer: Arc<Mutex<PuzzleRenderer>>,
    pub render_engine: RenderEngine,
    pub view_params: ViewParams,
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

        let sampler = self
            .gfx
            .device
            .create_sampler(&wgpu::SamplerDescriptor::default());

        // egui expects sRGB colors in the shader, so we have to read the sRGB
        // texture as though it were linear to prevent the GPU from doing gamma
        // conversion.
        let format = Some(renderer.buffers.out_texture.format().remove_srgb_suffix());
        let texture = &renderer.buffers.out_texture.texture;
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format,
            ..Default::default()
        });
        let bind_groups: Vec<(u32, wgpu::BindGroup)> =
            self.gfx.pipelines.blit_bind_groups.bind_groups(
                &self.gfx.device,
                &[&[
                    wgpu::BindingResource::TextureView(&texture_view),
                    wgpu::BindingResource::Sampler(&sampler),
                ]],
            );

        callback_resources.insert(bind_groups);

        vec![]
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a eframe::egui_wgpu::CallbackResources,
    ) {
        render_pass.set_pipeline(&self.gfx.pipelines.blit);

        let Some(bind_groups) = callback_resources.get::<Vec<(u32, wgpu::BindGroup)>>() else {
            log::error!("lost bind groups for blitting puzzle view");
            return;
        };
        for (index, bind_group) in bind_groups {
            render_pass.set_bind_group(*index, bind_group, &[]);
        }

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
pub struct ViewParams {
    pub width: u32,
    pub height: u32,

    pub rot: Isometry,
    pub zoom: f32,

    pub background_color: egui::Color32,
    pub outlines_color: egui::Color32,

    pub prefs: ViewPreferences,

    pub piece_opacities: PerPiece<f32>,
}
impl Default for ViewParams {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,

            rot: Isometry::ident(),
            zoom: 0.3,

            background_color: egui::Color32::BLACK,
            outlines_color: egui::Color32::BLACK,

            prefs: ViewPreferences::default(),

            piece_opacities: PerPiece::default(),
        }
    }
}
impl ViewParams {
    /// Returns the X and Y scale factors to use in the view matrix. Returns
    /// `Err` if either the width or height is smaller than one pixel.
    pub fn xy_scale(&self) -> Result<cgmath::Vector2<f32>> {
        if self.width == 0 || self.height == 0 {
            bail!("puzzle view has zero size");
        }
        let w = self.width as f32;
        let h = self.height as f32;

        let min_dimen = f32::min(w as f32, h as f32);
        Ok(cgmath::vec2(min_dimen / w, min_dimen / h) * self.zoom)
    }

    pub fn w_factor_4d(&self) -> f32 {
        (self.prefs.fov_4d.to_radians() * 0.5).tan()
    }
    pub fn w_factor_3d(&self) -> f32 {
        (self.prefs.fov_3d.to_radians() * 0.5).tan()
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
        let p = p / (1.0 + (self.prefs.fov_3d.signum() - z) * self.w_factor_3d()) as Float;

        // Apply scaling.
        let xy_scale = self.xy_scale().ok()?;
        let x = p[0] as f32 * xy_scale.x;
        let y = p[1] as f32 * xy_scale.y;

        Some(cgmath::point2(x, y))
    }

    /// Returns the projection parameters to send to the GPU.
    fn gfx_projection_params(&self, ndim: u8) -> GfxProjectionParams {
        GfxProjectionParams {
            facet_shrink: if self.prefs.show_internals && ndim == 3 {
                0.0
            } else {
                self.prefs.facet_shrink
            },
            sticker_shrink: if self.prefs.show_internals && ndim == 3 {
                0.0
            } else {
                self.prefs.sticker_shrink
            },
            piece_explode: self.prefs.piece_explode,

            w_factor_4d: self.w_factor_4d(),
            w_factor_3d: self.w_factor_3d(),
            fov_signum: self.prefs.fov_3d.signum(),
        }
    }

    fn light_ambient_amount(&self) -> f32 {
        hypermath::util::lerp(
            0.0,
            1.0 - self.prefs.light_directional,
            self.prefs.light_ambient,
        )
    }
    fn light_vector(&self) -> cgmath::Vector3<f32> {
        use cgmath::{Deg, Matrix3, Vector3};

        Matrix3::from_angle_y(Deg(self.prefs.light_yaw))
            * Matrix3::from_angle_x(Deg(-self.prefs.light_pitch)) // pitch>0 means light comes from above
            * Vector3::unit_z()
            * self.prefs.light_directional
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

    pub fn clone(&self) -> Self {
        Self {
            gfx: Arc::clone(&self.gfx),
            puzzle: Arc::clone(&self.puzzle),
            model: Arc::clone(&self.model),
            buffers: self.buffers.clone(&self.gfx),
        }
    }

    pub fn draw_puzzle_single_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &ViewParams,
    ) -> Result<()> {
        let opacity_buckets = self.init_buffers(encoder, view_params)?;
        if opacity_buckets.is_empty() {
            return Ok(());
        }
        let index_range =
            opacity_buckets[0].index_range.start..opacity_buckets.last().unwrap().index_range.end;

        let pipelines = &self.gfx.pipelines;

        // Make the textures the right size.
        let size = [view_params.width, view_params.height];
        self.buffers.depth_texture.set_size(size);
        self.buffers.out_texture.set_size(size);

        // Render in a single pass.
        {
            let bind_groups = pipelines.render_single_pass_bind_groups.bind_groups(
                &self.gfx.device,
                &[
                    &[
                        self.model.vertex_positions.as_entire_binding(),
                        self.model.u_tangents.as_entire_binding(),
                        self.model.v_tangents.as_entire_binding(),
                        self.model.sticker_shrink_vectors.as_entire_binding(),
                    ],
                    &[
                        self.model.piece_centroids.as_entire_binding(),
                        self.model.facet_centroids.as_entire_binding(),
                        self.model.facet_normals.as_entire_binding(),
                        self.model.polygon_color_ids.as_entire_binding(),
                    ],
                    &[
                        self.buffers.puzzle_transform.as_entire_binding(),
                        self.buffers.piece_transforms.as_entire_binding(),
                        self.buffers.camera_4d_pos.as_entire_binding(),
                        self.buffers.projection_params.as_entire_binding(),
                        self.buffers.lighting_params.as_entire_binding(),
                        self.buffers.view_params.as_entire_binding(),
                        wgpu::BindingResource::TextureView(
                            &self.buffers.sticker_colors_texture.view,
                        ),
                        wgpu::BindingResource::TextureView(
                            &self.buffers.special_colors_texture.view,
                        ),
                    ],
                ],
            );

            let [r, g, b, _] = egui::Rgba::from(view_params.background_color).to_array();
            let [r, g, b] = [r as f64, g as f64, b as f64];
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_puzzle"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.buffers.out_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.buffers.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..wgpu::RenderPassDescriptor::default()
            });

            render_pass.set_pipeline(
                pipelines
                    .render_single_pass(self.model.ndim)
                    .ok_or_eyre("error fetching single-pass render pipeline")?,
            );
            for (index, bind_group) in &bind_groups {
                render_pass.set_bind_group(*index, bind_group, &[]);
            }
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
        view_params: &ViewParams,
    ) -> Result<()> {
        let mut opacity_buckets = self.init_buffers(encoder, view_params)?;
        if opacity_buckets.is_empty() {
            return Ok(());
        }

        // Make the textures the right size.
        let size = [view_params.width, view_params.height];
        self.buffers.first_pass_texture.set_size(size);
        self.buffers.depth_texture.set_size(size);
        self.buffers.out_texture.set_size(size);

        // Compute 3D vertex positions on the GPU.
        self.compute_3d_vertex_positions(encoder)?;

        // Compute incremental opacity for each bucket.
        for i in 0..opacity_buckets.len() - 1 {
            opacity_buckets[i].opacity -= opacity_buckets[i + 1].opacity;
        }

        // Render each bucket.
        let mut is_first = true;
        for bucket in opacity_buckets {
            let composite_params = GfxCompositeParams { outline_radius: 1 };

            self.render_polygon_ids(encoder, bucket.index_range, is_first)?;
            self.render_composite_puzzle(encoder, composite_params, bucket.opacity, is_first)?;

            is_first = false;
        }

        Ok(())
    }

    /// Initializes buffers and returns the number of triangles to draw.
    fn init_buffers(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view_params: &ViewParams,
    ) -> Result<Vec<OpacityBucket>> {
        if self.model.is_empty() {
            return Ok(vec![]);
        }

        // Write the projection parameters.
        let data = view_params.gfx_projection_params(self.model.ndim);
        self.gfx.queue.write_buffer(
            &self.buffers.projection_params,
            0,
            bytemuck::bytes_of(&data),
        );

        // Write the lighting parameters.
        let data = GfxLightingParams {
            dir: view_params.light_vector().into(),
            ambient: view_params.light_ambient_amount(),
        };
        self.gfx
            .queue
            .write_buffer(&self.buffers.lighting_params, 0, bytemuck::bytes_of(&data));

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

        // Write the view parameters.
        let scale = view_params.xy_scale()?;
        let data = GfxViewParams {
            scale: [scale.x, scale.y],
            align: [0.0, 0.0],

            clip_4d_backfaces: view_params.prefs.clip_4d_backfaces as i32,
            clip_4d_behind_camera: view_params.prefs.clip_4d_behind_camera as i32,
        };
        self.gfx
            .queue
            .write_buffer(&self.buffers.view_params, 0, bytemuck::bytes_of(&data));

        // Sort pieces into buckets by opacity and write triangle indices.
        let mut buffer_index = 0;
        let mut buckets: Vec<OpacityBucket> = vec![];
        let mut new_bucket = OpacityBucket {
            opacity: 1.0,
            index_range: 0..0,
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
                new_bucket = OpacityBucket {
                    opacity,
                    index_range: buffer_index..buffer_index,
                };
            }
            self.write_triangles_for_piece(encoder, piece, &mut buffer_index, view_params);
            new_bucket.index_range.end = buffer_index;
        }
        buckets.push(new_bucket);

        Ok(buckets)
    }
    fn write_triangles_for_piece(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        piece: Piece,
        destination_offset: &mut u32,
        view_params: &ViewParams,
    ) {
        if view_params.prefs.show_internals {
            self.write_triangles(
                encoder,
                &self.model.piece_internals_index_ranges[piece],
                destination_offset,
            );
        }
        for &sticker in &self.puzzle.pieces[piece].stickers {
            self.write_triangles(
                encoder,
                &self.model.sticker_index_ranges[sticker],
                destination_offset,
            );
        }
    }
    fn write_triangles(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        triangles_index_range: &Range<u32>,
        destination_offset: &mut u32,
    ) {
        const INDEX_SIZE: u64 = std::mem::size_of::<u32>() as u64;
        let start = triangles_index_range.start * 3;
        let len = triangles_index_range.len() * 3;
        encoder.copy_buffer_to_buffer(
            &self.model.triangles,
            start as u64 * INDEX_SIZE,
            &self.buffers.sorted_triangles,
            *destination_offset as u64 * INDEX_SIZE,
            len as u64 * INDEX_SIZE,
        );
        *destination_offset += len as u32;
    }

    fn compute_3d_vertex_positions(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        let pipelines = &self.gfx.pipelines;

        let bind_groups = pipelines.compute_transform_points_bind_groups.bind_groups(
            &self.gfx.device,
            &[
                &[
                    self.model.vertex_positions.as_entire_binding(),
                    self.model.u_tangents.as_entire_binding(),
                    self.model.v_tangents.as_entire_binding(),
                    self.model.sticker_shrink_vectors.as_entire_binding(),
                    self.model.piece_ids.as_entire_binding(),
                    self.model.facet_ids.as_entire_binding(),
                ],
                &[
                    self.model.piece_centroids.as_entire_binding(),
                    self.model.facet_centroids.as_entire_binding(),
                    self.model.facet_normals.as_entire_binding(),
                    self.buffers.vertex_3d_positions.as_entire_binding(),
                    self.buffers.vertex_lightings.as_entire_binding(),
                    self.buffers.vertex_culls.as_entire_binding(),
                ],
                &[
                    self.buffers.puzzle_transform.as_entire_binding(),
                    self.buffers.piece_transforms.as_entire_binding(),
                    self.buffers.camera_4d_pos.as_entire_binding(),
                    self.buffers.projection_params.as_entire_binding(),
                    self.buffers.lighting_params.as_entire_binding(),
                    self.buffers.view_params.as_entire_binding(),
                ],
            ],
        );

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute_3d_vertex_positions"),
            ..wgpu::ComputePassDescriptor::default()
        });
        compute_pass.set_pipeline(
            pipelines
                .compute_transform_points(self.model.ndim)
                .ok_or_eyre("error fetching transform points compute pipeline")?,
        );
        for (index, bind_group) in &bind_groups {
            compute_pass.set_bind_group(*index, bind_group, &[]);
        }

        dispatch_work_groups(&mut compute_pass, self.model.vertex_count as u32);
        Ok(())
    }

    fn render_polygon_ids(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        index_range: Range<u32>,
        clear: bool,
    ) -> Result<()> {
        let pipelines = &self.gfx.pipelines;

        let bind_groups = pipelines.render_polygon_ids_bind_groups.bind_groups(
            &self.gfx.device,
            &[&[], &[], &[self.buffers.view_params.as_entire_binding()]],
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_polygon_ids"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.buffers.first_pass_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match clear {
                        true => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        false => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.buffers.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: match clear {
                        true => wgpu::LoadOp::Clear(0.0),
                        false => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..wgpu::RenderPassDescriptor::default()
        });

        render_pass.set_pipeline(&pipelines.render_polygon_ids);
        for (index, bind_group) in &bind_groups {
            render_pass.set_bind_group(*index, bind_group, &[]);
        }
        render_pass.set_vertex_buffer(0, self.buffers.vertex_3d_positions.slice(..));
        render_pass.set_vertex_buffer(1, self.buffers.vertex_culls.slice(..));
        render_pass.set_vertex_buffer(2, self.buffers.vertex_lightings.slice(..));
        render_pass.set_vertex_buffer(3, self.model.polygon_ids.slice(..));
        render_pass.set_index_buffer(
            self.buffers.sorted_triangles.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(index_range, 0, 0..1);
        Ok(())
    }

    fn render_composite_puzzle(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        composite_params: GfxCompositeParams,
        alpha: f32,
        clear: bool,
    ) -> Result<()> {
        let pipelines = &self.gfx.pipelines;

        self.gfx.queue.write_buffer(
            &self.buffers.composite_params,
            0,
            bytemuck::bytes_of(&composite_params),
        );

        let bind_groups = pipelines.render_composite_puzzle_bind_groups.bind_groups(
            &self.gfx.device,
            &[
                &[],
                &[self.model.polygon_color_ids.as_entire_binding()],
                &[
                    wgpu::BindingResource::TextureView(&self.buffers.first_pass_texture.view),
                    wgpu::BindingResource::TextureView(&self.buffers.sticker_colors_texture.view),
                    wgpu::BindingResource::TextureView(&self.buffers.special_colors_texture.view),
                ],
                &[self.buffers.composite_params.as_entire_binding()],
            ],
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_composite_puzzle"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.buffers.out_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match clear {
                        true => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        false => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..wgpu::RenderPassDescriptor::default()
        });

        render_pass.set_pipeline(&pipelines.render_composite_puzzle);
        for (index, bind_group) in &bind_groups {
            render_pass.set_bind_group(*index, bind_group, &[]);
        }
        render_pass.set_vertex_buffer(0, self.buffers.composite_vertices.slice(..));
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
            }

            StaticPuzzleModel {
                ndim: u8 = mesh.ndim(),
                color_count: usize = mesh.color_count(),
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
                    ndim as usize * 4,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
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
                vertex_culls: wgpu::Buffer = gfx.create_buffer::<f32>(
                    label("vertex_culls"),
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
                 * TEXTURES
                 */
                /// First pass texture, which includes lighting, facet ID, and
                /// polygon ID for each pixel.
                first_pass_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("first_pass_texture"),
                    wgpu::TextureFormat::Rg32Sint,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Depth texture for use in the first pass.
                depth_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("depth_texture"),
                    wgpu::TextureFormat::Depth24PlusStencil8,
                    wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Output color texture.
                out_texture: CachedTexture2d = CachedTexture2d::new(
                    Arc::clone(&gfx),
                    label("color_texture"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                ),
                /// Sticker colors texture.
                sticker_colors_texture: CachedTexture1d = CachedTexture1d::new(
                    Arc::clone(&gfx),
                    label("sticker_colors"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                /// Special colors texture.
                special_colors_texture: CachedTexture1d = CachedTexture1d::new(
                    Arc::clone(&gfx),
                    label("special_colors"),
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
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
            camera_4d_pos: clone_buffer!(gfx, id, self.camera_4d_pos),
            view_params: clone_buffer!(gfx, id, self.view_params),
            composite_params: clone_buffer!(gfx, id, self.composite_params),
            vertex_3d_positions: clone_buffer!(gfx, id, self.vertex_3d_positions),
            vertex_lightings: clone_buffer!(gfx, id, self.vertex_lightings),
            vertex_culls: clone_buffer!(gfx, id, self.vertex_culls),
            composite_vertices: clone_buffer!(gfx, id, self.composite_vertices),
            sorted_triangles: clone_buffer!(gfx, id, self.sorted_triangles),

            first_pass_texture: clone_texture!(gfx, id, self.first_pass_texture),
            depth_texture: clone_texture!(gfx, id, self.depth_texture),
            out_texture: clone_texture!(gfx, id, self.out_texture),
            sticker_colors_texture: clone_texture!(gfx, id, self.sticker_colors_texture),
            special_colors_texture: clone_texture!(gfx, id, self.special_colors_texture),
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
struct OpacityBucket {
    opacity: f32,
    index_range: Range<u32>,
}

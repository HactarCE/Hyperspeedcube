//! Rendering logic.

use cgmath::{Matrix3, Matrix4, Rad, Transform};
use glium::{BackfaceCullingMode, DrawParameters, Surface};
use glium_glyph::glyph_brush::{
    rusttype, BuiltInLineBreaker, HorizontalAlign, Layout, SectionText, VariedSection,
    VerticalAlign,
};

mod cache;
mod shaders;
mod verts;

use crate::config::get_config;
use crate::puzzle::{traits::*, PuzzleController, PuzzleEnum};
use crate::DISPLAY;
use cache::FONT;
pub use verts::WireframeVertex;
use verts::*;

const CLIPPING_RADIUS: f32 = 2.0;

pub fn draw_puzzle(target: &mut glium::Frame, puzzle: &PuzzleEnum) {
    match puzzle {
        PuzzleEnum::Rubiks3D(cube) => _draw_puzzle(target, cube),
        PuzzleEnum::Rubiks4D(cube) => _draw_puzzle(target, cube),
    }
}

fn _draw_puzzle<P: PuzzleTrait>(target: &mut glium::Frame, puzzle: &PuzzleController<P>) {
    let config = get_config();

    let mut cache_ = cache::borrow_cache();
    let cache = &mut *cache_;

    let (target_w, target_h) = target.get_dimensions();
    let [r, g, b] = config.colors.background;
    target.clear_color_srgb_and_depth((r, g, b, 1.0), 1.0);

    let view_config = &config.view[P::TYPE];

    // Compute the model transform, which must be applied here on the CPU so that we
    // can do proper Z ordering.
    let model_transform = Matrix3::from_angle_x(Rad(view_config.theta))
        * Matrix3::from_angle_y(Rad(view_config.phi))
        / CLIPPING_RADIUS;
    // Compute the perspective transform, which we will apply on the GPU.
    let perspective_transform = {
        let min_dimen = std::cmp::min(target_w, target_h) as f32;
        let scale = min_dimen * view_config.scale;

        let xx = scale / target_w as f32;
        let yy = scale / target_h as f32;

        let fov = view_config.fov_3d;
        let zw = (fov / 2.0).tan(); // `tan(fov/2)` is the factor of how much the Z coordinate affects the XY coordinates.
        let ww = 1.0 + fov.signum() * zw;

        Matrix4::from_cols(
            cgmath::vec4(xx, 0.0, 0.0, 0.0),
            cgmath::vec4(0.0, yy, 0.0, 0.0),
            cgmath::vec4(0.0, 0.0, -1.0, -zw),
            cgmath::vec4(0.0, 0.0, 0.0, ww),
        )
    };
    let perspective_transform_matrix: [[f32; 4]; 4] = perspective_transform.into();

    let mut geo_params = GeometryParams {
        sticker_spacing: view_config.sticker_spacing,
        face_spacing: view_config.face_spacing,
        fov_4d: view_config.fov_4d,

        anim: puzzle.current_twist(),
        transform: model_transform,

        ..GeometryParams::default()
    };

    /*
     * Generate sticker vertices and write them to the VBO.
     */
    let stickers_vbo;
    {
        let face_colors = &config.colors.stickers[P::TYPE];
        // Each sticker has a `Vec<StickerVertex>` with all of its vertices and
        // a single f32 containing the average Z value.
        let mut verts_by_sticker: Vec<(Vec<WireframeVertex>, f32)> = vec![];
        for piece in P::Piece::iter() {
            for sticker in piece.stickers() {
                let alpha = if (puzzle.highlight_filter)(sticker) {
                    config.colors.opacity
                } else {
                    0.1
                };

                let [r, g, b] = face_colors[puzzle.displayed().get_sticker(sticker).idx()];
                geo_params.fill_color = [r, g, b, alpha];
                geo_params.line_color = geo_params.fill_color;
                if view_config.enable_outline {
                    geo_params.line_color[..3].copy_from_slice(&config.colors.outline);
                }

                if let Some(verts) = sticker.verts(geo_params) {
                    let avg_z = verts.iter().map(|v| v.avg_z()).sum::<f32>() / verts.len() as f32;
                    verts_by_sticker.push((verts, avg_z));
                }
            }
        }
        // Sort by average Z position for proper transparency.
        verts_by_sticker.sort_by(|(_, z1), (_, z2)| z1.partial_cmp(z2).unwrap());
        let verts: Vec<WireframeVertex> = verts_by_sticker
            .into_iter()
            .flat_map(|(verts, _)| verts)
            .collect();

        // Write sticker vertices to the VBO.
        stickers_vbo = cache.stickers_vbo.slice(verts.len());
        stickers_vbo.write(&verts);
    }

    let draw_params = DrawParameters {
        blend: glium::Blend::alpha_blending(),
        smooth: Some(glium::Smooth::Nicest),
        depth: glium::Depth {
            test: glium::DepthTest::IfLessOrEqual,
            write: true,
            ..glium::Depth::default()
        },
        backface_culling: BackfaceCullingMode::CullClockwise,
        ..DrawParameters::default()
    };

    /*
     * Draw puzzle geometry.
     */
    target
        .draw(
            stickers_vbo,
            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &shaders::OUTLINED,
            &uniform! {
                target_size: [target_w as f32, target_h as f32],
                transform: perspective_transform_matrix,
                wire_width: 1.0_f32,
            },
            &draw_params,
        )
        .expect("draw error");

    /*
     * Draw text labels.
     */
    if !puzzle.labels.is_empty() {
        let scale = rusttype::Scale::uniform(config.gfx.label_size);

        let mut backdrop_verts = vec![];

        let post_transform =
            Matrix4::from_nonuniform_scale(2.0 / target_w as f32, 2.0 / target_h as f32, 1.0);
        let pre_transform = post_transform.inverse_transform().unwrap() * perspective_transform;

        for (facet, text) in &puzzle.labels {
            // let screen_position

            let mut text_center = pre_transform * facet.projection_center(geo_params).extend(1.0);
            text_center /= text_center.w;
            text_center.z = -1.0;

            // Queue backdrop.
            let (w, h) = label_size(text, scale);
            for (dx, dy) in [
                (-0.5, -0.5),
                (0.5, -0.5),
                (-0.5, 0.5),
                (0.5, 0.5),
                (-0.5, 0.5),
                (0.5, -0.5),
            ] {
                let pos = text_center + cgmath::vec4(dx * w, dy * h, 0.0, 0.0);
                backdrop_verts.push(RgbaVertex {
                    pos: (post_transform * pos).into(),
                    color: config.colors.label_bg,
                });
            }

            // Queue text.
            cache.glyph_brush.queue(VariedSection {
                screen_position: (text_center.x, -text_center.y),
                // bounds: todo!(),
                z: text_center.z,
                layout: Layout::SingleLine {
                    line_breaker: BuiltInLineBreaker::default(),
                    h_align: HorizontalAlign::Center,
                    v_align: VerticalAlign::Center,
                },
                text: vec![SectionText {
                    text,
                    scale,
                    color: config.colors.label_fg,
                    ..Default::default()
                }],
                ..Default::default()
            });
        }

        // Draw backdrops.
        let backdrop_vbo = cache.label_backdrops_vbo.slice(backdrop_verts.len());
        backdrop_vbo.write(&backdrop_verts);
        target
            .draw(
                backdrop_vbo,
                glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &shaders::BASIC,
                &uniform! {},
                &draw_params,
            )
            .expect("draw error");

        // Draw text.
        cache
            .glyph_brush
            .draw_queued_with_transform(post_transform.into(), &**DISPLAY, target);
    }
}

fn label_size(text: &str, scale: rusttype::Scale) -> (f32, f32) {
    const PADDING: f32 = 16.0; // 16 pixels

    let layout = FONT.layout(text, scale, rusttype::Point::default());
    let bounding_boxes = layout.filter_map(|g| g.pixel_bounding_box());
    let min_x = bounding_boxes.clone().map(|b| b.min.x).min().unwrap_or(0);
    let min_y = bounding_boxes.clone().map(|b| b.min.y).max().unwrap_or(0);
    let max_x = bounding_boxes.clone().map(|b| b.max.x).min().unwrap_or(0);
    let max_y = bounding_boxes.clone().map(|b| b.max.y).max().unwrap_or(0);
    (
        (max_x - min_x) as f32 + PADDING,
        (max_y - min_y) as f32 + PADDING,
    )
}

//! Rendering logic.

use cgmath::{Deg, Matrix3, Matrix4, Vector3};
use egui::Rgba;
use glium::texture::SrgbTexture2d;
use glium::uniforms::MagnifySamplerFilter;
use glium::{BackfaceCullingMode, BlitTarget, DrawParameters, Surface};
use std::rc::Rc;

pub mod cache;
mod shaders;
mod verts;

use crate::app::App;
use crate::puzzle::traits::*;
pub use cache::PuzzleRenderCache;
pub use verts::WireframeVertex;

const CLIPPING_RADIUS: f32 = 2.0;

pub fn draw_puzzle(
    app: &mut App,
    width: u32,
    height: u32,
    pixels_per_point: f32,
) -> Rc<SrgbTexture2d> {
    let prefs = &app.prefs;
    let puzzle = &app.puzzle;
    let view_prefs = &prefs.view[puzzle.ty()];
    let puzzle_highlight = app.puzzle_selection();
    let cache = &mut app.render_cache;

    let mut target = cache.target.get(width, height, app.prefs.gfx.msaa as u32);
    let clear_color = Rgba::from(prefs.colors.background).to_tuple();
    target.clear_color_srgb_and_depth(clear_color, 1.0);

    // Compute the model transform, which must be applied here on the CPU so
    // that we can do proper Z ordering.
    let view_transform = Matrix3::from_angle_x(Deg(view_prefs.pitch))
        * Matrix3::from_angle_y(Deg(view_prefs.yaw))
        / CLIPPING_RADIUS;
    // Compute the perspective transform, which we will apply on the GPU.
    let perspective_transform = {
        let min_dimen = std::cmp::min(width, height) as f32;
        let scale = min_dimen * view_prefs.scale;

        let xx = scale / width as f32;
        let yy = scale / height as f32;

        let fov = view_prefs.fov_3d;
        let zw = (fov.to_radians() / 2.0).tan(); // `tan(fov/2)` is the factor of how much the Z coordinate affects the XY coordinates.
        let ww = 1.0 + fov.signum() * zw;

        // We've already normalize all puzzle coordinates, so the near and far
        // planes are z=-1 and z=+1 respectively. This makes constructing the
        // perspective transformation matrix relatively easy.
        //
        // NOTE: This call constructs a matrix from **columns**, so it appears
        // transposed in code.
        Matrix4::from_cols(
            cgmath::vec4(xx, 0.0, 0.0, 0.0),
            cgmath::vec4(0.0, yy, 0.0, 0.0),
            cgmath::vec4(0.0, 0.0, -1.0, -zw),
            cgmath::vec4(0.0, 0.0, 0.0, ww),
        )
    };
    let perspective_transform_matrix: [[f32; 4]; 4] = perspective_transform.into();

    let mut geo_params = GeometryParams {
        sticker_spacing: view_prefs.sticker_spacing,
        face_spacing: view_prefs.face_spacing,
        fov_4d: view_prefs.fov_4d,

        view_transform,

        ..GeometryParams::default()
    };
    geo_params.line_color = Rgba::from(prefs.colors.outline).to_array();

    let wire_width = if view_prefs.outline_thickness <= 0.0 {
        -1.0
    } else {
        pixels_per_point * view_prefs.outline_thickness
    };
    let light_direction = Matrix3::from_angle_y(Deg(view_prefs.light_yaw))
        * Matrix3::from_angle_x(Deg(view_prefs.light_pitch))
        * Vector3::unit_z();
    let light_direction: [f32; 3] = light_direction.into();

    /*
     * Generate sticker vertices and write them to the VBO.
     */
    let stickers_vbo;
    {
        // Each sticker has a `Vec<StickerVertex>` with all of its vertices and
        // a single f32 containing the average Z value.
        let mut verts_by_sticker: Vec<(Vec<WireframeVertex>, f32)> = vec![];
        for piece in puzzle.pieces() {
            geo_params.model_transform = puzzle.model_transform_for_piece(*piece);

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
                geo_params.fill_color = Rgba::from(sticker_color).to_array();
                geo_params.fill_color[3] = alpha;
                geo_params.line_color[3] = alpha;

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

    /*
     * Draw puzzle geometry.
     */
    target
        .draw(
            stickers_vbo,
            glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &shaders::OUTLINED,
            &glium::uniform! {
                target_size: [width as f32, height as f32],
                transform: perspective_transform_matrix,

                light_direction: light_direction,
                min_light: 1.0 - view_prefs.light_intensity,

                wire_width: wire_width,
            },
            &DrawParameters {
                blend: glium::Blend::alpha_blending(),
                smooth: Some(glium::Smooth::Nicest),
                depth: glium::Depth {
                    test: glium::DepthTest::IfLessOrEqual,
                    write: true,
                    ..glium::Depth::default()
                },
                backface_culling: BackfaceCullingMode::CullClockwise,
                ..DrawParameters::default()
            },
        )
        .expect("draw error");

    /*
     * Blit to non-multisampled buffer.
     */
    let (out_fbo, out_texture) = cache.out_tex.get(width, height);
    let blit_target = BlitTarget {
        left: 0,
        bottom: 0,
        width: width as i32,
        height: height as i32,
    };
    target.blit_whole_color_to(&out_fbo, &blit_target, MagnifySamplerFilter::Linear);

    out_texture
}

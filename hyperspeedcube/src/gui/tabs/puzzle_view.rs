use std::{ops::Range, sync::Arc};

use cgmath::{EuclideanSpace, SquareMatrix};
use egui::NumExt;
use hypermath::prelude::*;
use hyperpuzzle::{PerPiece, Piece, Puzzle, Sticker};
use parking_lot::Mutex;

use crate::{
    gfx::{DrawParams, GraphicsState, PuzzleRenderResources, PuzzleRenderer},
    preferences::{Preferences, ViewPreferences},
};

#[derive(Debug)]
pub struct PuzzleView {
    pub puzzle: Option<Arc<Puzzle>>,
    renderer: Option<Arc<Mutex<PuzzleRenderer>>>,
    gfx: Arc<GraphicsState>,

    pub rot: Isometry,
    zoom: f32,
    piece_face_opacities: PerPiece<f32>,
    piece_edge_opacities: PerPiece<f32>,

    rect: egui::Rect,

    highlighted_piece_types: [bool; 10],
}
impl PuzzleView {
    pub(crate) fn new(gfx: &Arc<GraphicsState>) -> Self {
        PuzzleView {
            puzzle: None,
            renderer: None,
            gfx: Arc::clone(gfx),

            rot: Isometry::ident(),
            zoom: 0.5,
            piece_face_opacities: PerPiece::default(),
            piece_edge_opacities: PerPiece::default(),

            rect: egui::Rect::NOTHING,

            highlighted_piece_types: [true; 10],
        }
    }
    pub(crate) fn set_puzzle(&mut self, puzzle: Arc<Puzzle>) {
        self.piece_face_opacities = puzzle.pieces.map_ref(|_, _| 1.0);
        self.piece_edge_opacities = puzzle.pieces.map_ref(|_, _| 1.0);
        self.zoom = 1.0;
        self.rot = Isometry::ident();

        self.puzzle = Some(Arc::clone(&puzzle));
        self.renderer = Some(Arc::new(Mutex::new(PuzzleRenderer::new(&self.gfx, puzzle))));
    }
    pub(crate) fn ndim(&self) -> Option<u8> {
        self.puzzle.as_ref().map(|puzzle| puzzle.ndim())
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, prefs: &Preferences) -> egui::Response {
        let dpi = ui.ctx().pixels_per_point();

        // Round rectangle to pixel boundary for crisp image.
        let mut pixels_rect = ui.available_rect_before_wrap();
        pixels_rect.set_left((dpi * pixels_rect.left()).ceil());
        pixels_rect.set_bottom((dpi * pixels_rect.bottom()).floor());
        pixels_rect.set_right((dpi * pixels_rect.right()).floor());
        pixels_rect.set_top((dpi * pixels_rect.top()).ceil());

        // Convert back from pixel coordinates to egui coordinates.
        let mut egui_rect = pixels_rect;
        *egui_rect.left_mut() /= dpi;
        *egui_rect.bottom_mut() /= dpi;
        *egui_rect.right_mut() /= dpi;
        *egui_rect.top_mut() /= dpi;

        self.rect = egui_rect;

        let r = ui.allocate_rect(egui_rect, egui::Sense::click_and_drag());

        if r.is_pointer_button_down_on() {
            // TODO: request focus not working?
            r.request_focus();
        }

        let (Some(puzzle), Some(renderer)) = (self.puzzle.clone(), self.renderer.clone()) else {
            // Hint to the user to load a puzzle.
            ui.allocate_ui_at_rect(egui_rect, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a puzzle from the puzzle list");
                });
            });
            return r;
        };

        let view_prefs = prefs.view(&puzzle).clone();

        let target_size = [
            pixels_rect.width() as u32 / view_prefs.downscale_rate,
            pixels_rect.height() as u32 / view_prefs.downscale_rate,
        ];

        let min_size = egui_rect.size().min_elem();
        const DRAG_SPEED: f32 = 5.0;
        let drag_delta = r.drag_delta() * DRAG_SPEED / min_size.abs();
        // Convert to higher precision before dividing.
        let scaled_drag_x = drag_delta.x as Float / self.zoom.at_least(1.0) as Float;
        let scaled_drag_y = drag_delta.y as Float / self.zoom.at_least(1.0) as Float;

        let scroll_delta = ui.input(|input| input.scroll_delta);
        let mut mouse_pos: Option<cgmath::Point2<f32>> = None;
        if r.hovered() {
            self.zoom *= (scroll_delta.y / 100.0).exp2();
            self.zoom = self.zoom.clamp(2.0_f32.powf(-6.0), 2.0_f32.powf(8.0));
            if let Some(pos) = r.hover_pos() {
                let mouse_pos_ndc = (pos - r.rect.center()) * 2.0 / r.rect.size();
                mouse_pos = Some(cgmath::point2(mouse_pos_ndc.x, -mouse_pos_ndc.y));
            }
        }

        let mut z_axis = 2;
        if ui.input(|input| input.modifiers.shift) {
            z_axis += 1;
        }
        if ui.input(|input| input.modifiers.alt) {
            z_axis += 2;
        };
        self.rot = Isometry::from_angle_in_axis_plane(0, z_axis, -scaled_drag_x)
            * Isometry::from_angle_in_axis_plane(1, z_axis, scaled_drag_y)
            * &self.rot;

        if r.has_focus() {
            ui.input(|input| {
                if input.key_pressed(egui::Key::Num1) {
                    self.highlighted_piece_types[1] ^= true;
                }
                if input.key_pressed(egui::Key::Num2) {
                    self.highlighted_piece_types[2] ^= true;
                }
                if input.key_pressed(egui::Key::Num3) {
                    self.highlighted_piece_types[3] ^= true;
                }
                if input.key_pressed(egui::Key::Num4) {
                    self.highlighted_piece_types[4] ^= true;
                }
                if input.key_pressed(egui::Key::Num5) {
                    self.highlighted_piece_types[5] ^= true;
                }
                if input.key_pressed(egui::Key::Num6) {
                    self.highlighted_piece_types[6] ^= true;
                }
                if input.key_pressed(egui::Key::Num7) {
                    self.highlighted_piece_types[7] ^= true;
                }
                if input.key_pressed(egui::Key::Num8) {
                    self.highlighted_piece_types[8] ^= true;
                }
                if input.key_pressed(egui::Key::Num9) {
                    self.highlighted_piece_types[9] ^= true;
                }
                if input.key_pressed(egui::Key::Num0) {
                    self.highlighted_piece_types[0] ^= true;
                }
            });
        }
        // self.piece_face_opacities = puzzle.pieces.map_ref(|piece, info| {
        //     let sticker_count = info.stickers.len();
        //     let fallback = self.highlighted_piece_types[0];
        //     match self
        //         .highlighted_piece_types
        //         .get(sticker_count)
        //         .unwrap_or(&fallback)
        //     {
        //         true => 1.0,
        //         false => 0.0,
        //     }
        // });
        self.piece_edge_opacities = puzzle.pieces.map_ref(|piece, info| 1.0);

        let vertex_3d_positions = renderer.lock().vertex_3d_positions();
        if vertex_3d_positions.is_none() {
            // Redraw each frame until the image is stable and we have
            // computed 3D vertex positions.
            ui.ctx().request_repaint();
        }

        // IIFE to mimic try_block
        let screen_space_mouse_pos = (|| {
            let p = mouse_pos?.to_vec();
            let s = DrawParams::compute_xy_scale(target_size, self.zoom).ok()?;
            Some(cgmath::point2(p.x / s.x, p.y / s.y))
        })();
        self.update_hover_state(screen_space_mouse_pos.filter(|_| !r.dragged()), prefs);

        let draw_params = DrawParams {
            prefs: view_prefs,
            target_size,
            mouse_pos: mouse_pos.unwrap_or_else(cgmath::Point2::origin).into(),
            rot: self.rot.clone(),
            zoom: self.zoom,
            background_color: prefs.colors.background,
            outlines_color: prefs.outlines.default_color,
            piece_face_opacities: self.piece_face_opacities.clone(),
            piece_edge_opacities: self.piece_edge_opacities.clone(),
        };

        let draw_prep = renderer.lock().prepare_draw(draw_params);

        if !draw_prep
            .vertex_3d_positions
            .is_some_and(|inner| inner.lock().is_some())
        {
            ui.ctx().request_repaint();
        }

        // Draw puzzle.
        let painter = ui.painter_at(egui_rect);
        painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
            egui_rect,
            PuzzleRenderResources {
                gfx: Arc::clone(&self.gfx),
                renderer: Arc::clone(&renderer),
            },
        ));

        r
    }

    fn update_hover_state(
        &mut self,
        screen_space_mouse_pos: Option<cgmath::Point2<f32>>,
        prefs: &Preferences,
    ) {
        let r = self.recompute_hover_state(screen_space_mouse_pos, prefs);
        let decay = 0.05;
        for (piece, opacity) in &mut self.piece_face_opacities {
            *opacity -= decay;
            *opacity = opacity.clamp(0.0, 1.0);
        }
        if let Some(r) = r {
            self.piece_face_opacities[r.piece] += decay + 0.005;
            self.piece_face_opacities[r.piece] = self.piece_face_opacities[r.piece].clamp(0.0, 1.0);
        }
    }

    fn recompute_hover_state(
        &mut self,
        screen_space_mouse_pos: Option<cgmath::Point2<f32>>,
        prefs: &Preferences,
    ) -> Option<HoverResponse> {
        let mouse_pos = screen_space_mouse_pos?;
        let puzzle = self.puzzle.as_ref()?;
        let vertex_3d_positions = self.renderer.as_ref()?.lock().vertex_3d_positions()?;

        let sticker_tri_ranges = puzzle
            .mesh
            .sticker_triangle_ranges
            .iter()
            .map(|(sticker, tri_range)| (puzzle.stickers[sticker].piece, Some(sticker), tri_range));

        let empty_internals_list = PerPiece::new();
        let internals_tri_ranges = if prefs.view(&puzzle).show_internals {
            &puzzle.mesh.piece_internals_triangle_ranges
        } else {
            &empty_internals_list
        }
        .iter()
        .map(|(piece, tri_range)| (piece, None, tri_range));

        itertools::chain(sticker_tri_ranges, internals_tri_ranges)
            .flat_map(|(piece, sticker, tri_range)| {
                triangle_hovers(
                    mouse_pos,
                    puzzle,
                    piece,
                    sticker,
                    tri_range,
                    &vertex_3d_positions,
                )
            })
            .max_by(|a, b| f32::total_cmp(&a.z, &b.z))
    }

    /// Adds an animation to the view settings animation queue.
    pub fn animate_from_view_settings(&mut self, view_prefs: ViewPreferences) {
        // TODO: animate
        // self.view_settings_anim.queue.push_back(view_prefs);
    }
}

/// Returns data about triangles that contain the screen-space point `p`.
fn triangle_hovers<'a>(
    p: cgmath::Point2<f32>,
    puzzle: &'a Puzzle,
    piece: Piece,
    sticker: Option<Sticker>,
    tri_range: &Range<u32>,
    vertex_3d_positions: &'a [cgmath::Vector4<f32>],
) -> impl 'a + Iterator<Item = HoverResponse> {
    puzzle.mesh.triangles[tri_range.start as usize..tri_range.end as usize]
        .iter()
        .filter_map(move |&vertex_ids| {
            let tri_verts @ [a, b, c] = vertex_ids.map(|i| vertex_3d_positions[i as usize]);
            let [qa, qb, qc] = triangle_hover_barycentric_coordinates(p, tri_verts)?;
            Some(HoverResponse {
                piece,
                sticker,
                z: qa * a.z + qb * b.z + qc * c.z,
            })
        })
}

/// Returns the perspective-correct barycentric coordinates for the point `p` in
/// triangle `tri`. The Z coordinate is ignored; only X, Y, and W are used.
///
/// Returns `None` if the point is not in the triangle.
///
/// This method uses the math described at
/// <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/visibility-problem-depth-buffer-depth-interpolation.html>
fn triangle_hover_barycentric_coordinates(
    p: cgmath::Point2<f32>,
    tri: [cgmath::Vector4<f32>; 3],
) -> Option<[f32; 3]> {
    // If any vertex is culled, skip it.
    if tri.iter().any(|p| p.w == 0.0) {
        return None;
    }

    let mut tri_2d = tri.map(|p| cgmath::point2(p.x / p.w, p.y / p.w));

    // Ensure the triangle is counterclockwise.
    let mut total_area = triangle_area_2x(tri_2d);
    let rev = total_area < 0.0;
    if rev {
        tri_2d.reverse();
        total_area = -total_area;
    }

    // Compute the barycentric coordinates in screen space.
    let [a, b, c] = tri_2d;
    let recip_total_area = total_area.recip();
    let qa = triangle_area_2x([p, b, c]) * recip_total_area;
    let qb = triangle_area_2x([a, p, c]) * recip_total_area;
    let qc = triangle_area_2x([a, b, p]) * recip_total_area;
    // If the point is inside the triangle ...
    (qa > 0.0 && qb > 0.0 && qc > 0.0).then(|| {
        let [a, b, c] = tri;
        // ... then compute the perspective-correct W value
        let w = qa * a.w + qb * b.w + qc * c.w;
        // ... and use that to compute perspective-correct barycentric
        //     coordinates.
        let mut out = [qa * w / a.w, qb * w / b.w, qc * w / c.w];
        if rev {
            out.reverse();
        }
        out
    })
}

fn triangle_area_2x([a, b, c]: [cgmath::Point2<f32>; 3]) -> f32 {
    cgmath::Matrix2::from_cols(b - a, b - c).determinant()
}

#[derive(Debug, Clone, PartialEq)]
struct HoverResponse {
    piece: Piece,
    sticker: Option<Sticker>,
    z: f32,
}

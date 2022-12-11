use ndpuzzle::math::VectorRef;
use winit::event::ModifiersState;

use crate::app::{App, AppEvent};

// experimental
const ENABLE_CONTEXT_MENU: bool = false;

pub fn build(ui: &mut egui::Ui, app: &mut App, puzzle_texture_id: egui::TextureId) {
    let dpi = ui.ctx().pixels_per_point();

    // Round rectangle to pixel boundary for crisp
    // image.
    let mut pixels_rect = ui.available_rect_before_wrap();
    pixels_rect.set_left((dpi * pixels_rect.left()).ceil());
    pixels_rect.set_bottom((dpi * pixels_rect.bottom()).floor());
    pixels_rect.set_right((dpi * pixels_rect.right()).floor());
    pixels_rect.set_top((dpi * pixels_rect.top()).ceil());

    // Update texture size.
    app.puzzle_texture_size = (pixels_rect.width() as u32, pixels_rect.height() as u32);

    // Convert back from pixel coordinates to egui
    // coordinates.
    let mut egui_rect = pixels_rect;
    *egui_rect.left_mut() /= dpi;
    *egui_rect.bottom_mut() /= dpi;
    *egui_rect.right_mut() /= dpi;
    *egui_rect.top_mut() /= dpi;

    let mut r = ui.put(
        egui_rect,
        egui::Image::new(puzzle_texture_id, egui_rect.size()).sense(egui::Sense::click_and_drag()),
    );

    // let mut any_circle_hovered = false;

    // let is_hover = |pos: egui::Pos2, rad: f32, hov_rad_mult: f32| -> bool {
    //     if let Some(hov) = r.hover_pos() {
    //         let dist = hov - pos;
    //         dist.dot(dist) <= rad * rad * hov_rad_mult * hov_rad_mult
    //     } else {
    //         false
    //     }
    // };

    // let mut draw_circle =
    //     |pos: egui::Pos2, rad: f32, hov_rad_mult: f32, color: egui::Color32| -> bool {
    //         let hovered = is_hover(pos, rad, hov_rad_mult);
    //         any_circle_hovered |= hovered;

    //         ui.painter().circle(
    //             pos,
    //             if hovered { rad * 1.5 } else { rad },
    //             if hovered { color } else { egui::Color32::WHITE },
    //             egui::Stroke {
    //                 width: 2.0,
    //                 color: egui::Color32::BLACK,
    //             },
    //         );

    //         hovered
    //     };

    // let project = |ndpos: ndpuzzle::math::Vector, rad: f32| {
    //     let pos = unsafe { &crate::VIEW_TRANSFORM } * ndpos * 1.5;

    //     let mut x = pos[0];
    //     let mut y = pos[1];
    //     let mut z = pos[2];
    //     let w = pos[3];

    //     let w_divisor = 1.0 + w * unsafe { crate::W_FACTOR_4D };
    //     x /= w_divisor;
    //     y /= w_divisor;
    //     z /= w_divisor;

    //     // Apply 3D perspective transformation.
    //     let z_divisor = 1.0 + (1.0 - z) * unsafe { crate::W_FACTOR_3D };
    //     x /= z_divisor;
    //     y /= z_divisor;

    //     let size = f32::min(r.rect.width(), r.rect.height());

    //     x = r.rect.center().x + x * size * 0.5;
    //     y = r.rect.center().y - y * size * 0.5;

    //     (egui::pos2(x, y), rad / w_divisor / z_divisor)
    // };

    // for ax in &app.puzzle.ty().twists.axes {
    //     let ndcenter = ax.reference_frame.reverse().matrix() * ndpuzzle::math::Vector::unit(0);
    //     let (center, rad) = project(ndcenter.clone(), 10.0);

    //     if is_hover(center, rad, 5.0) {
    //         let other_axis = app
    //             .puzzle
    //             .ty()
    //             .twists
    //             .axes
    //             .iter()
    //             .filter(|a| a.symbol != ax.symbol)
    //             .max_by_key(|a| {
    //                 ((a.reference_frame.reverse().matrix() * ndpuzzle::math::Vector::unit(0))
    //                     .dot(&ndcenter)
    //                     * 10000000.0) as i64
    //             })
    //             .unwrap();

    //         use itertools::Itertools;

    //         let mut transforms = app
    //             .puzzle
    //             .ty()
    //             .twists
    //             .directions
    //             .iter()
    //             .map(|dir| dir.transform.matrix())
    //             .collect_vec();

    //         let m = ndpuzzle::math::Matrix::EMPTY_IDENT;

    //         transforms.push(&m);

    //         let c2r2s = transforms
    //             .iter()
    //             .map(|xf| {
    //                 let v = (ax.reference_frame.reverse().matrix()
    //                     * xf.clone()
    //                     * other_axis.reference_frame.reverse().matrix()
    //                     * ndpuzzle::math::Vector::unit(0))
    //                 .normalize()
    //                 .unwrap()
    //                     * 0.25;

    //                 let dot = v.dot(ndcenter.normalize().unwrap());
    //                 let v = (v - ndcenter.normalize().unwrap() * dot)
    //                     .normalize()
    //                     .unwrap()
    //                     / 3.0;

    //                 project(&ndcenter + &v, 5.0)
    //             })
    //             .collect_vec();

    //         for &(c2, r2) in &c2r2s {
    //             ui.painter().line_segment(
    //                 [center, c2],
    //                 egui::Stroke {
    //                     width: 3.0,
    //                     color: egui::Color32::BLUE,
    //                 },
    //             );
    //         }

    //         draw_circle(center, rad, 5.0, egui::Color32::RED);

    //         for &(c2, r2) in &c2r2s {
    //             if draw_circle(c2, r2, 2.0, egui::Color32::BLUE) {
    //                 // println!("heya");
    //             }
    //         }
    //     } else {
    //         draw_circle(center, rad, 5.0, egui::Color32::RED);
    //     }
    // }

    // Update app cursor position.
    app.cursor_pos = r.hover_pos().map(|pos| {
        let p = (pos - egui_rect.min) / egui_rect.size();
        // Transform from egui to wgpu coordinates.
        cgmath::point2(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0)
    });

    let popup_state_id = egui::Id::new("puzzle_context_menu_state");
    let mut popup_was_open = ui.data().get_temp(popup_state_id).unwrap_or(false);
    if popup_was_open || app.pressed_modifiers() == ModifiersState::SHIFT {
        ui.data().insert_temp(popup_state_id, false);
        if ENABLE_CONTEXT_MENU {
            r = r.context_menu(|ui| {
                if !popup_was_open && app.puzzle.hovered_sticker().is_some() {
                    app.event(AppEvent::Click(egui::PointerButton::Secondary));
                }
                ui.data().insert_temp(popup_state_id, true);
                popup_was_open |= true;

                build_puzzle_context_menu(ui, app);
            });
        }
    }
    if popup_was_open {
        return; // Ignore click and drag events while the popup is open.
    }

    // Submit click events.
    for button in [
        egui::PointerButton::Primary,
        egui::PointerButton::Secondary,
        egui::PointerButton::Middle,
    ] {
        if r.clicked_by(button) {
            app.event(AppEvent::Click(button))
        }
    }

    // Submit drag events.
    if r.dragged() {
        app.event(AppEvent::Drag(r.drag_delta() / egui_rect.size().min_elem()))
    }
    if r.drag_released() {
        app.event(AppEvent::DragReleased);
    }

    // Submit scroll events.
    if ui.input().scroll_delta.length_sq() > 0.0 {
        app.event(AppEvent::Scroll(ui.input().scroll_delta));
    }

    // Show debug info for each sticker.
    #[cfg(debug_assertions)]
    if let Some(sticker) = app.puzzle.hovered_sticker() {
        let mut s = String::new();
        app.puzzle.displayed().sticker_debug_info(&mut s, sticker);
        if !s.is_empty() {
            egui::popup::show_tooltip_at_pointer(
                ui.ctx(),
                egui::Id::new("sticker_debug_info"),
                |ui| ui.label(s),
            );
        }
    }
}

fn build_puzzle_context_menu(_ui: &mut egui::Ui, _app: &mut App) {
    // let ty = app.puzzle.ty();

    // let selection = app.puzzle.selection().clone();
    // let colors: HashSet<_> = selection.iter().map(|&s| ty.info(s).color).collect();
    // let piece_types: HashSet<_> = selection
    //     .iter()
    //     .map(|&s| ty.info(ty.info(s).piece).piece_type)
    //     .collect();

    // if ui.button("Show all pieces").clicked() {
    //     app.puzzle.show_stickers(|_| true);
    // }
    // ui.separator();
    // if ui.button("Show only this color").clicked() {
    //     app.puzzle.hide_stickers(|s| {
    //         !colors.iter().all(|&c| {
    //             ty.info(ty.info(s).piece)
    //                 .stickers
    //                 .iter()
    //                 .any(|&s| ty.info(s).color == c)
    //         })
    //     });
    //     ui.close_menu();
    // }
    // if ui.button("Show only this piece type").clicked() {
    //     app.puzzle
    //         .hide_stickers(|s| !piece_types.contains(&ty.info(ty.info(s).piece).piece_type));
    //     ui.close_menu();
    // }
    // if ui.button("Show only this piece").clicked() {
    //     app.puzzle.hide_stickers(|_| true);
    //     for &sticker in &selection {
    //         app.puzzle
    //             .show_stickers(|s| ty.info(s).piece == ty.info(sticker).piece);
    //     }
    //     ui.close_menu();
    // }
    // ui.separator();
    // if ui.button("Hide this color").clicked() {
    //     app.puzzle.hide_stickers(|s| {
    //         ty.info(ty.info(s).piece)
    //             .stickers
    //             .iter()
    //             .any(|&s| colors.contains(&ty.info(s).color))
    //     });
    //     ui.close_menu();
    // }
    // if ui.button("Hide this piece type").clicked() {
    //     app.puzzle
    //         .hide_stickers(|s| piece_types.contains(&ty.info(ty.info(s).piece).piece_type));
    //     ui.close_menu();
    // }
    // if ui.button("Hide this piece").clicked() {
    //     for &sticker in &selection {
    //         app.puzzle
    //             .hide_stickers(|s| ty.info(s).piece == ty.info(sticker).piece);
    //     }
    //     ui.close_menu();
    // }
}

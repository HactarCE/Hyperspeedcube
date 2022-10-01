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

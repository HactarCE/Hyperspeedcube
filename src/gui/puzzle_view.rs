use crate::app::{App, AppEvent};

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

    let r = ui.put(
        egui_rect,
        egui::Image::new(puzzle_texture_id, egui_rect.size()).sense(egui::Sense::click_and_drag()),
    );

    // Update app cursor position.
    app.cursor_pos = r.hover_pos().map(|pos| {
        let p = (pos - egui_rect.min) / egui_rect.size();
        // Transform from egui to wgpu coordinates.
        cgmath::point2(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0)
    });

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
        use crate::puzzle::traits::*;

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

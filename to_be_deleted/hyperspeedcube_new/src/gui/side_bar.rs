pub fn build(
    ctx: &egui::Context,
    title: &str,
    is_open: &mut bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    if !*is_open {
        return;
    }

    egui::SidePanel::left(unique_id!(title)).show(ctx, |ui| {
        ui.style_mut().wrap = Some(false);

        let heading_response = ui.heading(title);

        add_contents(ui);

        // Build a "close" button (stolen from egui source code). Do this after
        // constructing all the other contents so that it knows how wide the
        // panel is.
        let button_size = egui::Vec2::splat(ui.spacing().icon_width);
        let rect = egui::Rect::from_center_size(
            egui::pos2(
                ui.max_rect().right() - 0.5 * button_size.x,
                heading_response.rect.center().y,
            ),
            button_size,
        );
        let close_id = unique_id!(title);
        let r = ui.interact(rect, close_id, egui::Sense::click());
        ui.expand_to_include_rect(r.rect);

        let visuals = ui.style().interact(&r);
        let rect = rect.shrink(2.0).expand(visuals.expansion);
        let stroke = visuals.fg_stroke;
        ui.painter() // paints \
            .line_segment([rect.left_top(), rect.right_bottom()], stroke);
        ui.painter() // paints /
            .line_segment([rect.right_top(), rect.left_bottom()], stroke);
        if r.clicked() {
            *is_open = false;
        }
    });
}

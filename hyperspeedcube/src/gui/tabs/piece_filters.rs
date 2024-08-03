use egui::{epaint, NumExt};

use crate::{app::App, update_styles};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.has_active_puzzle_view(), |ui| {
        app.with_active_puzzle_view(|p| {
            let puz = p.puzzle();
            let mut changed = false;

            egui::CollapsingHeader::new("Colors")
                .default_open(true)
                .show(ui, |ui| {
                    let states_iter = p.view.filters.colors.iter_values_mut();
                    let rgbs_iter = p
                        .view
                        .colors
                        .value
                        .values()
                        .map(|color| app.prefs.color_palette.get(color).unwrap_or_default());
                    let color_infos_iter = puz.colors.list.iter_values();
                    for ((state, rgb), color_info) in
                        states_iter.zip(rgbs_iter).zip(color_infos_iter)
                    {
                        ui.horizontal(|ui| {
                            let color32 = crate::util::rgb_to_egui_color32(rgb);
                            egui::color_picker::show_color(ui, color32, ui.spacing().interact_size);
                            changed |= filter_checkbox(ui, state, &color_info.name, true).changed();
                        })
                        .inner
                    }
                });

            egui::CollapsingHeader::new("Piece types")
                .default_open(true)
                .show(ui, |ui| {
                    let states_iter = p.view.filters.piece_types.iter_values_mut();
                    let piece_type_infos_iter = puz.piece_types.iter_values();
                    for (state, piece_type_info) in states_iter.zip(piece_type_infos_iter) {
                        changed |=
                            filter_checkbox(ui, state, &piece_type_info.name, false).changed();
                    }
                });

            if changed {
                p.view.notify_filters_changed();
            }

            ui.separator();
        })
    });
}

fn next_filter_setting(current: Option<bool>, allow_checked: bool) -> Option<bool> {
    if !allow_checked {
        return current.is_none().then_some(false);
    }

    match current {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}
fn prev_filter_setting(current: Option<bool>, allow_checked: bool) -> Option<bool> {
    if !allow_checked {
        return current.is_none().then_some(false);
    }

    match current {
        None => Some(false),
        Some(false) => Some(true),
        Some(true) => None,
    }
}

fn filter_checkbox(
    ui: &mut egui::Ui,
    state: &mut Option<bool>,
    text: impl Into<egui::WidgetText>,
    allow_checked: bool,
) -> egui::Response {
    let text = text.into();

    let spacing = &ui.spacing();
    let icon_width = spacing.icon_width;
    let icon_spacing = spacing.icon_spacing;

    let (galley, mut desired_size) = if text.is_empty() {
        (None, egui::vec2(icon_width, 0.0))
    } else {
        let total_extra = egui::vec2(icon_width + icon_spacing, 0.0);

        let wrap_width = ui.available_width() - total_extra.x;
        let galley = text.into_galley(ui, None, wrap_width, egui::TextStyle::Button);

        let mut desired_size = total_extra + galley.size();
        desired_size = desired_size.at_least(spacing.interact_size);

        (Some(galley), desired_size)
    };

    desired_size = desired_size.at_least(egui::Vec2::splat(spacing.interact_size.y));
    desired_size.y = desired_size.y.max(icon_width);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *state = next_filter_setting(*state, allow_checked);
        response.mark_changed();
    } else if response.secondary_clicked() {
        *state = prev_filter_setting(*state, allow_checked);
        response.mark_changed();
    }
    response.widget_info(|| {
        let widget_type = egui::WidgetType::Checkbox;
        let label = galley.as_ref().map_or("", |x| x.text());
        match *state {
            None => egui::WidgetInfo::labeled(widget_type, label),
            Some(checked_state) => egui::WidgetInfo::selected(widget_type, checked_state, label),
        }
    });

    if ui.is_rect_visible(rect) {
        // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
        let visuals = ui.style().interact(&response);
        let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
        ui.painter().add(epaint::RectShape::new(
            big_icon_rect.expand(visuals.expansion),
            visuals.rounding,
            visuals.bg_fill,
            visuals.bg_stroke,
        ));

        match *state {
            None => (),
            Some(true) => {
                // Check mark
                ui.painter().add(egui::Shape::line(
                    vec![
                        egui::pos2(small_icon_rect.left(), small_icon_rect.center().y),
                        egui::pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                        egui::pos2(small_icon_rect.right(), small_icon_rect.top()),
                    ],
                    visuals.fg_stroke,
                ));
            }
            Some(false) => {
                // X mark
                ui.painter().add(egui::Shape::line_segment(
                    [small_icon_rect.left_top(), small_icon_rect.right_bottom()],
                    visuals.fg_stroke,
                ));
                ui.painter().add(egui::Shape::line_segment(
                    [small_icon_rect.left_bottom(), small_icon_rect.right_top()],
                    visuals.fg_stroke,
                ));
            }
        }
        if let Some(galley) = galley {
            let text_pos = egui::pos2(
                rect.min.x + icon_width + icon_spacing,
                rect.center().y - 0.5 * galley.size().y,
            );
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }
    }

    response
}

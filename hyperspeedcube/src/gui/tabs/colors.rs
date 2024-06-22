use crate::{
    app::App,
    gui::components::{HintWidget, BIG_ICON_BUTTON_SIZE, SMALL_ICON_BUTTON_SIZE},
    preferences::SavedColor,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.set_enabled(app.has_active_puzzle());

    ui.group(|ui| {
        ui.collapsing("Puzzle colors", |ui| {
            if let Some(ty) = app.active_puzzle_type() {
                for (id, face_color) in &ty.colors {
                    ui.horizontal(|ui| {
                        ui.label(&face_color.name);
                        if let Some(default) = &face_color.default_color {
                            ui.label(default);
                        }
                    });
                }
            } else {
                ui.label("No puzzle loaded");
            }
        });
    });

    let mut changed = false;

    ui.group(|ui| {});

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Global color palette");
            HintWidget::show(ui, |ui| {
                ui.heading("Global color palette");
                ui.label("TODO: explain the global color palette!");
            })
        });
        ui.separator();
        ui.with_layout(
            egui::Layout::bottom_up(egui::Align::Center).with_cross_justify(true),
            |ui| {
                // ui.set_min_size(ui.available_size());

                let r = &ui.add(egui::Button::new("Add color").min_size(BIG_ICON_BUTTON_SIZE));
                if r.clicked() {
                    // Add a new color
                    app.prefs.colors.push(SavedColor {
                        name: "custom color #1".to_string(),
                        rgb: [127, 127, 127],
                    });
                    changed = true;
                }

                ui.add_space(ui.spacing().item_spacing.x);

                if app.prefs.colors.is_empty() {
                    app.prefs.colors = [
                        ([255, 255, 255], "white"),
                        ([255, 0, 0], "red"),
                        ([255, 255, 0], "yellow"),
                        ([0, 255, 0], "green"),
                        ([0, 255, 255], "cyan"),
                        ([0, 0, 255], "blue"),
                        ([255, 0, 255], "magenta"),
                    ]
                    .map(|([r, g, b], name)| SavedColor {
                        name: name.to_string(),
                        rgb: [r, g, b],
                    })
                    .to_vec();
                }
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    egui::ScrollArea::vertical().show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        app.prefs.colors.len(),
                        |ui, range| {
                            for i in range {
                                if let Some(saved_color) = app.prefs.colors.get_mut(i) {
                                    ui.horizontal(|ui| {
                                        let r = ui.color_edit_button_srgb(&mut saved_color.rgb);
                                        changed |= r.changed();
                                        if i < 7 {
                                            ui.label(&saved_color.name);
                                        } else {
                                            let r =
                                                egui::TextEdit::singleline(&mut saved_color.name)
                                                    .show(ui);
                                            changed |= r.response.changed();
                                        }
                                    });
                                }
                            }
                        },
                    );
                });
            },
        );
    });

    app.prefs.needs_save |= changed;
}

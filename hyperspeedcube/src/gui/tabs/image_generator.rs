use std::path::Path;

use eyre::{OptionExt, Result};

use crate::app::App;
use crate::gui::util::EguiTempValue;
use crate::L;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let l = L.image_generator;

    let has_active_puzzle = app.active_puzzle.has_puzzle();

    let mut changed = false;

    let status = EguiTempValue::new(ui);
    let can_save_screenshot = has_active_puzzle
        && app.prefs.image_generator.dir.is_some()
        && !app.prefs.image_generator.filename.is_empty();
    ui.add_enabled_ui(can_save_screenshot, |ui| {
        let mut r = ui
            .allocate_ui_with_layout(
                egui::vec2(150.0, 30.0),
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| ui.button(l.save_image),
            )
            .inner;

        if app.prefs.image_generator.dir.is_none() {
            r = r.on_disabled_hover_text(l.errors.no_output_dir);
        } else if app.prefs.image_generator.filename.is_empty() {
            r = r.on_disabled_hover_text(l.errors.no_output_filename);
        } else if !has_active_puzzle {
            r = r.on_disabled_hover_text(l.errors.no_active_puzzle);
        }

        if r.clicked() {
            if let Some(dir) = &app.prefs.image_generator.dir {
                let file_path = dir.join(&app.prefs.image_generator.filename);
                status.set(Some(
                    if file_path.is_file() && !matches!(status.get(), Some(Status::Exists)) {
                        Status::Exists
                    } else {
                        match save_screenshot(app, &file_path) {
                            Ok(()) => Status::Success,
                            Err(e) => Status::Error(e.to_string()),
                        }
                    },
                ));
            }
        }

        match status.get() {
            None | Some(Status::None) => ui.label(""),
            Some(Status::Exists) => {
                ui.colored_label(ui.visuals().warn_fg_color, l.already_exists_confirm)
            }
            Some(Status::Success) => ui.label(L.statuses.saved),
            Some(Status::Error(e)) => ui.colored_label(
                ui.visuals().error_fg_color,
                L.statuses.error.with(&e.to_string()),
            ),
        };
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(l.browse).clicked() {
            let mut dialog = rfd::FileDialog::new().set_title(l.select_output_dir);
            if let Some(dir) = &app.prefs.image_generator.dir {
                dialog = dialog.set_directory(dir);
            }
            if let Some(new_dir) = dialog.pick_folder() {
                app.prefs.image_generator.dir = Some(new_dir);
                changed = true;
            }
        }
        if let Some(dir_path) = &app.prefs.image_generator.dir {
            ui.label(dir_path.to_string_lossy());
        }
    });

    changed |= ui
        .add(
            egui::TextEdit::singleline(&mut app.prefs.image_generator.filename)
                .desired_width(200.0)
                .hint_text("screenshot.png"),
        )
        .changed();

    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut app.prefs.image_generator.width).range(1..=2048));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut app.prefs.image_generator.height).range(1..=2048));
    });

    #[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
    enum Status {
        #[default]
        None,
        Exists,
        Success,
        Error(String),
    }

    app.prefs.needs_save |= changed;
}

fn save_screenshot(app: &mut App, path: &Path) -> Result<()> {
    app.active_puzzle
        .with_view(|view| {
            Ok(view
                .renderer
                .screenshot(
                    app.prefs.image_generator.width,
                    app.prefs.image_generator.height,
                )?
                .save(path)?)
        })
        .ok_or_eyre("no active puzzle")?
}

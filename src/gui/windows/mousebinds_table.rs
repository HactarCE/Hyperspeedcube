use crate::app::App;
use crate::commands::PuzzleMouseCommand;
use crate::gui::util::FancyComboBox;
use crate::gui::widgets;
use crate::preferences::{MouseButton, Mousebind};

pub(super) struct MousebindsTable<'a> {
    app: &'a mut App,
}
impl<'a> MousebindsTable<'a> {
    pub(super) fn new(app: &'a mut App) -> Self {
        Self { app }
    }
}

impl egui::Widget for MousebindsTable<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;

        let mut mousebinds = &mut self.app.prefs.mousebinds;

        let yaml_editor = widgets::PlaintextYamlEditor { id: unique_id!() };

        let mut r = yaml_editor.show(ui, mousebinds).unwrap_or_else(|| {
            ui.scope(|ui| {
                let mut header_rect = None;
                let mut mouse_button_x_pos = None;
                let mut modifiers_x_pos = None;
                let mut command_x_pos = None;

                ui.horizontal(|ui| {
                    if widgets::big_icon_button(ui, "✏", "Edit as plaintext").clicked() {
                        yaml_editor.set_active(ui, mousebinds);
                    }

                    if widgets::big_icon_button(ui, "➕", "Add a new mousebind").clicked() {
                        mousebinds.push(Mousebind::default());
                        changed = true;
                    };

                    let mut rect = ui.min_rect();
                    rect.set_width(ui.available_width());
                    header_rect = Some(rect);
                });

                ui.separator();

                egui::ScrollArea::new([false, true]).show(ui, |ui| {
                    let id = unique_id!();
                    let r = widgets::ReorderableList::new(id, &mut mousebinds).show(
                        ui,
                        |ui, idx, mousebind| {
                            mouse_button_x_pos = Some(ui.cursor().left());

                            let mut r = ui.add(FancyComboBox {
                                combo_box: egui::ComboBox::from_id_source(unique_id!(idx)),
                                selected: &mut mousebind.button,
                                options: vec![
                                    (MouseButton::Left, "Left".into()),
                                    (MouseButton::Right, "Right".into()),
                                    (MouseButton::Middle, "Middle".into()),
                                ],
                            });

                            modifiers_x_pos = Some(ui.cursor().left());

                            for ch in key_names::MODIFIERS_ORDER.chars() {
                                let (mut_bool, name) = match ch {
                                    'c' => (&mut mousebind.ctrl, key_names::CTRL_STR),
                                    's' => (&mut mousebind.shift, key_names::SHIFT_STR),
                                    'a' => (&mut mousebind.alt, key_names::ALT_STR),
                                    'm' => (&mut mousebind.logo, key_names::LOGO_STR),
                                    _ => continue, // unreachable
                                };
                                r |= ui.toggle_value(mut_bool, name);
                            }

                            command_x_pos = Some(ui.cursor().left());

                            r |= ui.add(FancyComboBox {
                                combo_box: egui::ComboBox::from_id_source(unique_id!(idx)),
                                selected: &mut mousebind.command,
                                options: vec![
                                    (PuzzleMouseCommand::None, "None".into()),
                                    (PuzzleMouseCommand::TwistCw, "Twist clockwise".into()),
                                    (
                                        PuzzleMouseCommand::TwistCcw,
                                        "Twist counterclockwise".into(),
                                    ),
                                    (PuzzleMouseCommand::Recenter, "Recenter".into()),
                                    (PuzzleMouseCommand::SelectPiece, "Select piece".into()),
                                ],
                            });

                            r
                        },
                    );
                    changed |= r.changed();

                    ui.allocate_space(egui::vec2(1.0, 200.0));
                });

                if let Some(mut rect) = header_rect {
                    if let Some(x) = mouse_button_x_pos {
                        rect.min.x = x;
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.horizontal_centered(|ui| ui.strong("Button"))
                        });
                    }
                    if let Some(x) = modifiers_x_pos {
                        rect.min.x = x;
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.horizontal_centered(|ui| ui.strong("Modifiers"))
                        });
                    }
                    if let Some(x) = command_x_pos {
                        rect.min.x = x;
                        ui.allocate_ui_at_rect(rect, |ui| {
                            ui.horizontal_centered(|ui| ui.strong("Command"))
                        });
                    }
                }

                // TODO: what's this for?
                // if ui.available_height() > 0.0 {
                //     ui.allocate_space(ui.available_size());
                // }
            })
            .response
        });

        if changed {
            r.mark_changed();
        }
        r
    }
}

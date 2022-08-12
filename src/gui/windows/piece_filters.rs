use crate::app::App;
use crate::gui::util::{self, ResponseExt};
use crate::puzzle::{traits::*, Face, Piece, PieceType};

const MIN_WIDTH: f32 = 300.0;

pub fn cleanup(app: &mut App) {
    app.puzzle.set_preview_hidden(|_| None);
}

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    app.puzzle.set_preview_hidden(|_| None);

    let puzzle_type = app.puzzle.ty();

    ui.set_min_width(MIN_WIDTH);

    let mut changed = false;

    let prefs = &mut app.prefs;
    changed |= resettable_opacity_dragvalue!(ui, prefs.opacity.hidden, "Hidden").changed();
    let r = ui
        .add(util::CheckboxWithReset {
            label: "Unhide grip",
            value: &mut prefs.opacity.unhide_grip,
            reset_value: crate::preferences::DEFAULT_PREFS.opacity.unhide_grip,
        })
        .on_hover_explanation(
            "",
            "When enabled, gripping a face will temporarily \
         disable piece filters.",
        );
    changed |= r.changed();

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }

    ui.separator();

    PieceFilterWidget::new("everything", |_| true)
        .no_all_except()
        .show(ui, app);

    ui.collapsing("Types", |ui| {
        for (i, piece_type) in puzzle_type.piece_types().iter().enumerate() {
            PieceFilterWidget::new(&format!("{}s", piece_type.name), move |piece| {
                puzzle_type.info(piece).piece_type == PieceType(i as _)
            })
            .show(ui, app);
        }
    });

    ui.collapsing("Colors", |ui| {
        ui.set_enabled(!app.prefs.colors.blindfold);

        let face_colors = app.prefs.colors.face_colors_list(app.puzzle.ty());

        let colors_selection_id = unique_id!();
        let mut selected_colors: Vec<bool> =
            ui.data().get_temp(colors_selection_id).unwrap_or_default();
        selected_colors.resize(app.puzzle.faces().len(), false);

        for i in 0..puzzle_type.faces().len() {
            PieceFilterWidget::new("pieces with this color", move |piece| {
                let mut stickers = puzzle_type.info(piece).stickers.iter();
                stickers.any(|&sticker| puzzle_type.info(sticker).color == Face(i as _))
            })
            .label_ui(|ui: &mut egui::Ui| {
                ui.horizontal(|ui| {
                    egui::color_picker::show_color(ui, face_colors[i], ui.spacing().interact_size);
                    ui.checkbox(&mut selected_colors[i], "");
                })
                .response
            })
            .show(ui, app);
        }

        ui.add_enabled_ui(selected_colors.contains(&true), |ui| {
            PieceFilterWidget::new("pieces with all these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                selected_colors.iter().enumerate().all(|(i, selected)| {
                    !selected
                        || stickers
                            .iter()
                            .any(|&s| puzzle_type.info(s).color == Face(i as _))
                })
            })
            .show(ui, app);
            PieceFilterWidget::new("pieces with any of these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                stickers
                    .iter()
                    .any(|&s| selected_colors[puzzle_type.info(s).color.0 as usize])
            })
            .show(ui, app);
            PieceFilterWidget::new("pieces with only these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                stickers
                    .iter()
                    .all(|&s| selected_colors[puzzle_type.info(s).color.0 as usize])
            })
            .show(ui, app);
        });

        ui.data().insert_temp(colors_selection_id, selected_colors);
    });

    ui.collapsing("Presets", |ui| ui.label("spoookyyy..."));
}

#[must_use]
struct PieceFilterWidget<'a, W, P> {
    name: &'a str,
    label_ui: W,
    all_except: bool,
    predicate: P,
}
impl<'a, P> PieceFilterWidget<'a, egui::Label, P>
where
    P: Copy + FnMut(Piece) -> bool,
{
    fn new(name: &'a str, predicate: P) -> Self {
        let mut s = name.to_string();
        s[0..1].make_ascii_uppercase();

        Self {
            name,
            label_ui: egui::Label::new(s).sense(egui::Sense::click()),
            all_except: true,
            predicate,
        }
    }
}
impl<'a, W, P> PieceFilterWidget<'a, W, P>
where
    W: egui::Widget,
    P: Copy + FnMut(Piece) -> bool,
{
    fn label_ui<W2>(self, label_ui: W2) -> PieceFilterWidget<'a, W2, P> {
        PieceFilterWidget {
            name: self.name,
            label_ui,
            all_except: self.all_except,
            predicate: self.predicate,
        }
    }

    fn no_all_except(mut self) -> Self {
        self.all_except = false;
        self
    }

    fn show(mut self, ui: &mut egui::Ui, app: &mut App) -> egui::Response {
        let puzzle = &mut app.puzzle;

        ui.horizontal(|ui| {
            let r = ui.add(self.label_ui);
            if r.hovered() {
                puzzle.set_preview_hidden(|piece| Some(!(self.predicate)(piece)));
            }

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                let r = ui.add_enabled(
                    !puzzle.are_all_shown(self.predicate),
                    |ui: &mut egui::Ui| ui.button("👁").on_hover_text(format!("Show {}", self.name)),
                );
                if r.hovered() {
                    puzzle.set_preview_hidden(|piece| (self.predicate)(piece).then_some(false));
                }
                if r.clicked() {
                    puzzle.show(self.predicate);
                }

                let r = ui.add_enabled(
                    !puzzle.are_all_hidden(self.predicate),
                    |ui: &mut egui::Ui| {
                        ui.button("ｘ").on_hover_text(format!("Hide {}", self.name))
                    },
                );
                if r.hovered() {
                    puzzle.set_preview_hidden(|piece| (self.predicate)(piece).then_some(true));
                }
                if r.clicked() {
                    puzzle.hide(self.predicate);
                }

                if self.all_except {
                    let r = ui.add_enabled(
                        !puzzle.are_all_hidden(|p| !(self.predicate)(p)),
                        |ui: &mut egui::Ui| {
                            ui.button("❎")
                                .on_hover_text(format!("Hide all except {}", self.name))
                        },
                    );
                    if r.hovered() {
                        puzzle
                            .set_preview_hidden(|piece| (!(self.predicate)(piece)).then_some(true));
                    }
                    if r.clicked() {
                        puzzle.hide(|p| !(self.predicate)(p));
                    }
                }
            })
        })
        .response
    }
}

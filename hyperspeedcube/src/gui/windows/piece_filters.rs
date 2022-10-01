use bitvec::vec::BitVec;
use std::sync::Arc;

use crate::app::App;
use crate::gui::{util, widgets};
use crate::preferences::{PieceFilter, DEFAULT_PREFS};
use crate::puzzle::{traits::*, Face, PieceInfo, PieceType};

const MIN_WIDTH: f32 = 300.0;

fn piece_subset(ty: &PuzzleType, predicate: impl FnMut(&PieceInfo) -> bool) -> BitVec {
    ty.pieces.iter().map(predicate).collect()
}
macro_rules! piece_subset_from_sticker_colors {
    ($puzzle_ty:expr, |$color_iter:ident| $predicate:expr $(,)?) => {{
        // This is a macro instead of a function because I don't know how to
        // write the type of the predicate closure except as `impl FnMut(impl
        // Iterator<Item=Face>) -> bool`, which isn't allowed.
        let ty = &$puzzle_ty;
        ty.pieces
            .iter()
            .map(|piece| {
                #[allow(unused_mut)]
                let mut $color_iter = piece.stickers.iter().map(|&sticker| ty.info(sticker).color);
                $predicate
            })
            .collect()
    }};
}

pub fn cleanup(app: &mut App) {
    app.puzzle.set_visible_pieces_preview(None, None);
}

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    app.puzzle.set_visible_pieces_preview(None, None);

    let puzzle_type = Arc::clone(app.puzzle.ty());

    ui.set_min_width(MIN_WIDTH);

    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.opacity,
        defaults: &DEFAULT_PREFS.opacity,
        changed: &mut changed,
    };

    prefs_ui.percent("Hidden", access!(.hidden));
    crate::gui::prefs::build_unhide_grip_checkbox(&mut prefs_ui);

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }

    ui.separator();

    PieceFilterWidget::new_uppercased("everything", piece_subset(&puzzle_type, |_| true))
        .no_all_except()
        .show(ui, app);

    ui.collapsing("Types", |ui| {
        for (i, piece_type) in puzzle_type.piece_types.iter().enumerate() {
            PieceFilterWidget::new_uppercased(
                &format!("{}s", piece_type.name),
                piece_subset(&puzzle_type, move |piece| {
                    piece.piece_type == PieceType(i as _)
                }),
            )
            .show(ui, app);
        }
    });

    ui.collapsing("Colors", |ui| {
        ui.set_enabled(!app.prefs.colors.blindfold);

        let face_colors = app.prefs.colors.face_colors_list(app.puzzle.ty());

        let colors_selection_id = unique_id!();
        let mut selected_colors: Vec<bool> =
            ui.data().get_temp(colors_selection_id).unwrap_or_default();
        selected_colors.resize(app.puzzle.ty().shape.faces.len(), false);

        for i in 0..puzzle_type.shape.faces.len() {
            PieceFilterWidget::new_uppercased(
                "pieces with this color",
                piece_subset_from_sticker_colors!(puzzle_type, |colors| {
                    colors.any(|c| c == Face(i as _))
                }),
            )
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
            PieceFilterWidget::new_uppercased(
                "pieces with all these colors",
                piece_subset_from_sticker_colors!(puzzle_type, |colors| {
                    selected_colors.iter().enumerate().all(|(i, selected)| {
                        !selected || colors.clone().any(|color| color == Face(i as _))
                    })
                }),
            )
            .show(ui, app);

            PieceFilterWidget::new_uppercased(
                "pieces with any of these colors",
                piece_subset_from_sticker_colors!(puzzle_type, |colors| {
                    colors.any(|c| selected_colors[c.0 as usize])
                }),
            )
            .show(ui, app);

            PieceFilterWidget::new_uppercased(
                "pieces with only these colors",
                piece_subset_from_sticker_colors!(puzzle_type, |colors| {
                    colors.all(|c| selected_colors[c.0 as usize])
                }),
            )
            .show(ui, app);
        });

        ui.data().insert_temp(colors_selection_id, selected_colors);
    });

    ui.collapsing("Presets", |ui| {
        ui.set_enabled(!app.prefs.colors.blindfold);

        let opacity_prefs = &mut app.prefs.opacity;
        let mut piece_filter_presets = std::mem::take(&mut app.prefs.piece_filters[&puzzle_type]);

        let mut changed = false;

        let mut presets_ui = widgets::PresetsUi {
            id: unique_id!(),
            presets: &mut piece_filter_presets,
            changed: &mut changed,
            strings: Default::default(),
            enable_yaml: true,
        };

        presets_ui.show_header(ui, || PieceFilter {
            visible_pieces: app.puzzle.visible_pieces().to_bitvec(),
            hidden_opacity: opacity_prefs
                .save_opacity_in_piece_filter_preset
                .then_some(opacity_prefs.hidden),
        });
        presets_ui.show_postheader(ui, |ui| {
            ui.checkbox(
                &mut opacity_prefs.save_opacity_in_piece_filter_preset,
                "Save opacity",
            );
        });
        ui.separator();
        presets_ui.show_list(ui, |ui, _idx, preset| {
            preset
                .value
                .visible_pieces
                .resize(app.puzzle.ty().pieces.len(), false);
            PieceFilterWidget::new(
                &preset.preset_name,
                &preset.preset_name,
                preset.value.visible_pieces.clone(),
                preset.value.hidden_opacity,
            )
            .show(ui, app)
        });

        app.prefs.piece_filters[&puzzle_type] = piece_filter_presets;

        app.prefs.needs_save |= changed;
    });
}

#[must_use]
struct PieceFilterWidget<'a, W> {
    name: &'a str,
    label_ui: W,
    highlight_if_active: bool,
    all_except: bool,
    piece_set: BitVec,
    hidden_opacity: Option<f32>,
}
impl<'a> PieceFilterWidget<'a, egui::Button> {
    fn new_uppercased(name: &'a str, piece_set: BitVec) -> Self {
        let mut s = name.to_string();
        s[0..1].make_ascii_uppercase();
        Self::new(name, &s, piece_set, None)
    }
    fn new(name: &'a str, label: &str, piece_set: BitVec, hidden_opacity: Option<f32>) -> Self {
        Self {
            name,
            label_ui: egui::Button::new(label).frame(false),
            highlight_if_active: true,
            all_except: true,
            piece_set,
            hidden_opacity,
        }
    }
}
impl<'a, W> PieceFilterWidget<'a, W>
where
    W: egui::Widget,
{
    fn label_ui<W2>(self, label_ui: W2) -> PieceFilterWidget<'a, W2> {
        PieceFilterWidget {
            name: self.name,
            label_ui,
            highlight_if_active: false,
            all_except: self.all_except,
            piece_set: self.piece_set,
            hidden_opacity: self.hidden_opacity,
        }
    }

    /// Removes the "hide all except" button.
    fn no_all_except(mut self) -> Self {
        self.all_except = false;
        self
    }

    fn show(self, ui: &mut egui::Ui, app: &mut App) -> egui::Response {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                ui.spacing_mut().item_spacing.x /= 2.0;

                let puzzle = &mut app.puzzle;
                let current = puzzle.visible_pieces();

                let show_these = self.piece_set.clone() | current;
                let hide_these = !self.piece_set.clone() & current;
                let hide_others = self.piece_set.clone() & current;

                let mut small_button = |new_visible_set: BitVec, text: &str, hover_text: &str| {
                    let r = ui.add_enabled(
                        puzzle.visible_pieces() != new_visible_set,
                        |ui: &mut egui::Ui| widgets::small_icon_button(ui, text, hover_text),
                    );
                    if r.hovered() {
                        puzzle.set_visible_pieces_preview(Some(&new_visible_set), None);
                    }
                    if r.clicked() {
                        puzzle.set_visible_pieces(&new_visible_set);
                    }
                };
                small_button(show_these, "👁", &format!("Show {}", self.name));
                small_button(hide_these, "ｘ", &format!("Hide {}", self.name));
                small_button(hide_others, "❎", &format!("Hide all except {}", self.name));

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.min_size().y),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown)
                        .with_cross_align(egui::Align::LEFT),
                    |ui| {
                        let puzzle = &mut app.puzzle;
                        let current = puzzle.visible_pieces();

                        // Highlight name of active filter.
                        if ui.is_enabled() && self.highlight_if_active && current == self.piece_set
                        {
                            let visuals = ui.visuals_mut();
                            visuals.widgets.hovered = visuals.widgets.active;
                            visuals.widgets.inactive = visuals.widgets.active;
                        }

                        let r = ui.add(self.label_ui);
                        if r.hovered() {
                            puzzle.set_visible_pieces_preview(
                                Some(&self.piece_set),
                                self.hidden_opacity,
                            );
                        }
                        if r.clicked() {
                            puzzle.set_visible_pieces(&self.piece_set);
                            if let Some(hidden_opacity) = self.hidden_opacity {
                                if app.prefs.opacity.hidden != hidden_opacity {
                                    app.prefs.opacity.hidden = hidden_opacity;
                                    app.prefs.needs_save = true;
                                    app.request_redraw_puzzle();
                                }
                            }
                        }
                    },
                );
            });
        })
        .response
    }
}

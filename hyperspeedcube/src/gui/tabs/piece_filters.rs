use std::{collections::HashSet, hash::Hash};

use egui::NumExt;
use hyperpuzzle::{PieceMask, Puzzle};
use itertools::Itertools;

use crate::{
    app::App,
    gui::{
        components::{
            DragAndDrop, FancyComboBox, FilterCheckbox, FilterCheckboxAllowedStates,
            HelpHoverWidget, PresetHeaderUi, PresetSaveStatus, TextEditPopup,
            TextEditPopupResponse, PRESET_NAME_TEXT_EDIT_WIDTH,
        },
        markdown::{md, md_inline},
        util::EguiTempValue,
    },
    preferences::{
        ColorScheme, FilterCheckboxes, FilterExpr, FilterPieceSet, FilterPresetName,
        FilterPresetRef, FilterRule, FilterSeqPreset, Preferences, PresetsList,
    },
    L,
};

const PRESET_LIST_MIN_WIDTH: f32 = 200.0;
const CURRENT_PRESET_MIN_WIDTH: f32 = 350.0;

fn show_two_panels<R1, R2>(
    (ui, app): (&mut egui::Ui, &mut App),
    side: egui::panel::Side,
    side_panel_min_size: f32,
    side_panel_ui: impl FnOnce(&mut egui::Ui, &mut App) -> R1,
    central_panel_min_size: f32,
    central_panel_ui: impl FnOnce(&mut egui::Ui, &mut App) -> R2,
) -> (egui::InnerResponse<R1>, egui::InnerResponse<R2>) {
    // Unfortunately as of egui v0.28.1, the edges of the panel contents get cut
    // off unless we have *some* inner margin. Setting a negative outer margin
    // doesn't help.
    let panel_frame = egui::Frame::default().inner_margin(2.0);

    // TODO: file an egui bug! We shouldn't have to clear the background here.
    // There's a bug where `SidePanel` doesn't respect the clip rect from parent
    // `ScrollArea`s.

    let mut side_panel_frame = panel_frame;
    let mut central_panel_frame = panel_frame;
    match side {
        egui::panel::Side::Left => {
            side_panel_frame.inner_margin.right = 8.0;
            central_panel_frame.inner_margin.left = 8.0;
        }
        egui::panel::Side::Right => {
            side_panel_frame.inner_margin.left = 8.0;
            central_panel_frame.inner_margin.right = 8.0;
        }
    }

    let side_panel_margin = side_panel_frame.total_margin().sum().x;
    let central_panel_margin = central_panel_frame.total_margin().sum().x;

    let panel_margin = side_panel_margin + central_panel_margin;
    let min_total_size = side_panel_min_size + central_panel_min_size + panel_margin;
    ui.set_min_width(min_total_size);

    let max_side_panel_size = ui.available_width() - central_panel_min_size - panel_margin;

    // TODO: use `side`

    let clip_rect = ui.max_rect();
    egui::ScrollArea::horizontal()
        // .max_width(ui.available_width())
        .max_width(100.0)
        .min_scrolled_width(10000.0)
        // .min_scrolled_width(min_total_size)
        .show(ui, |ui| {
            let r1 = egui::SidePanel::left("piece_filters_side_panel")
                .resizable(ui.available_width() > min_total_size)
                .frame(side_panel_frame)
                .min_width(side_panel_min_size + side_panel_margin)
                .max_width(max_side_panel_size + side_panel_margin)
                .show_inside(ui, |ui| {
                    ui.set_clip_rect(egui::Rect::from_x_y_ranges(
                        clip_rect.x_range(),
                        egui::Rangef::EVERYTHING,
                    ));
                    side_panel_ui(ui, app)
                });

            let r2 = egui::CentralPanel::default()
                .frame(central_panel_frame)
                .show_inside(ui, |ui| {
                    ui.set_clip_rect(egui::Rect::from_x_y_ranges(
                        clip_rect.x_range(),
                        egui::Rangef::EVERYTHING,
                    ));
                    central_panel_ui(ui, app)
                });

            (r1, r2)
        })
        .inner
}

// TODO: factor out this (and `ColorsTab` and `DevToolsTab`)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum FiltersTab {
    #[default]
    AdHoc,
    PresetsList,
    EditPresets,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let l = &L.piece_filters;

    if !app.active_puzzle_view.has_puzzle() {
        ui.label(L.no_active_puzzle);
        return;
    };

    let tab_state = EguiTempValue::<FiltersTab>::new(ui);
    let mut tab = tab_state.get().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut tab, FiltersTab::AdHoc, l.ad_hoc);
            ui.selectable_value(&mut tab, FiltersTab::PresetsList, l.presets_list);
            ui.selectable_value(&mut tab, FiltersTab::EditPresets, l.edit_presets);
        });
    });
    tab_state.set(Some(tab));

    ui.group(|ui| match tab {
        FiltersTab::AdHoc => {
            app.active_puzzle_view.with(|p| p.view.filters.base = None);
            show_current_filter_preset_ui(ui, app);
        }
        FiltersTab::PresetsList => show_filter_presets_list_ui(ui, app),
        FiltersTab::EditPresets => {
            show_two_panels(
                (ui, app),
                egui::panel::Side::Left,
                PRESET_LIST_MIN_WIDTH,
                |ui, app| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| show_filter_presets_list_ui(ui, app))
                },
                CURRENT_PRESET_MIN_WIDTH,
                |ui, app| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| show_current_filter_preset_ui(ui, app))
                },
            );
        }
    });
}

fn show_filter_presets_list_ui(ui: &mut egui::Ui, app: &mut App) {
    let l = L.presets.piece_filters;

    ui.set_min_width(PRESET_LIST_MIN_WIDTH);

    let mut changed = false;

    let Some(puz) = app.active_puzzle_view.ty() else {
        return;
    };

    let current = app
        .active_puzzle_view
        .with_opt(|p| p?.view.filters.base.clone());

    ui.horizontal(|ui| {
        ui.strong(l.saved_presets);
        ui.label(format!("({})", puz.name));
        HelpHoverWidget::show_right_aligned(ui, L.help.piece_filter_presets);
    });

    let filter_prefs = app.prefs.filters_mut(&puz);

    let mut preset_dnd = DragAndDrop::new(ui);
    let mut seq_dnd = DragAndDrop::new(ui);

    ui.visuals_mut().collapsing_header_frame = true;

    let mut preset_to_activate = None;
    let mut preset_to_delete = None;
    let mut preset_to_rename = None;
    // let mut seq_to_delete = None; // TODO
    // let mut seq_to_rename = None;

    // Show filter presets
    ui.scope(|ui| {
        let taken_names: HashSet<String> = filter_prefs
            .presets
            .user_presets()
            .map(|p| p.name().clone())
            .collect();

        for preset in filter_prefs.presets.user_presets_mut() {
            let name = FilterPresetName::new(preset.name().clone());
            preset_dnd.vertical_reorder_by_handle(ui, name.clone(), |ui, _is_dragging| {
                show_preset_name(
                    ui,
                    &taken_names,
                    &current,
                    name,
                    &mut preset_to_activate,
                    &mut preset_to_rename,
                    &mut preset_to_delete,
                );
            });
        }
    });

    if ui.button(L.piece_filters.add_preset).clicked() {
        let name = (1..)
            .map(|i| format!("Filter preset {i}"))
            .find(|s| !filter_prefs.presets.contains_key(s))
            .expect("ran out of preset names!");
        filter_prefs
            .presets
            .save_preset(name.clone(), Default::default());
        preset_to_activate = Some(FilterPresetName::new(name));
        changed = true;
    }

    ui.separator();

    ui.horizontal(|ui| {
        ui.strong(L.piece_filters.saved_sequences);
        ui.label(format!("({})", puz.name));
        HelpHoverWidget::show_right_aligned(ui, L.help.piece_filter_sequences);
    });

    // Show filter sequences
    ui.scope(|ui| {
        for seq_preset in filter_prefs.sequences.user_presets_mut() {
            let taken_names: HashSet<String> = seq_preset
                .value
                .user_presets()
                .map(|p| p.name().clone())
                .collect();

            let seq_name = seq_preset.name().clone();
            let seq_list = &mut seq_preset.value;
            let is_any_dragging = seq_dnd.is_dragging();
            seq_dnd.vertical_reorder_by_handle(ui, seq_name.clone(), |ui, _is_dragging| {
                egui::CollapsingHeader::new(&seq_name)
                    .open(is_any_dragging.then_some(false))
                    // TODO: default open when created, but not when reordered
                    .show_unindented(ui, |ui| {
                        let mut is_first = true;
                        for preset in seq_list.user_presets_mut() {
                            let name = FilterPresetName {
                                seq: Some(seq_name.clone()),
                                preset: preset.name().clone(),
                            };
                            preset_dnd.vertical_reorder_by_handle(
                                ui,
                                name.clone(),
                                |ui, _is_dragging| {
                                    show_seq_preset_name(
                                        ui,
                                        &taken_names,
                                        &current,
                                        name,
                                        &mut preset.value,
                                        &mut preset_to_activate,
                                        &mut preset_to_rename,
                                        &mut preset_to_delete,
                                        &mut changed,
                                        is_first,
                                    );
                                },
                            );
                            is_first = false;
                        }

                        ui.horizontal(|ui| {
                            ui.add_space(ui.spacing().indent + ui.spacing().item_spacing.x + 29.0);
                            if ui.button(L.piece_filters.add_preset).clicked() {
                                let preset_name = (1..)
                                    .map(|i| format!("Step {i}"))
                                    .find(|s| !seq_list.contains_key(s))
                                    .expect("ran out of preset names!");
                                seq_list
                                    .save_preset(preset_name.clone(), FilterSeqPreset::default());
                                preset_to_activate = Some(FilterPresetName {
                                    seq: Some(seq_name),
                                    preset: preset_name,
                                });
                                changed = true;
                            }
                        });
                    });
            });
        }
    });

    if ui.button(L.piece_filters.add_sequence).clicked() {
        let seq_name = (1..)
            .map(|i| format!("Filter sequence {i}"))
            .find(|s| !filter_prefs.sequences.contains_key(s))
            .expect("ran out of sequence names!");
        let preset_name = "Step 1".to_owned();
        let mut seq = PresetsList::default();
        seq.save_preset(preset_name.clone(), FilterSeqPreset::default());
        filter_prefs.sequences.save_preset(seq_name.clone(), seq);
        preset_to_activate = Some(FilterPresetName {
            seq: Some(seq_name),
            preset: preset_name,
        });
        changed = true;
    }

    changed |= preset_dnd.end_reorder(ui, filter_prefs);
    changed |= seq_dnd.end_reorder(ui, &mut filter_prefs.sequences);

    if let Some((old_name, new_name)) = preset_to_rename {
        filter_prefs.rename_preset(&old_name, &new_name);
        changed = true;
    }
    if let Some(name) = preset_to_delete {
        filter_prefs.remove_preset(&name);
        changed = true;
    }
    if let Some(name) = preset_to_activate {
        app.active_puzzle_view
            .with(|p| p.view.filters.load_preset(filter_prefs, &name));
    }

    if changed {
        app.active_puzzle_view
            .with(|p| p.view.filters.mark_changed());
    }
    app.prefs.needs_save |= changed;
}

fn show_seq_preset_name(
    ui: &mut egui::Ui,
    taken_names: &HashSet<String>,
    current: &Option<FilterPresetRef>,
    name: FilterPresetName,
    value: &mut FilterSeqPreset,
    to_activate: &mut Option<FilterPresetName>,
    to_rename: &mut Option<(FilterPresetName, String)>,
    to_delete: &mut Option<FilterPresetName>,
    changed: &mut bool,
    is_first: bool,
) {
    ui.horizontal(|ui| {
        // "Include previous" button
        ui.scope(|ui| {
            if is_first {
                ui.disable();
            }
            if is_first && value.include_previous {
                value.include_previous = false;
                *changed = true;
            }
            let label;
            let hover;
            if value.include_previous {
                let inactive_text_color = &mut ui.visuals_mut().widgets.inactive.fg_stroke.color;
                *inactive_text_color = inactive_text_color.gamma_multiply(0.3);
                label = "⮩";
                hover = L.piece_filters.include_privous;
            } else {
                label = "★";
                hover = L.piece_filters.ignore_previous;
            }

            if ui
                .selectable_label(false, label)
                .on_hover_text(hover)
                .clicked()
            {
                value.include_previous ^= true;
                *changed = true;
            }
        });

        show_preset_name(
            ui,
            taken_names,
            current,
            name,
            to_activate,
            to_rename,
            to_delete,
        );
    });
}

fn show_preset_name(
    ui: &mut egui::Ui,
    taken_names: &HashSet<String>,
    current: &Option<FilterPresetRef>,
    name: FilterPresetName,
    to_activate: &mut Option<FilterPresetName>,
    to_rename: &mut Option<(FilterPresetName, String)>,
    to_delete: &mut Option<FilterPresetName>,
) {
    let is_active = current.as_ref().is_some_and(|r| r.name() == name);

    let r = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        ui.selectable_label(is_active, &name.preset)
    });

    let r = r.inner.on_hover_ui(|ui| {
        md(ui, L.click_to.activate.with(L.inputs.click));
        md(ui, L.click_to.rename.with(L.inputs.right_click));
        md(ui, L.click_to.delete.with(L.inputs.middle_click));
    });

    let mods = ui.input(|input| input.modifiers);
    let cmd = mods.command;
    let alt = mods.alt;

    // Click to activate
    if r.clicked() {
        *to_activate = Some(name.clone());
    }

    // Right-click to rename
    let mut popup = TextEditPopup::new(ui);
    if r.secondary_clicked() {
        popup.open(name.preset.clone());
    }
    let popup_response = popup.if_open(|popup| {
        let validate_preset_rename = move |new_name: &str| {
            let l = &L.presets.piece_filters;
            if new_name.is_empty() {
                Err(Some(l.errors.empty_name.into()))
            } else if taken_names.contains(new_name) {
                Err(Some(l.errors.name_conflict.into()))
            } else {
                Ok(Some(l.actions.rename.into()))
            }
        };

        popup
            .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
            .text_edit_hint(L.presets.piece_filters.new_name_hint)
            .confirm_button_validator(&validate_preset_rename)
            .delete_button_validator(&|_| Ok(Some(L.presets.piece_filters.actions.delete.into())))
            .at(ui, &r, egui::vec2(-4.0, 1.0))
            .show(ui)
    });
    if let Some(r) = popup_response {
        match r {
            TextEditPopupResponse::Confirm(new_name) => {
                *to_rename = Some((name.clone(), new_name));
            }
            TextEditPopupResponse::Delete => *to_delete = Some(name.clone()),
            TextEditPopupResponse::Cancel => (),
        }
    }

    // Alt+click to delete
    if r.middle_clicked() || alt && !cmd && r.clicked() {
        *to_delete = Some(name.clone());
    }
}

fn show_current_filter_preset_ui(ui: &mut egui::Ui, app: &mut App) {
    ui.set_min_width(ui.available_width().at_least(CURRENT_PRESET_MIN_WIDTH));

    let puz = app.active_puzzle_view.ty();
    if puz.is_none() {
        ui.disable();
    }

    let saved_preset_ref = app
        .active_puzzle_view
        .with_opt(|p| p?.view.filters.base.clone());
    let current = app
        .active_puzzle_view
        .with(|p| p.view.filters.preset.clone());

    let mut filter_prefs = puz.as_ref().map(|puz| app.prefs.filters_mut(&puz));
    let saved_preset = filter_prefs
        .as_ref()
        .and_then(|filter_prefs| filter_prefs.get(&saved_preset_ref.as_ref()?.name()));

    let is_unsaved = saved_preset.as_ref().map(|f| &f.inner) != current.as_ref();

    let preset_name = saved_preset_ref.as_ref().map(|r| r.to_string());

    let mut save_changes = false;
    ui.add(PresetHeaderUi::<()> {
        text: &L.presets.piece_filters,
        preset_name: preset_name.as_deref().unwrap_or(""),

        help_contents: Some(L.help.piece_filters),
        yaml: None,
        save_status: if preset_name.is_some() {
            PresetSaveStatus::ManualSave {
                is_unsaved,
                overwrite: saved_preset.is_some(),
            }
        } else {
            PresetSaveStatus::CantSave {
                message: L.piece_filters.cant_save,
            }
        },

        save_changes: &mut save_changes,
    });
    if save_changes {
        if let Some(filter_prefs) = &mut filter_prefs {
            if let Some(preset_ref) = saved_preset_ref {
                if let Some(value) = current {
                    filter_prefs.save_preset(
                        &preset_ref.name(),
                        FilterSeqPreset {
                            inner: value,
                            ..saved_preset.unwrap_or_default()
                        },
                    );
                    app.prefs.needs_save = true;
                }
            }
        }
    }

    app.active_puzzle_view.with(|p| {
        let puz = p.puzzle();
        let colors = puz.colors.list.map_ref(|_, info| info.name.as_str());
        let piece_types = puz.piece_types.map_ref(|_, info| info.name.as_str());

        let mut style_options = vec![(None, crate::DEFAULT_STYLE_NAME.into())];
        for style in app.prefs.custom_styles.user_presets() {
            style_options.push((Some(style.new_ref()), style.name().into()));
        }

        let mut changed = false;
        let mut new_fallback_state = None;

        let active_rules = &mut p.view.filters.active_rules;
        let preset = &mut p.view.filters.preset;

        active_rules.resize(preset.rules.len(), true);

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut dnd = DragAndDrop::new(ui);
                let is_any_dragging = dnd.is_dragging();

                let mut remaining_pieces = PieceMask::new_full(puz.pieces.len());
                let mut to_delete = None;

                for (i, rule) in preset.rules.iter_mut().enumerate() {
                    let these_pieces = rule.set.eval(&puz);

                    let affected_piece_count = (remaining_pieces.clone() & &these_pieces).len();
                    if active_rules[i] {
                        remaining_pieces &= !&these_pieces;
                    }

                    dnd.vertical_reorder_by_handle(ui, i, |ui, _is_dragging| {
                        ui.vertical(|ui| {
                            // TODO: it would be better to show the frame on
                            // just the collapsing headers we want, but then
                            // they aren't horizontally justified. This
                            // seems to be a bug in egui.
                            ui.visuals_mut().collapsing_header_frame = true;
                            ui.horizontal(|ui| {
                                // TODO: singlular vs. plural
                                let n = affected_piece_count.to_string();
                                let r = ui.checkbox(
                                    &mut active_rules[i],
                                    md_inline(
                                        ui,
                                        L.piece_filters.show_n_pieces_with_style.with(&n),
                                    ),
                                );
                                changed |= r.changed();

                                let r = &ui.add(FancyComboBox::new(
                                    unique_id!(i),
                                    &mut rule.style,
                                    &style_options,
                                ));
                                changed |= r.changed();
                            });
                            let previous_rule_piece_count =
                                these_pieces.len() - affected_piece_count;
                            if previous_rule_piece_count > 0 {
                                // TODO: singular vs. plural
                                let n = previous_rule_piece_count.to_string();
                                md(ui, L.piece_filters.n_match_previous_rule.with(&n));
                            }

                            match &mut rule.set {
                                FilterPieceSet::Expr(expr) => {
                                    let r = &show_filter_expr_ui(ui, i, expr, &puz);
                                    changed |= r.changed();
                                }
                                FilterPieceSet::Checkboxes(checkboxes) => {
                                    let expr_string = checkboxes.to_string(&colors, &piece_types);
                                    let r = egui::CollapsingHeader::new(&expr_string)
                                        .id_source(unique_id!(i))
                                        // TODO: default open when created, but not when reordered
                                        .open(is_any_dragging.then_some(false)) // TODO: reopen?
                                        .show_unindented(ui, |ui| {
                                            ui.visuals_mut().collapsing_header_frame = false;
                                            show_filter_checkboxes_ui(
                                                i,
                                                ui,
                                                checkboxes,
                                                &puz,
                                                &p.view.colors.value,
                                                &app.prefs,
                                                &mut changed,
                                            );
                                        });
                                    r.header_response.context_menu(|ui| {
                                        if ui.button(L.piece_filters.convert_to_text_rule).clicked()
                                        {
                                            rule.set = FilterPieceSet::Expr(expr_string);
                                        }
                                    });
                                }
                            }

                            if ui.button(L.piece_filters.delete_rule).clicked() {
                                to_delete = Some(i);
                            }

                            ui.add_space(ui.spacing().item_spacing.y * 2.0);
                        });
                    });
                }

                if let Some(i) = to_delete {
                    preset.rules.remove(i);
                }

                changed |= dnd.end_reorder(ui, &mut preset.rules);

                ui.horizontal_wrapped(|ui| {
                    if ui.button(L.piece_filters.add_checkboxes_rule).clicked() {
                        preset.rules.push(FilterRule::new_checkboxes());
                    }
                    if ui.button(L.piece_filters.add_text_rule).clicked() {
                        preset.rules.push(FilterRule::new_expr());
                    }
                });

                ui.separator();

                let mut fallback_to_previous_filter =
                    p.view.filters.combined_fallback_preset.is_some();
                let r = ui.checkbox(
                    &mut fallback_to_previous_filter,
                    L.piece_filters.show_remaining_pieces_with_previous_filter,
                );
                if r.clicked() {
                    changed = true;
                    new_fallback_state = Some(fallback_to_previous_filter);
                }

                ui.horizontal(|ui| {
                    if fallback_to_previous_filter {
                        ui.disable();
                    }
                    ui.label(L.piece_filters.show_remaining_peices_with_style);
                    let r = ui.add(FancyComboBox {
                        combo_box: egui::ComboBox::from_id_source(unique_id!()),
                        selected: &mut preset.fallback_style,
                        options: style_options.clone(),
                    });
                    changed |= r.changed();
                });
            });

        if let Some(fallback_to_previous_filter) = new_fallback_state {
            changed = true;
            if fallback_to_previous_filter {
                p.view
                    .filters
                    .update_combined_fallback_preset(app.prefs.filters_mut(&puz));
            } else {
                p.view.filters.combined_fallback_preset = None;
            }
        }

        if changed {
            p.view.filters.mark_changed();
        }
    });
}

#[must_use]
fn show_filter_expr_ui(
    ui: &mut egui::Ui,
    i: usize,
    expr_string: &mut String,
    puz: &std::sync::Arc<Puzzle>,
) -> egui::Response {
    ui.scope(|ui| {
        let err = FilterExpr::from_str(expr_string).validate(puz).err();
        if err.is_some() {
            ui.visuals_mut().selection.stroke.color = ui.visuals().warn_fg_color;
            ui.visuals_mut().widgets.hovered.bg_stroke.color = ui.visuals().warn_fg_color;
            ui.visuals_mut().widgets.active.bg_stroke.color = ui.visuals().warn_fg_color;
        }
        let r = ui.add(
            egui::TextEdit::multiline(expr_string)
                .id_source(unique_id!(i))
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(1),
        );
        if let Some(e) = err {
            if r.has_focus() || r.hovered() {
                r.show_tooltip_ui(|ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, e);
                });
            }
        }
        r
    })
    .inner
}

fn show_filter_checkboxes_ui(
    id: impl Hash,
    ui: &mut egui::Ui,
    filters: &mut FilterCheckboxes,
    puzzle: &Puzzle,
    color_scheme: &ColorScheme,
    prefs: &Preferences,
    changed: &mut bool,
) {
    let _ = filters.colors.resize(puzzle.colors.len());
    let _ = filters.piece_types.resize(puzzle.piece_types.len());

    let allowed_states = FilterCheckboxAllowedStates::NeutralShowHide;
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        unique_id!(&id),
        true,
    )
    .show_header(ui, |ui| {
        let mut common_state = get_common_state(filters.colors.iter_values());
        let r = ui.add(FilterCheckbox::new(
            allowed_states,
            common_state.as_mut(),
            L.piece_filters.colors,
        ));
        *changed |= r.changed();
        if r.changed() {
            filters.colors.fill(common_state.flatten());
        }
    })
    .body(|ui| {
        let states_iter = filters.colors.iter_values_mut();
        let rgbs_iter = color_scheme
            .values()
            .map(|color| prefs.color_palette.get(color).unwrap_or_default());
        let color_infos_iter = puzzle.colors.list.iter_values();

        // TODO: refactor
        let show_the_things = |ui: &mut egui::Ui| {
            for ((state, rgb), color_info) in states_iter.zip(rgbs_iter).zip(color_infos_iter) {
                let r = &ui.add(
                    FilterCheckbox::new(allowed_states, Some(state), &color_info.display)
                        .color(crate::util::rgb_to_egui_color32(rgb))
                        .indent(),
                );
                *changed |= r.changed();
            }
        };

        if puzzle.colors.len() > 12 {
            egui::Frame {
                stroke: egui::Stroke {
                    width: 1.0,
                    color: ui.visuals().window_stroke.color,
                },
                inner_margin: egui::Margin::same(3.5),
                ..Default::default()
            }
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .min_scrolled_height(300.0)
                    .max_height(100.0)
                    .show(ui, |ui| {
                        show_the_things(ui);
                        ui.set_min_width(ui.min_rect().width() + 50.0);
                    });
            });
        } else {
            show_the_things(ui);
        }
    });

    let allowed_states = FilterCheckboxAllowedStates::NeutralHide;
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        unique_id!(&id),
        true,
    )
    .show_header(ui, |ui| {
        let mut common_state = get_common_state(filters.piece_types.iter_values());
        let r = ui.add(FilterCheckbox::new(
            allowed_states,
            common_state.as_mut(),
            L.piece_filters.piece_types,
        ));
        *changed |= r.changed();
        if r.changed() {
            filters.piece_types.fill(common_state.flatten());
        }
    })
    .body(|ui| {
        let states_iter = filters.piece_types.iter_values_mut();
        let piece_type_infos_iter = puzzle.piece_types.iter_values();
        for (state, piece_type_info) in states_iter.zip(piece_type_infos_iter) {
            let r = &ui.add(
                FilterCheckbox::new(allowed_states, Some(state), &piece_type_info.name).indent(),
            );
            *changed |= r.changed();
        }
    });
}

fn get_common_state<'a>(
    states: impl IntoIterator<Item = &'a Option<bool>>,
) -> Option<Option<bool>> {
    states.into_iter().all_equal_value().ok().cloned()
}

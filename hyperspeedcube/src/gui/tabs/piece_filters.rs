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
        ColorScheme, FilterCheckboxes, FilterExpr, FilterPieceSet, FilterRule, Preferences,
    },
    puzzle::PuzzleFiltersState,
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
    let mut panel_frame = egui::Frame::central_panel(ui.style());
    let panel_margin = panel_frame.inner_margin.sum().x;
    panel_frame.inner_margin.bottom = 0.0;
    panel_frame.inner_margin.top = 0.0;

    let mut side_panel_frame = panel_frame;
    let mut central_panel_frame = panel_frame;
    match side {
        egui::panel::Side::Left => {
            side_panel_frame.inner_margin.left = 0.0;
            central_panel_frame.inner_margin.right = 0.0;
        }
        egui::panel::Side::Right => {
            side_panel_frame.inner_margin.right = 0.0;
            central_panel_frame.inner_margin.left = 0.0;
        }
    }

    let min_total_size = side_panel_min_size + central_panel_min_size + panel_margin;
    ui.set_min_width(min_total_size);

    let max_side_panel_size = ui.available_width() - central_panel_min_size - panel_margin;

    let r1 = egui::SidePanel::left("piece_filters_side_panel")
        .resizable(ui.available_width() > min_total_size)
        .frame(side_panel_frame)
        .min_width(side_panel_min_size)
        .max_width(max_side_panel_size)
        .show_inside(ui, |ui| side_panel_ui(ui, app));
    let r2 = egui::CentralPanel::default()
        .frame(central_panel_frame)
        .show_inside(ui, |ui| central_panel_ui(ui, app));

    (r1, r2)
}

// TODO: factor out this (and `ColorsTab` and `DevToolsTab`)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum FiltersTab {
    PresetsList,
    #[default]
    CurrentPreset,
    Both,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let l = &L.piece_filters;

    if !app.has_active_puzzle() {
        ui.label(L.no_active_puzzle);
        return;
    };

    let tab_state = EguiTempValue::<FiltersTab>::new(ui);
    let mut tab = tab_state.get().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut tab, FiltersTab::PresetsList, l.presets_list);
            ui.selectable_value(&mut tab, FiltersTab::CurrentPreset, l.current_preset);
            ui.selectable_value(&mut tab, FiltersTab::Both, l.both);
        });
    });
    tab_state.set(Some(tab));

    ui.group(|ui| {
        egui::ScrollArea::horizontal()
            .auto_shrink(false)
            .show(ui, |ui| match tab {
                FiltersTab::PresetsList => show_filter_presets_list_ui(ui, app),
                FiltersTab::CurrentPreset => show_current_filter_preset_ui(ui, app),
                FiltersTab::Both => {
                    show_two_panels(
                        (ui, app),
                        egui::panel::Side::Left,
                        PRESET_LIST_MIN_WIDTH,
                        show_filter_presets_list_ui,
                        CURRENT_PRESET_MIN_WIDTH,
                        show_current_filter_preset_ui,
                    );
                }
            });
    });
}

fn show_filter_presets_list_ui(ui: &mut egui::Ui, app: &mut App) {
    let l = L.presets.piece_filters;

    ui.set_min_width(PRESET_LIST_MIN_WIDTH);

    let mut changed = false;
    let mut changed_current = false;

    let Some(puz) = app.active_puzzle_type() else {
        return;
    };

    let Some((current_seq, current_preset)) = app.with_active_puzzle_view(|p| {
        (
            p.view.filters.sequence_name.clone(),
            p.view.filters.preset_name.clone(),
        )
    }) else {
        return;
    };

    ui.horizontal(|ui| {
        ui.strong(l.saved_presets);
        ui.label(format!("({})", puz.name));
        HelpHoverWidget::show_right_aligned(ui, L.help.piece_filter_presets);
    });

    let filter_prefs = app.prefs.piece_filters.settings_mut(&puz);

    let mut preset_dnd = DragAndDrop::new(ui);
    let mut seq_dnd = DragAndDrop::new(ui);

    ui.visuals_mut().collapsing_header_frame = true;

    let mut to_activate = None;
    let mut to_delete = None;
    let mut to_rename = None;

    let taken_preset_names: HashSet<String> = filter_prefs.presets.keys().cloned().collect();
    let validate_preset_rename = move |new_name: &str| {
        if new_name.is_empty() {
            Err(Some(l.errors.empty_name.into()))
        } else if taken_preset_names.contains(new_name) {
            Err(Some(l.errors.name_conflict.into()))
        } else {
            Ok(Some(l.actions.rename.into()))
        }
    };

    // Show filter presets
    for (i, (preset_name, preset)) in &mut filter_prefs.presets.iter().enumerate() {
        let is_active = current_seq.is_none() && current_preset.as_ref() == Some(preset_name);
        let index = (None, i);
        let r = preset_dnd.vertical_reorder_by_handle(ui, index, |ui, _is_dragging| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT)
                    .with_main_justify(true)
                    .with_cross_justify(true),
                |ui| ui.selectable_label(is_active, preset_name),
            )
            .inner
        });
        let r = r.inner.on_hover_ui(|ui| {
            md(ui, L.click_to.activate.with(&L.inputs.click));
            md(ui, L.click_to.rename.with(&L.inputs.right_click));
            md(ui, L.click_to.delete.with(&L.inputs.middle_click));
        });

        let mods = ui.input(|input| input.modifiers);
        let cmd = mods.command;
        let alt = mods.alt;

        // Click to activate
        if r.clicked() {
            to_activate = Some(PuzzleFiltersState {
                sequence_name: None,
                preset_name: Some(preset_name.clone()),
                preset: preset.clone(),
                active_rules: vec![],
            });
        }

        // Right-click to rename
        let mut popup = TextEditPopup::new(ui);
        if r.secondary_clicked() {
            popup.open(preset_name.clone());
        }
        let popup_response = popup.if_open(|popup| {
            popup
                .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                .text_edit_hint(l.new_name_hint)
                .confirm_button_validator(&validate_preset_rename)
                .delete_button_validator(&|_| {
                    Ok(Some(L.presets.color_schemes.actions.delete.into()))
                })
                .at(ui, &r, egui::vec2(-4.0, 0.0))
                .show(ui)
        });
        if let Some(r) = popup_response {
            match r {
                TextEditPopupResponse::Confirm(new_name) => {
                    to_rename = Some((i, new_name));
                }
                TextEditPopupResponse::Delete => to_delete = Some(i),
                TextEditPopupResponse::Cancel => (),
            }
        }

        // Alt+click to delete
        if r.middle_clicked() || alt && !cmd && r.clicked() {
            to_delete = Some(i);
        }
    }

    if let Some((i, new_name)) = to_rename {
        // TODO: factor out this logic
        if let Some((_, v)) = filter_prefs.presets.swap_remove_index(i) {
            let (j, _) = filter_prefs.presets.insert_full(new_name, v);
            filter_prefs.presets.swap_indices(i, j);
            changed_current = true; // maybe we changed the current one!
                                    // TODO: handle global rename
            changed = true;
        }
    }
    if let Some(i) = to_delete {
        filter_prefs.presets.shift_remove_index(i);
        changed_current = true; // TODO: is this right?
        changed = true;
    }

    if ui.button(L.piece_filters.add_preset).clicked() {
        let name = (1..)
            .map(|i| format!("Filter preset {i}"))
            .find(|s| !filter_prefs.presets.contains_key(s))
            .expect("ran out of preset names!");
        filter_prefs
            .presets
            .insert(name.clone(), Default::default());
        to_activate = Some(PuzzleFiltersState {
            sequence_name: None,
            preset_name: Some(name),
            preset: Default::default(),
            active_rules: vec![],
        });
        changed = true;
        changed_current = true;
    }

    // Show filter sequences
    for (i, (seq_name, presets)) in &mut filter_prefs.sequences.iter_mut().enumerate() {
        seq_dnd.vertical_reorder_by_handle(ui, i, |ui, is_dragging| {
            egui::CollapsingHeader::new(seq_name)
                .open(is_dragging.then_some(false))
                .show_unindented(ui, |ui| {
                    for (j, (preset_name, preset)) in presets.iter_mut().enumerate() {
                        let is_active = current_seq.as_ref() == Some(seq_name)
                            && current_preset.as_ref() == Some(preset_name);
                        let index = (Some(i), j);
                        preset_dnd.vertical_reorder_by_handle(ui, index, |ui, _is_dragging| {
                            ui.horizontal(|ui| {
                                ui.scope(|ui| {
                                    if j == 0 {
                                        ui.disable();
                                    }
                                    if j == 0 && preset.include_previous {
                                        preset.include_previous = false;
                                        changed = true;
                                    }
                                    let label;
                                    let hover;
                                    if preset.include_previous {
                                        label = "★";
                                        hover = L.piece_filters.ignore_previous;
                                    } else {
                                        let inactive_text_color =
                                            &mut ui.visuals_mut().widgets.inactive.fg_stroke.color;
                                        *inactive_text_color =
                                            inactive_text_color.gamma_multiply(0.3);
                                        label = "⮩";
                                        hover = L.piece_filters.include_privous;
                                    }
                                    if ui
                                        .selectable_label(false, label)
                                        .on_hover_text(hover)
                                        .clicked()
                                    {
                                        preset.include_previous ^= true;
                                        changed = true;
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::top_down_justified(egui::Align::LEFT),
                                    |ui| ui.selectable_label(is_active, preset_name),
                                );
                                // TODO: activate
                            });
                        });
                    }
                });
        });
    }

    preset_dnd.end_reorder(ui, filter_prefs);

    if let Some(new_state) = to_activate {
        app.with_active_puzzle_view(|p| p.view.filters = new_state);
        changed_current = true;
    }

    // Copy settings back to the active puzzle.
    if changed_current {
        app.with_active_puzzle_view(|p| p.view.notify_filters_changed());
    }
    app.prefs.needs_save |= changed;
}

fn show_current_filter_preset_ui(ui: &mut egui::Ui, app: &mut App) {
    ui.set_min_width(ui.available_width().at_least(CURRENT_PRESET_MIN_WIDTH));

    let puz = app.active_puzzle_type();
    if !app.has_active_puzzle() {
        ui.disable();
    }

    let sequence_name = app
        .with_active_puzzle_view(|p| p.view.filters.sequence_name.clone())
        .flatten();
    let preset_name = app
        .with_active_puzzle_view(|p| p.view.filters.preset_name.clone())
        .flatten();
    let current_preset = app.with_active_puzzle_view(|p| p.view.filters.preset.clone());

    let mut filter_prefs = puz
        .as_ref()
        .map(|puz| app.prefs.piece_filters.settings_mut(puz));
    let saved_preset = filter_prefs
        .as_ref()
        .and_then(|filter_prefs| filter_prefs.get(sequence_name.as_ref(), preset_name.as_ref()));

    let is_unsaved = saved_preset != current_preset.as_ref();

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
            if let Some(name) = preset_name.clone() {
                if let Some(value) = current_preset {
                    filter_prefs.presets.insert(name, value);
                }
            }
        }
    }

    app.with_active_puzzle_view(|p| {
        let puz = p.puzzle();
        let colors = puz.colors.list.map_ref(|_, info| info.name.as_str());
        let piece_types = puz.piece_types.map_ref(|_, info| info.name.as_str());

        let mut style_options = vec![(None, crate::DEFAULT_STYLE_NAME.into())];
        for style in app.prefs.styles.custom.user_list() {
            style_options.push((Some(style.name.clone()), style.name.clone().into()));
        }

        let mut changed = false;

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
                                        .default_open(true)
                                        .open(is_any_dragging.then_some(false))
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

                dnd.end_reorder(ui, &mut preset.rules);

                ui.horizontal_wrapped(|ui| {
                    if ui.button(L.piece_filters.add_checkboxes_rule).clicked() {
                        preset.rules.push(FilterRule::new_checkboxes());
                    }
                    if ui.button(L.piece_filters.add_text_rule).clicked() {
                        preset.rules.push(FilterRule::new_expr());
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(L.piece_filters.show_remaining_peices_with_style);
                    let r = ui.add(FancyComboBox {
                        combo_box: egui::ComboBox::from_id_source(unique_id!()),
                        selected: &mut preset.fallback_style,
                        options: style_options.clone(),
                    });
                    changed |= r.changed();
                });
            });

        // Copy settings back to the active puzzle.
        if changed {
            p.view.notify_filters_changed();
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
        let err = FilterExpr::from_str(&expr_string).validate(puz).err();
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

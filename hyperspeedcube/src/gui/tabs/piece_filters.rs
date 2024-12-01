use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hyperprefs::{
    ColorScheme, FilterCheckboxes, FilterExpr, FilterPieceSet, FilterPreset, FilterPresetName,
    FilterPresetRef, FilterRule, FilterSeqPreset, Preferences, PresetRef, PresetsList,
    PuzzleFilterPreferences,
};
use hyperpuzzle::{PerPieceType, PieceMask, PieceTypeHierarchy, Puzzle};
use itertools::Itertools;

use super::PuzzleWidget;
use crate::app::App;
use crate::gui::components::{
    DragAndDrop, FancyComboBox, FilterCheckbox, FilterCheckboxAllowedStates, HelpHoverWidget,
    PresetHeaderUi, PresetSaveStatus, TextEditPopup, TextEditPopupResponse,
    PRESET_NAME_TEXT_EDIT_WIDTH,
};
use crate::gui::markdown::{md, md_inline};
use crate::gui::util::{text_width, EguiTempValue};
use crate::puzzle::PuzzleFiltersState;
use crate::L;

const PRESET_LIST_MIN_WIDTH: f32 = 200.0;
const CURRENT_PRESET_MIN_WIDTH: f32 = 350.0;

// TODO: factor out this (and `ColorsTab` and `DevToolsTab`)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum FiltersTab {
    #[default]
    AdHoc,
    PresetsList,
    EditPresets,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let l = &L.piece_filters.tabs;

    let tab_state = EguiTempValue::<FiltersTab>::new(ui);
    let mut tab = tab_state.get().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::horizontal()
            .id_salt("tab_select")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut tab, FiltersTab::AdHoc, l.ad_hoc);
                    ui.selectable_value(&mut tab, FiltersTab::PresetsList, l.presets_list);
                    ui.selectable_value(&mut tab, FiltersTab::EditPresets, l.edit_presets);
                });
            });
    });
    tab_state.set(Some(tab));

    ui.group(|ui| {
        egui::ScrollArea::horizontal()
            .auto_shrink(false)
            .show(ui, |ui| match tab {
                FiltersTab::AdHoc => {
                    app.active_puzzle_view.with(|p| p.view.filters.base = None);
                    egui::ScrollArea::vertical()
                        .id_salt("current_filter")
                        .auto_shrink(false)
                        .show(ui, |ui| show_current_filter_preset_ui(ui, app));
                }
                FiltersTab::PresetsList => {
                    ui.set_min_width(PRESET_LIST_MIN_WIDTH);
                    show_filter_presets_list_ui(ui, app, false);
                }
                FiltersTab::EditPresets => {
                    let h = ui.available_height();
                    ui.horizontal(|ui| {
                        ui.set_height(h);
                        ui.vertical(|ui| {
                            ui.set_width(PRESET_LIST_MIN_WIDTH);
                            show_filter_presets_list_ui(ui, app, true);
                        });
                        ui.add(egui::Separator::default().grow(6.0));
                        ui.vertical(|ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("current_filter")
                                .auto_shrink(false)
                                .show(ui, |ui| show_current_filter_preset_ui(ui, app));
                        });
                    });
                }
            });
    });
}

fn show_filter_presets_list_ui(ui: &mut egui::Ui, app: &mut App, allow_ad_hoc: bool) {
    let mut changed = false;

    let fallback_style = app.prefs.first_custom_style();

    app.active_puzzle_view.with_opt(|p| {
        egui::ScrollArea::vertical()
            .id_salt("filter_presets_list")
            .show(ui, |ui| {
                let ad_hoc_rect = allow_ad_hoc.then(|| reserve_space_for_ad_hoc_preset_name(ui));

                if let Some(p) = p {
                    let puz = p.puzzle();
                    let filter_prefs = app.prefs.filters_mut(&puz);
                    show_filter_presets_list_ui_contents(
                        ui,
                        Some(&puz),
                        filter_prefs,
                        &mut p.view.filters,
                        &mut changed,
                        fallback_style,
                    );
                    if let Some(rect) = ad_hoc_rect {
                        if show_ad_hoc_preset_name(ui, rect, &p.view.filters.base).clicked() {
                            p.view.filters.load_preset(filter_prefs, None);
                        }
                    }
                    if changed {
                        p.view.filters.reload(&filter_prefs);
                    }
                } else {
                    ui.disable();
                    show_filter_presets_list_ui_contents(
                        ui,
                        None,
                        &mut PuzzleFilterPreferences::default(),
                        &mut PuzzleFiltersState::new_empty(),
                        &mut false,
                        fallback_style,
                    );
                    if let Some(rect) = ad_hoc_rect {
                        show_ad_hoc_preset_name(ui, rect, &None);
                    }
                }
            });
    });

    app.prefs.needs_save |= changed;
}
fn show_filter_presets_list_ui_contents(
    ui: &mut egui::Ui,
    puzzle_type: Option<&Puzzle>,
    filter_prefs: &mut PuzzleFilterPreferences,
    current: &mut PuzzleFiltersState,
    changed: &mut bool,
    fallback_style: Option<PresetRef>,
) {
    let l = L.presets.piece_filters;

    ui.horizontal(|ui| {
        ui.strong(l.saved_presets);
        if let Some(puz) = puzzle_type {
            ui.label(format!("({})", puz.name));
        }
        HelpHoverWidget::show_right_aligned(ui, L.help.piece_filter_presets);
    });

    let mut preset_dnd = DragAndDrop::new(ui);
    let mut seq_dnd = DragAndDrop::new(ui);

    ui.visuals_mut().collapsing_header_frame = true;

    let mut preset_to_activate = None;
    let mut preset_to_delete = None;
    let mut preset_to_rename = None;
    let mut seq_to_delete = None;
    let mut seq_to_rename = None;

    // Show filter presets
    let taken_preset_names = filter_prefs.presets.taken_names();

    for preset in filter_prefs.presets.user_presets_mut() {
        let name = FilterPresetName::new(preset.name().clone());
        preset_dnd.vertical_reorder_by_handle(ui, name.clone(), |ui, _is_dragging| {
            show_preset_name(
                ui,
                &taken_preset_names,
                &current.base,
                name,
                &mut preset_to_activate,
                &mut preset_to_rename,
                &mut preset_to_delete,
            );
        });
    }

    if ui.button(l.actions.add).clicked() {
        let desired_name = match &current.base {
            Some(r) => r.name().preset,
            None => make_unique_filter_name(&filter_prefs.presets),
        };
        let name = filter_prefs
            .presets
            .save_preset_with_nonconflicting_name(&desired_name, current.current.inner.clone());
        preset_to_activate = Some(FilterPresetName::new(name));
        *changed = true;
    }

    ui.separator();

    let l = &L.presets.piece_filter_sequences;

    ui.horizontal(|ui| {
        ui.strong(l.saved_presets);
        HelpHoverWidget::show_right_aligned(ui, L.help.piece_filter_sequences);
    });

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    struct ForcedCollapsingState {
        /// State to set.
        state: bool,
        /// Whether to set the state on the next non-dragging frame.
        set: bool,
    }
    impl ForcedCollapsingState {
        fn current_state(self) -> Option<bool> {
            self.set.then_some(self.state)
        }
    }

    // Track whether each `CollapsingHeader`s should be kept open. By default,
    // open them.
    let saved_collapsing_states = EguiTempValue::<Vec<(PresetRef, ForcedCollapsingState)>>::new(ui);
    let mut collapsing_states: HashMap<String, ForcedCollapsingState> = filter_prefs
        .sequences
        .user_presets()
        .map(|p| {
            let default_state = ForcedCollapsingState {
                state: true,
                set: true,
            };
            (p.name().clone(), default_state)
        })
        .collect();
    for (k, v) in saved_collapsing_states.get().unwrap_or_default() {
        collapsing_states.insert(k.name(), v);
    }

    let is_any_dragging = seq_dnd.is_dragging();

    let taken_sequence_names = filter_prefs.sequences.taken_names();

    // Show filter sequences
    for seq_preset in filter_prefs.sequences.user_presets_mut() {
        let taken_preset_names = seq_preset.value.taken_names();

        let seq_ptr = seq_preset.new_ref().ptr();
        let seq_name = seq_preset.name().clone();
        let seq_list = &mut seq_preset.value;
        seq_dnd.vertical_reorder_by_handle(ui, seq_name.clone(), |ui, _is_dragging| {
            let r = egui::CollapsingHeader::new(&seq_name)
                .id_salt(seq_ptr)
                .open(
                    is_any_dragging
                        .then_some(false)
                        .or_else(|| collapsing_states.get(&seq_name)?.current_state()),
                )
                .show_unindented(ui, |ui| {
                    let l = &L.presets.piece_filters;

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
                                    &taken_preset_names,
                                    &current.base,
                                    name,
                                    &mut preset.value,
                                    &mut preset_to_activate,
                                    &mut preset_to_rename,
                                    &mut preset_to_delete,
                                    &fallback_style,
                                    changed,
                                    is_first,
                                );
                            },
                        );
                        is_first = false;
                    }

                    ui.horizontal(|ui| {
                        ui.add_space(ui.spacing().indent + ui.spacing().item_spacing.x + 29.0);
                        if ui.button(l.actions.add).clicked() {
                            let desired_name = match &current.base {
                                Some(r) => r.name().preset,
                                None => make_unique_step_name(seq_list),
                            };
                            let preset_name = seq_list.save_preset_with_nonconflicting_name(
                                &desired_name,
                                current.current.clone(),
                            );
                            preset_to_activate = Some(FilterPresetName {
                                seq: Some(seq_name.clone()),
                                preset: preset_name,
                            });
                            *changed = true;
                        }
                    });
                });

            if let Some(state) = collapsing_states.get_mut(&seq_name) {
                state.set = is_any_dragging;
                if !is_any_dragging {
                    if r.fully_open() {
                        state.state = true;
                    }
                    if r.fully_closed() {
                        state.state = false;
                    }
                }
            }

            let r = r.header_response.on_hover_ui(|ui| {
                md(ui, L.click_to.rename_or_delete.with(L.inputs.right_click));
                // No alt+click to delete because it's too easy to accidentally
                // delete a whole filter sequence while trying to delete a bunch
                // of presets.
            });

            // Right-click to rename
            let mut popup = TextEditPopup::new(ui);
            if r.secondary_clicked() {
                popup.open(seq_name.clone());
            }
            let popup_response = popup.if_open(|popup| {
                let validate_sequence_rename = |new_name: &str| {
                    let l = &l;
                    if new_name.is_empty() {
                        Err(Some(l.errors.empty_name.into()))
                    } else if taken_sequence_names.contains(new_name) {
                        Err(Some(l.errors.name_conflict.into()))
                    } else {
                        Ok(Some(l.actions.rename.into()))
                    }
                };

                popup
                    .text_edit_width(PRESET_NAME_TEXT_EDIT_WIDTH)
                    .text_edit_hint(l.new_name_hint)
                    .confirm_button_validator(&validate_sequence_rename)
                    .delete_button_validator(&|_| Ok(Some(l.actions.delete.into())))
                    .at(ui, &r, egui::vec2(-18.0, 0.85))
                    .show(ui)
            });
            if let Some(r) = popup_response {
                match r {
                    TextEditPopupResponse::Confirm(new_name) => {
                        seq_to_rename = Some((seq_name.clone(), new_name));
                    }
                    TextEditPopupResponse::Delete => seq_to_delete = Some(seq_name.clone()),
                    TextEditPopupResponse::Cancel => (),
                }
            }
        });
    }

    saved_collapsing_states.set(Some(
        collapsing_states
            .into_iter()
            .map(|(k, v)| (filter_prefs.sequences.new_ref(&k), v))
            .map(|(k, mut v)| {
                v.set |= seq_to_rename.is_some();
                (k, v)
            })
            .collect(),
    ));

    if ui.button(l.actions.add).clicked() {
        let seq_name = make_unique_filter_sequence_name(&filter_prefs.sequences);
        let preset_name = "Step 1".to_owned();
        let mut seq = PresetsList::default();
        seq.save_preset(preset_name.clone(), current.current.clone());
        filter_prefs.sequences.save_preset(seq_name.clone(), seq);
        preset_to_activate = Some(FilterPresetName {
            seq: Some(seq_name),
            preset: preset_name,
        });
        *changed = true;
    }

    *changed |= preset_dnd.end_reorder(ui, filter_prefs);
    *changed |= seq_dnd.end_reorder(ui, &mut filter_prefs.sequences);

    if let Some((old_name, new_name)) = preset_to_rename {
        filter_prefs.rename_preset(&old_name, &new_name);
        *changed = true;
    }
    if let Some(name) = preset_to_delete {
        filter_prefs.remove_preset(&name);
        *changed = true;
    }
    if let Some(name) = preset_to_activate {
        current.load_preset(filter_prefs, Some(&name));
    }

    if let Some((old_name, new_name)) = seq_to_rename {
        filter_prefs.sequences.rename(&old_name, &new_name);
        *changed = true;
    }
    if let Some(name) = seq_to_delete {
        filter_prefs.sequences.remove(&name);
        *changed = true;
    }

    if *changed {
        current.mark_changed();
    }
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
    fallback_style: &Option<PresetRef>,
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
                hover = L.piece_filters.include_previous;
            } else {
                label = "★";
                hover = L.piece_filters.ignore_previous;
            }

            let r = ui.selectable_label(false, label);
            if r.hovered() || r.has_focus() {
                // Unfortunately, egui tries to cache the tooltip size between
                // frames so we need to pick a widget ahead of time.
                //
                // TODO: this problem shows up in other areas as well, such as
                // the error message on the confirm button in the preset
                // renaming popup
                let w = f32::max(
                    text_width(ui, md_inline(ui, &L.piece_filters.include_previous)),
                    text_width(ui, md_inline(ui, &L.piece_filters.ignore_previous)),
                );

                r.show_tooltip_ui(|ui| {
                    ui.set_width(w.ceil());
                    md(ui, hover);
                })
            }
            if r.clicked() {
                value.include_previous ^= true;
                value.inner.fallback_style = if value.include_previous {
                    None
                } else {
                    fallback_style.clone()
                };
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

fn reserve_space_for_ad_hoc_preset_name(ui: &mut egui::Ui) -> egui::Rect {
    ui.scope_builder(egui::UiBuilder::new().sizing_pass().invisible(), |ui| {
        let _ = ui.selectable_label(false, "");
        ui.separator();
    })
    .response
    .rect
}

fn show_ad_hoc_preset_name(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    current: &Option<FilterPresetRef>,
) -> egui::Response {
    let rect = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), rect.y_range());
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        let r = ui
            .with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.selectable_label(current.is_none(), L.piece_filters.tabs.ad_hoc)
            })
            .inner;
        ui.separator();
        r
    })
    .inner
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
    let l = &L.presets.piece_filters;

    let is_active = current.as_ref().is_some_and(|r| r.name() == name);

    let r = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        ui.selectable_label(is_active, &name.preset)
    });

    let r = r.inner.on_hover_ui(|ui| {
        md(ui, L.click_to.activate.with(L.inputs.click));
        md(ui, L.click_to.rename.with(L.inputs.right_click));
        crate::gui::md_middle_click_to_delete(ui);
    });

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
            .text_edit_hint(l.new_name_hint)
            .confirm_button_validator(&validate_preset_rename)
            .delete_button_validator(&|_| Ok(Some(l.actions.delete.into())))
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
    if crate::gui::middle_clicked(ui, &r) {
        *to_delete = Some(name.clone());
    }
}

fn show_current_filter_preset_ui(ui: &mut egui::Ui, app: &mut App) {
    app.active_puzzle_view.with_opt(|p| {
        if let Some(p) = p {
            show_current_filter_preset_ui_contents(ui, &mut app.prefs, p);
        } else {
            ui.label("No active puzzle");
        }
    })
}
fn show_current_filter_preset_ui_contents(
    ui: &mut egui::Ui,
    prefs: &mut Preferences,
    p: &mut PuzzleWidget,
) {
    ui.set_min_width(CURRENT_PRESET_MIN_WIDTH);

    let mut style_options = vec![(None, crate::DEFAULT_STYLE_NAME.into())];
    for style in prefs.custom_styles.user_presets() {
        style_options.push((Some(style.new_ref()), style.name().to_owned().into()));
    }

    let puz = p.puzzle();
    let filter_prefs = prefs.filters_mut(&puz);

    if let Some(preset_ref) = &p.view.filters.base {
        if !filter_prefs.has_preset(&preset_ref.name()) {
            p.view.filters.base = None;
        }
    }

    let preset_name = p.view.filters.base.as_ref().map(|r| r.to_string());

    ui.add(PresetHeaderUi::<()> {
        text: &L.presets.piece_filters,
        preset_name: preset_name.as_deref().unwrap_or(""),

        help_contents: Some(L.help.piece_filters),
        yaml: None,
        save_status: PresetSaveStatus::Autosave,

        save_preset: &mut false,
    });

    let puz = p.puzzle();

    let mut changed = false;
    let mut changed_include_previous = false;

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .id_salt("filter_preset_rules")
        .show(ui, |ui| {
            let active_rules = &mut p.view.filters.active_rules;
            let current = &mut p.view.filters.current;

            active_rules.resize(current.inner.rules.len(), true);

            let mut dnd = DragAndDrop::new(ui);
            let is_any_dragging = dnd.is_dragging();

            let mut remaining_pieces = PieceMask::new_full(puz.pieces.len());
            let mut to_delete = None;

            let rules_iter = current.inner.rules.iter_mut().enumerate();
            let rules_iter: Box<dyn Iterator<Item = (usize, &mut FilterRule)>> =
                match prefs.interaction.reverse_filter_rules {
                    true => Box::new(rules_iter.rev()),
                    false => Box::new(rules_iter),
                };

            for (i, rule) in rules_iter {
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
                                md_inline(ui, L.piece_filters.show_n_pieces_with_style.with(&n)),
                            );
                            changed |= r.changed();

                            let r = &ui.add(FancyComboBox::new(
                                unique_id!(i),
                                &mut rule.style,
                                &style_options,
                            ));
                            changed |= r.changed();
                        });
                        let previous_rule_piece_count = these_pieces.len() - affected_piece_count;
                        if previous_rule_piece_count > 0 {
                            // TODO: singular vs. plural
                            let n = previous_rule_piece_count.to_string();
                            if prefs.interaction.reverse_filter_rules {
                                md(ui, L.piece_filters.n_override_previous_rule.with(&n));
                            } else {
                                md(ui, L.piece_filters.n_match_previous_rule.with(&n));
                            }
                        }

                        match &mut rule.set {
                            FilterPieceSet::Expr(expr) => {
                                let r = &show_filter_expr_ui(ui, i, expr, &puz);
                                changed |= r.changed();
                            }
                            FilterPieceSet::Checkboxes(checkboxes) => {
                                let expr_string = checkboxes.to_string(&*puz);
                                let r = egui::CollapsingHeader::new(&expr_string)
                                    .id_salt(unique_id!(i))
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
                                            prefs,
                                            &mut changed,
                                        );
                                    });
                                r.header_response.context_menu(|ui| {
                                    if ui.button(L.piece_filters.convert_to_text_rule).clicked() {
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
                changed = true;
                current.inner.rules.remove(i);
            }

            changed |= dnd.end_reorder(ui, &mut current.inner.rules);

            ui.horizontal_wrapped(|ui| {
                if ui.button(L.piece_filters.add_checkboxes_rule).clicked() {
                    changed = true;
                    current.inner.rules.push(FilterRule::new_checkboxes());
                }
                if ui.button(L.piece_filters.add_text_rule).clicked() {
                    changed = true;
                    current.inner.rules.push(FilterRule::new_expr());
                }
            });

            ui.separator();

            if p.view
                .filters
                .base
                .as_ref()
                .is_some_and(|r| r.seq.is_some())
            {
                let r = ui.checkbox(
                    &mut current.include_previous,
                    L.piece_filters.show_remaining_pieces_with_previous_filter,
                );
                if r.clicked() {
                    changed = true;
                    changed_include_previous = true;
                    if current.include_previous {
                        current.inner.fallback_style = None;
                    } else {
                        current.inner.fallback_style = prefs.first_custom_style();
                    }
                }
            }

            let mut ui_builder = egui::UiBuilder::new();
            if current.include_previous {
                ui_builder = ui_builder.invisible();
            }
            ui.scope_builder(ui_builder, |ui| {
                ui.horizontal(|ui| {
                    ui.label(L.piece_filters.show_remaining_peices_with_style);
                    let r = ui.add(FancyComboBox {
                        combo_box: egui::ComboBox::from_id_salt(unique_id!()),
                        selected: &mut current.inner.fallback_style,
                        options: style_options.clone(),
                    });
                    changed |= r.changed();
                });
            });
        });

    if changed {
        p.view.filters.mark_changed();

        let filter_prefs = prefs.filters_mut(&puz);
        if let Some(preset_ref) = &p.view.filters.base {
            filter_prefs.save_preset(&preset_ref.name(), p.view.filters.current.clone());
            prefs.needs_save = true;
        }
    }

    if changed_include_previous {
        // Update fallback *after* saving changes to preferences.
        p.view
            .filters
            .update_combined_fallback_preset(prefs.filters_mut(&puz));
    }
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
                .id_salt(unique_id!(i))
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
                        .color(rgb.to_egui_color32())
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

    show_piece_type_hierarchy(
        unique_id!(&id),
        ui,
        L.piece_filters.piece_types,
        &puzzle.piece_type_hierarchy,
        &mut filters.piece_types,
        true,
        changed,
    );
}

fn get_common_state<'a>(
    states: impl IntoIterator<Item = &'a Option<bool>>,
) -> Option<Option<bool>> {
    states.into_iter().all_equal_value().ok().cloned()
}

fn show_piece_type_hierarchy(
    id: egui::Id,
    ui: &mut egui::Ui,
    name: &str,
    hierarchy: &PieceTypeHierarchy,
    filter_states: &mut PerPieceType<Option<bool>>,
    is_root: bool,
    changed: &mut bool,
) {
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        egui::Id::new(id.with(hierarchy as *const _)),
        is_root,
    )
    .show_header(ui, |ui| {
        let mut common_state =
            get_common_state(hierarchy.types.iter().map(|ty| &filter_states[ty]));
        let r = ui.add(FilterCheckbox::new(
            FilterCheckboxAllowedStates::NeutralHide,
            common_state.as_mut(),
            name,
        ));
        *changed |= r.changed();
        if r.changed() {
            for ty in hierarchy.types.iter() {
                filter_states[ty] = common_state.flatten();
            }
        }
    })
    .body(|ui| {
        for (k, node) in &hierarchy.nodes {
            let name = node.display.as_ref().unwrap_or(k);
            match &node.contents {
                hyperpuzzle::PieceTypeHierarchyNodeContents::Category(cat) => {
                    show_piece_type_hierarchy(id, ui, name, &cat, filter_states, false, changed);
                }
                hyperpuzzle::PieceTypeHierarchyNodeContents::Type(ty) => {
                    let r = &ui.add(
                        FilterCheckbox::new(
                            FilterCheckboxAllowedStates::NeutralHide,
                            Some(&mut filter_states[*ty]),
                            name,
                        )
                        .indent(),
                    );
                    *changed |= r.changed();
                }
            }
        }
    });
}

fn make_unique_filter_name(presets: &PresetsList<FilterPreset>) -> String {
    (1..)
        .map(|i| format!("Filter preset {i}"))
        .find(|s| !presets.contains_key(s))
        .expect("ran out of filter preset names!")
}
fn make_unique_step_name(seq_list: &PresetsList<FilterSeqPreset>) -> String {
    (1..)
        .map(|i| format!("Step {i}"))
        .find(|s| !seq_list.contains_key(s))
        .expect("ran out of filter preset names!")
}
fn make_unique_filter_sequence_name(
    sequences: &PresetsList<PresetsList<FilterSeqPreset>>,
) -> String {
    (1..)
        .map(|i| format!("Filter sequence {i}"))
        .find(|s| !sequences.contains_key(s))
        .expect("ran out of filter sequence names!")
}

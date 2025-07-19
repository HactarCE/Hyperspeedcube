use std::sync::Arc;

use hyperprefs::Preferences;
use hyperpuzzle::prelude::*;
use hyperpuzzle_view::{NdEuclidViewState, PuzzleView};
use itertools::Itertools;

use crate::L;
use crate::app::App;
use crate::gui::components::{DragAndDrop, color_assignment_popup};
use crate::gui::markdown::{md, md_bold_user_text};
use crate::gui::util::{EguiTempFlag, EguiTempValue};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum DevToolsTab {
    #[default]
    HoverInfo,
    HpsGenerator,
    Linter,
}

#[derive(Debug, Default, Clone)]
struct DevToolsState {
    puzzle: Option<Arc<Puzzle>>,

    current_tab: DevToolsTab,

    loaded_orbit: Option<AnyOrbit>,
    names_and_order: Vec<(usize, String)>,

    lint_results: Vec<PuzzleLintOutput>,
}

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let egui_stored_state = EguiTempValue::<DevToolsState>::new(ui);

    let mut state = egui_stored_state.get().unwrap_or_default();

    ui.group(|ui| {
        ui.set_min_size(ui.available_size());

        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut state.current_tab,
                DevToolsTab::HoverInfo,
                L.dev.hover_info,
            );
            ui.selectable_value(
                &mut state.current_tab,
                DevToolsTab::HpsGenerator,
                L.dev.hps_generator,
            );
            ui.selectable_value(&mut state.current_tab, DevToolsTab::Linter, L.dev.linter);
        });

        ui.separator();

        match state.current_tab {
            DevToolsTab::HoverInfo => app
                .active_puzzle
                .with_view(|view| show_hover_info(ui, view))
                .unwrap_or_else(|| {
                    ui.label("No active puzzle");
                }),
            DevToolsTab::HpsGenerator => show_hps_generator(ui, app, &mut state),
            DevToolsTab::Linter => show_linter(ui, &mut state),
        };
    });

    egui_stored_state.set(Some(state));
}

fn show_hover_info(ui: &mut egui::Ui, view: &PuzzleView) {
    if let Some(nd_euclid) = view.nd_euclid() {
        show_nd_euclid_hover_info(ui, view, nd_euclid);
    }
}

fn show_nd_euclid_hover_info(ui: &mut egui::Ui, view: &PuzzleView, euclid: &NdEuclidViewState) {
    let info_line = |ui: &mut egui::Ui, label: &str, text: &str| {
        md(ui, format!("{label} = {}", md_bold_user_text(text)))
    };

    let name_spec_info_lines = |ui: &mut egui::Ui, label: &str, name: &NameSpec| {
        info_line(ui, &format!("{label} name"), &name.canonical);
        info_line(ui, &format!("{label} name spec"), &name.spec);
        info_line(ui, &format!("{label} canonical name"), &name.canonical);
    };

    let puz = view.puzzle();

    if let Some(piece) = view.hovered_piece() {
        ui.strong(format!("Piece {piece}"));
        let piece_info = &puz.pieces[piece];
        info_line(ui, "Sticker count", &piece_info.stickers.len().to_string());
        ui.label("");
        ui.strong(format!("Piece type {}", piece_info.piece_type));
        let piece_type_info = &puz.piece_types[piece_info.piece_type];
        info_line(ui, "Piece type name", &piece_type_info.name);
    }
    if let Some(sticker) = view.hovered_sticker() {
        ui.label("");
        ui.strong(format!("Sticker {sticker}"));
        let sticker_info = &puz.stickers[sticker];
        ui.label("");
        ui.strong(format!("Color {}", sticker_info.color));
        if let Ok(name) = puz.colors.names.get(sticker_info.color) {
            name_spec_info_lines(ui, "Color", name);
        }
        if let Ok(display) = puz.colors.display_names.get(sticker_info.color) {
            info_line(ui, "Color display", display);
        }
    }

    if let Some(hov) = view.hovered_gizmo().filter(|_| view.show_gizmo_hover) {
        ui.strong(format!("Gizmo {}", hov.gizmo_face));
        info_line(ui, "Backface?", &hov.backface.to_string());
        info_line(ui, "Z", &format!("{:.3}", hov.z));
        let geom = puz
            .ui_data
            .downcast_ref::<NdEuclidPuzzleUiData>()
            .expect("expected NdEuclidPuzzleGeometry")
            .geom();
        let twist = geom.gizmo_twists[hov.gizmo_face];

        ui.label("");
        ui.strong(format!("Twist {twist}"));
        if let Ok(name) = puz.twists.names.get(twist) {
            name_spec_info_lines(ui, "Twist", name);
        }
        let twist_info = &puz.twists.twists[twist];
        let rev = twist_info.reverse;
        info_line(ui, "Reverse twist", &rev.to_string());
        info_line(ui, "Reverse twist name", &puz.twists.names[rev]);
        info_line(ui, "QTM", &twist_info.qtm.to_string());

        ui.label("");
        ui.strong(format!("Axis {}", twist_info.axis));
        if let Ok(name) = puz.axes().names.get(twist_info.axis) {
            name_spec_info_lines(ui, "Axis", name);
        }
        let axis_layers = &puz.axis_layers[twist_info.axis];
        info_line(ui, "Layer count", &axis_layers.len().to_string());
    }
}

fn show_hps_generator(ui: &mut egui::Ui, app: &mut App, state: &mut DevToolsState) {
    ui.with_layout(
        egui::Layout::top_down_justified(egui::Align::Center),
        |ui| {
            if !app.active_puzzle.has_puzzle() {
                ui.disable();
            }

            let r = &ui.button("Copy color system definition");
            app.active_puzzle.with_view(|view| {
                let text_to_copy = r
                    .clicked()
                    .then(|| color_system_to_hps_code(&view.puzzle().colors, &app.prefs));
                crate::gui::components::copy_on_click(ui, r, text_to_copy);
            });

            ui.separator();

            if state.loaded_orbit.is_none() {
                ui.menu_button("Load orbit from current puzzle", |ui| {
                    app.active_puzzle.with_view(|view| {
                        let puz = view.puzzle();
                        for (i, orbit) in puz.orbits().iter().enumerate() {
                            if ui
                                .button(format!(
                                    "#{} - {} ({})",
                                    i + 1,
                                    orbit.description(),
                                    orbit.first_name(&puz),
                                ))
                                .clicked()
                            {
                                state.puzzle = Some(Arc::clone(&puz));
                                state.loaded_orbit = Some(orbit.clone());
                                state.names_and_order = orbit.sorted_ids_and_names(&puz);
                            }
                        }
                    });
                });
            } else {
                ui.columns(2, |uis| {
                    let Some(loaded_orbit) = &mut state.loaded_orbit else {
                        return;
                    };

                    uis[0].menu_button("Copy Hps code", |ui| {
                        let r = ui.button("Compact");
                        let text_to_copy = r.clicked().then(|| {
                            hyperpuzzlescript::codegen::orbit_hps_code(
                                loaded_orbit,
                                &state.names_and_order,
                                true,
                            )
                        });
                        crate::gui::components::copy_on_click(ui, &r, text_to_copy);

                        let r = ui.button("Expanded");
                        let text_to_copy = r.clicked().then(|| {
                            hyperpuzzlescript::codegen::orbit_hps_code(
                                loaded_orbit,
                                &state.names_and_order,
                                false,
                            )
                        });
                        crate::gui::components::copy_on_click(ui, &r, text_to_copy);
                    });

                    if uis[1].button("Clear orbit").clicked() {
                        *state = Default::default();
                        state.current_tab = DevToolsTab::HpsGenerator;
                    }
                });
            }

            ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| show_orbit_list(ui, app, state));
        },
    );
}

fn show_orbit_list(ui: &mut egui::Ui, app: &mut App, state: &mut DevToolsState) {
    let Some(puz) = state.puzzle.as_ref().map(Arc::clone) else {
        return;
    };

    let mut dnd = DragAndDrop::new(ui);
    for (i, (index, name)) in state.names_and_order.iter_mut().enumerate() {
        dnd.vertical_reorder_by_handle(ui, i, |ui, _is_dragging| {
            let Some(loaded_orbit) = &state.loaded_orbit else {
                return;
            };

            let text_edit = egui::TextEdit::singleline(name);
            match &state.loaded_orbit {
                Some(AnyOrbit::Axes(orbit)) => {
                    if let Some(axis) = orbit.elements[*index] {
                        let r = ui.add(text_edit);
                        if r.hovered() || r.has_focus() {
                            app.active_puzzle.with_view(|view| {
                                if Arc::ptr_eq(&view.puzzle(), &puz) {
                                    view.temp_gizmo_highlight = Some(axis);
                                }
                            });
                        }
                    }
                }
                Some(AnyOrbit::Colors(orbit)) => {
                    if let Some(color) = orbit.elements[*index] {
                        show_orbit_color(app, &puz, ui, text_edit, color);
                    }
                }
                None => (),
            }
        });
    }
    let _ = dnd.end_reorder(ui, &mut state.names_and_order);
}

fn show_orbit_color(
    app: &mut App,
    puz: &Arc<Puzzle>,
    ui: &mut egui::Ui,
    text_edit: egui::TextEdit<'_>,
    color: Color,
) {
    ui.horizontal(|ui| {
        app.active_puzzle.with_view(|view| {
            if Arc::ptr_eq(&view.puzzle(), puz) {
                puzzle_color_edit_button(ui, view, &app.prefs, color);
            }
        });
        let r = ui.add(text_edit);
        if r.hovered() {
            app.active_puzzle.with_view(|view| {
                if Arc::ptr_eq(&view.puzzle(), puz) {
                    let orig_color = view.get_rgb_color(color, &app.prefs).unwrap_or_default();
                    let t = hyperpuzzle::Timestamp::now().subsec_nanos() as f32 / 1_000_000_000.0;
                    let contrasting = crate::util::contrasting_text_color(orig_color.into()).into();

                    view.temp_colors
                        .get_or_insert_with(|| view.colors.value.clone())
                        .insert(
                            puz.colors.names[color].to_owned(),
                            DefaultColor::HexCode {
                                rgb: hyperpuzzle::Rgb::mix(
                                    contrasting,
                                    orig_color,
                                    (0.5 - t).abs(),
                                ),
                            },
                        );
                    ui.ctx().request_repaint();
                }
            });
        }
    });
}

fn puzzle_color_edit_button(
    ui: &mut egui::Ui,
    puzzle_view: &mut PuzzleView,
    prefs: &Preferences,
    color: Color,
) {
    let Some(rgb) = puzzle_view.get_rgb_color(color, prefs) else {
        return;
    };

    let popup_id = ui.next_auto_id().with("color_edit_popup");

    let r = crate::gui::components::show_color_button(
        ui,
        rgb,
        ui.memory(|mem| mem.is_popup_open(popup_id)),
        ui.spacing().interact_size,
        egui::Sense::click(),
    );

    if r.clicked() {
        ui.memory_mut(|mem| mem.open_popup(popup_id));
    }

    let close_behavior = egui::PopupCloseBehavior::CloseOnClickOutside;
    let puzzle = puzzle_view.puzzle();
    egui::popup_below_widget(ui, popup_id, &r, close_behavior, |ui| {
        color_assignment_popup(ui, puzzle_view, &prefs.color_palette, Some(color));
    });
}

fn color_system_to_hps_code(color_system: &ColorSystem, prefs: &Preferences) -> String {
    use hyperprefs::MODIFIED_SUFFIX;

    let id_string_literal = hyperpuzzlescript::codegen::to_str_literal(&color_system.id);
    let name_string_literal = format!("{:?}", color_system.name); // escape using double quotes
    let mut default_scheme = hyperpuzzle::DEFAULT_COLOR_SCHEME_NAME.to_string();

    let mut schemes = color_system.schemes.clone();
    if let Some(custom_schemes) = prefs.color_schemes.get(color_system) {
        for scheme in custom_schemes.schemes.user_presets() {
            let name = scheme
                .name()
                .strip_suffix(MODIFIED_SUFFIX)
                .unwrap_or(scheme.name())
                .to_string();
            schemes.insert(name, scheme.value.values().cloned().collect());
        }

        default_scheme = custom_schemes.schemes.last_loaded_name().clone();
        if let Some(original_name) = default_scheme.strip_suffix(MODIFIED_SUFFIX) {
            default_scheme = original_name.to_string();
        }
    }

    let mut s = "add_color_system(\n".to_owned();

    s += &format!("    id = {id_string_literal},\n");
    s += &format!("    name = {name_string_literal},\n");

    let has_default_colors = schemes.len() == 1;

    let color_name_kv_pairs = pad_to_common_length(color_system.names.iter_values().map(|info| {
        let string_literal = hyperpuzzlescript::codegen::to_str_literal(&info.spec);
        format!(" name = {string_literal},")
    }));
    let color_display_kv_pairs = pad_to_common_length(
        color_system
            .display_names
            .iter_values()
            .map(|display_name| format!(" display = {display_name:?},")),
    );
    let default_color_kv_pairs =
        schemes
            .get_index(0)
            .filter(|_| has_default_colors)
            .map(|(_name, default_colors)| {
                default_colors
                    .iter_values()
                    .map(|default_color| {
                        let default_color_string = default_color.to_string();
                        format!(" default = {default_color_string:?}")
                    })
                    .collect_vec()
            });

    s += "\n    colors = [\n";
    for i in 0..color_system.names.len() {
        s += "        {";
        s += &color_name_kv_pairs[i];
        if has_default_colors {
            s += &color_display_kv_pairs[i];
        } else {
            s += color_display_kv_pairs[i]
                .trim_end()
                .strip_suffix(',')
                .expect("no trailing comma");
        }
        if let Some(kv_pairs) = &default_color_kv_pairs {
            s += &kv_pairs[i];
        }
        s += " },\n";
    }
    s += "    ],\n";

    if !has_default_colors {
        s += "\n    schemes = [\n";
        for (name, colors) in &schemes {
            s += &format!("        {{{name:?}, {{\n");
            for (k, v) in colors {
                let k = hyperpuzzlescript::codegen::to_map_key(match color_system.names.get(k) {
                    Ok(name) => &name.spec,
                    Err(_) => "?",
                });
                let v = v.to_string();
                s += &format!("          {k} = {v:?},\n");
            }
            s += "        }},\n";
        }
        s += "    ],\n";
        s += &format!("    default = {default_scheme:?},\n");
    }

    s += ")\n";
    s
}

fn pad_to_common_length(strings: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut ret = strings.into_iter().collect_vec();
    let max_len = ret.iter().map(|s| s.len()).max().unwrap_or(0);
    for s in &mut ret {
        s.reserve_exact(max_len - s.len());
        while s.len() < max_len {
            s.push(' ');
        }
    }
    ret
}

fn show_linter(ui: &mut egui::Ui, state: &mut DevToolsState) {
    ui.horizontal(|ui| {
        if ui.button("Lint all puzzles").clicked() {
            state.lint_results = hyperpuzzle::catalog()
                .puzzle_specs()
                .iter()
                .map(PuzzleLintOutput::from_spec)
                .filter(|lint| !lint.all_good())
                .collect();
        }
    });

    let show_experimental_saved = EguiTempFlag::new(ui);
    let mut show_experimental = show_experimental_saved.get();

    let mut override_state = None;
    ui.horizontal(|ui| {
        if ui.button("Collapse all").clicked() {
            override_state = Some(false);
        }
        if ui.button("Expand all").clicked() {
            override_state = Some(true);
        }
        if ui
            .checkbox(&mut show_experimental, "Show experimental")
            .changed()
        {
            match show_experimental {
                true => show_experimental_saved.set(),
                false => show_experimental_saved.reset(),
            };
        }
    });

    ui.separator();

    for lint in &state.lint_results {
        if !show_experimental && lint.puzzle.meta.tags.is_experimental() {
            continue;
        }

        egui::CollapsingHeader::new(&lint.puzzle.meta.name)
            .id_salt(&lint.puzzle.meta.id)
            .default_open(false)
            .open(override_state)
            .show_background(true)
            .show(ui, |ui| {
                let PuzzleLintOutput {
                    puzzle: _,
                    schema,
                    missing_tags,
                } = lint;

                if lint.all_good() {
                    ui.label("All good!");
                }

                let latest_schema = hyperpuzzle::TAGS.schema;
                if *schema != latest_schema {
                    ui.label(format!(
                        "Tags use schema {schema} but latest is {latest_schema}"
                    ));
                }
                if !missing_tags.is_empty() {
                    if ui.button("Copy Hps code to exclude tags").clicked() {
                        let text = missing_tags
                            .iter()
                            .filter_map(|tag| tag.iter().exactly_one().ok())
                            .map(|tag| format!("'!{tag}',\n"))
                            .join("");
                        ui.ctx().copy_text(text);
                    }
                    egui::CollapsingHeader::new("Missing tags")
                        .id_salt((&lint.puzzle.meta.id, "missing_tags"))
                        .default_open(true)
                        .show(ui, |ui| {
                            let markdown_text = missing_tags
                                .iter()
                                .map(|tag_set| {
                                    let line = tag_set
                                        .iter()
                                        .map(|tag_name| format!("`{tag_name}`"))
                                        .join(" OR ");
                                    format!("- {line}")
                                })
                                .join("\n");
                            md(ui, markdown_text);
                        });
                }
            });
    }
}

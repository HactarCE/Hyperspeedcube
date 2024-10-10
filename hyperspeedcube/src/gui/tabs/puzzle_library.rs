use hyperpuzzle::{
    lua::{GeneratorParamType, GeneratorParamValue, PuzzleGeneratorSpec},
    TagValue,
};
use itertools::Itertools;

use crate::{
    app::App,
    gui::{
        markdown::{md, md_escape, md_inline},
        util::EguiTempValue,
    },
    L,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let stored_search_query = EguiTempValue::new(ui);
    let mut search_query: String = stored_search_query.get().unwrap_or_default();

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    enum SubTab {
        #[default]
        Puzzles,
        Generators,
    }

    let subtab_stored = EguiTempValue::new(ui);
    let mut subtab = subtab_stored.get().unwrap_or_default();

    ui.group(|ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.selectable_value(&mut subtab, SubTab::Puzzles, "Puzzles");
            ui.selectable_value(&mut subtab, SubTab::Generators, "Puzzle generators");
            subtab_stored.set(Some(subtab));

            if crate::paths::lua_dir().is_ok() {
                if ui.button(L.library.reload_all_files).clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::F5))
                {
                    crate::reload_user_puzzles();
                }
            }
        });
    });

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.menu_button("Filter", |ui| {
                egui::ScrollArea::vertical().show(ui, show_tags_filter_menu);
            });

            ui.add(egui::TextEdit::singleline(&mut search_query).desired_width(f32::INFINITY));
            stored_search_query.set(Some(search_query.clone()));
        });

        ui.add_space(ui.spacing().item_spacing.y);

        crate::LIBRARY.with(|lib| match subtab {
            SubTab::Puzzles => {
                filtered_list(
                    ui,
                    &search_query,
                    &lib.puzzles()
                        .into_iter()
                        .map(|p| (p.display_name().to_owned(), p))
                        .collect_vec(),
                    "puzzles",
                    |_ui, mut r, puzzle| {
                        r = r.on_hover_ui(|ui| {
                            fn comma_list(strings: &[String]) -> String {
                                md_escape(&strings.iter().join(", ")).into_owned()
                            }

                            ui.strong(puzzle.display_name());

                            let inventors = puzzle.inventors();
                            if !inventors.is_empty() {
                                md(ui, &format!("**Inventors:** {}", comma_list(inventors)));
                            }

                            let authors = puzzle.authors();
                            if !authors.is_empty() {
                                md(ui, &format!("**Authors:** {}", comma_list(authors)));
                            }

                            let aliases = puzzle.aliases();
                            if !aliases.is_empty() {
                                md(ui, &format!("**Aliases:** {}", comma_list(aliases)));
                            }

                            if let Some(url) = puzzle.wca_url() {
                                ui.hyperlink_to("WCA leaderboards", url);
                            }
                        });
                        if r.clicked() {
                            crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle.id));
                        }
                    },
                );
            }
            SubTab::Generators => {
                let popup_data_stored = EguiTempValue::<PuzzleGeneratorPopupData>::new(ui);
                let mut popup_data = popup_data_stored.get();

                let puzzle_generators = lib
                    .puzzle_generators()
                    .into_iter()
                    .map(|g| (g.display_name().to_owned(), g))
                    .collect_vec();
                filtered_list(
                    ui,
                    &search_query,
                    &puzzle_generators,
                    "puzzle generators",
                    |ui, r, puzzle_generator| {
                        let popup_id = popup_data_stored.id.with(&puzzle_generator.id);

                        if r.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            popup_data = Some(PuzzleGeneratorPopupData::new(puzzle_generator));
                        }

                        let close_behavior = egui::PopupCloseBehavior::CloseOnClickOutside;
                        if let Some(popup_data) = popup_data
                            .as_mut()
                            .filter(|data| data.id == puzzle_generator.id)
                        {
                            egui::popup_below_widget(ui, popup_id, &r, close_behavior, |ui| {
                                ui.strong(puzzle_generator.display_name());

                                for (param, value) in
                                    std::iter::zip(&puzzle_generator.params, &mut popup_data.params)
                                {
                                    match param.ty {
                                        GeneratorParamType::Int { min, max } => {
                                            let GeneratorParamValue::Int(i) = value;
                                            ui.horizontal(|ui| {
                                                ui.add(egui::Slider::new(i, min..=max));
                                                ui.label(&param.name);
                                            });
                                        }
                                    }
                                }

                                if ui.button("Generate puzzle").clicked() {
                                    ui.memory_mut(|mem| mem.close_popup());
                                    let puzzle_id = hyperpuzzle::generated_puzzle_id(
                                        &puzzle_generator.id,
                                        &popup_data.params,
                                    );
                                    crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle_id));
                                };
                            });
                        }
                    },
                );

                popup_data_stored.set(popup_data);
            }
        });
    });
}

fn fuzzy_match(ui: &egui::Ui, query: &str, s: &str) -> Option<(egui::WidgetText, isize)> {
    let m = sublime_fuzzy::best_match(query, s)?;

    let mut md_source = String::new();
    let mut i = 0;
    for c in m.continuous_matches() {
        md_source += &md_escape(&s[i..c.start()]);
        md_source += "**";
        md_source += &md_escape(&s[c.start()..c.start() + c.len()]);
        md_source += "**";
        i = c.start() + c.len();
    }
    md_source += &md_escape(&s[i..]);
    Some((md_inline(ui, md_source).into(), -m.score()))
}

fn filtered_list<'a, T>(
    ui: &mut egui::Ui,
    query: &str,
    candidates: &[(String, T)],
    plural_noun: &str,
    mut handle_resp: impl FnMut(&mut egui::Ui, egui::Response, &T),
) {
    let filtered_candidates: Vec<(egui::WidgetText, &T)> = if query.is_empty() {
        candidates
            .iter()
            .map(|(s, value)| (s.into(), value))
            .collect()
    } else {
        candidates
            .iter()
            .flat_map(|(s, value)| Some((fuzzy_match(ui, query, s)?, value)))
            .sorted_by_key(|((_text, score), _value)| *score)
            .map(|((text, _score), value)| (text, value))
            .collect()
    };

    let n = filtered_candidates.len();
    let m = candidates.len();
    if query.is_empty() {
        md(ui, &format!("_**{m}** {plural_noun}_"));
    } else {
        md(ui, &format!("_**{n}/{m}** {plural_noun}_"));
    }

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                for (s, obj) in filtered_candidates {
                    let r = ui.selectable_label(false, s);
                    handle_resp(ui, r, obj);
                }
            })
        });
}

fn show_tags_filter_menu(ui: &mut egui::Ui) {
    for node in &*hyperpuzzle::TAGS_MENU {
        show_tags_recursive(ui, node);
    }
}

fn show_tags_recursive(ui: &mut egui::Ui, node: &hyperpuzzle::TagMenuNode) {
    match node {
        hyperpuzzle::TagMenuNode::Heading(s) => {
            ui.strong(*s);
        }
        hyperpuzzle::TagMenuNode::Separator => {
            ui.separator();
        }
        hyperpuzzle::TagMenuNode::Tag {
            name,
            display,
            ty: _,
            subtags: children,

            section,
            checkbox,
            flatten,
            hidden,
            expected: _,
            list,
        } => {
            if *hidden {
                return;
            }

            if name.as_ref().is_some_and(|&name| {
                crate::LIBRARY.with(|lib| {
                    !lib.puzzles()
                        .iter()
                        .any(|puzzle| match puzzle.tags.get(name) {
                            None | Some(TagValue::False) => false,
                            _ => true,
                        })
                })
            }) {
                return;
            }

            if *list {
                tag_checkbox_menu(ui, *&display, |ui| {
                    match name {
                        Some("colors/system") => crate::LIBRARY.with(|lib| {
                            let mut color_systems = lib.color_systems();
                            color_systems.sort_by_key(|cs| cs.display_name().to_owned()); // TODO: precompute this or something!
                            for color_system in color_systems {
                                tag_checkbox(ui, color_system.display_name());
                            }
                        }),
                        Some("author") => crate::LIBRARY.with(|lib| {
                            for author in lib
                                .puzzles()
                                .iter()
                                .flat_map(|puzzle| puzzle.authors())
                                .unique()
                                .sorted()
                            {
                                tag_checkbox(ui, &author);
                            }
                        }),
                        Some("inventor") => crate::LIBRARY.with(|lib| {
                            for inventor in lib
                                .puzzles()
                                .iter()
                                .flat_map(|puzzle| puzzle.inventors())
                                .unique()
                                .sorted()
                            {
                                tag_checkbox(ui, &inventor);
                            }
                        }),
                        _ => {
                            ui.colored_label(ui.visuals().error_fg_color, "invalid auto tag");
                        }
                    }
                });
                return;
            }

            let show_contents = |ui: &mut egui::Ui| {
                for child in children {
                    show_tags_recursive(ui, child);
                }
            };

            if *flatten {
                show_contents(ui)
            } else if *section {
                ui.strong(*display);
                show_contents(ui);
            } else if *checkbox {
                if children.is_empty() {
                    tag_checkbox(ui, *display);
                } else {
                    tag_checkbox_menu(ui, *display, show_contents);
                }
            } else {
                tag_submenu(ui, *display, show_contents);
            }
        }
    }
}

fn tag_checkbox(ui: &mut egui::Ui, s: &str) {
    // ui.horizontal(|ui| {
    let r = ui.checkbox(&mut false, s);
    r.on_hover_ui(|ui| {
        md(
            ui,
            "**15** puzzles matching current filter\n\n**30** puzzles total",
        );
    });
    // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
    //     ui.label(format!("{}", r()));
    // });
    // });
}

fn tag_checkbox_menu(ui: &mut egui::Ui, s: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut false, "");
        ui.add_space(-10.0);
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.menu_button(s, contents)
        });
    });
}
fn tag_submenu(ui: &mut egui::Ui, s: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.menu_button(s, contents);
}

#[derive(Debug, Default, Clone, PartialEq)]
struct PuzzleGeneratorPopupData {
    id: String,
    params: Vec<GeneratorParamValue>,
}
impl PuzzleGeneratorPopupData {
    fn new(puzzle_generator: &PuzzleGeneratorSpec) -> Self {
        Self {
            id: puzzle_generator.id.clone(),
            params: puzzle_generator
                .params
                .iter()
                .map(|param| param.default.clone())
                .collect(),
        }
    }
}

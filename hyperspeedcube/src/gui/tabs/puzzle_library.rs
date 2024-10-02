use std::sync::Arc;

use hyperpuzzle::lua::{GeneratorParamType, GeneratorParamValue, PuzzleGeneratorSpec};
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
                let single_puzzles = lib.puzzles();
                let puzzle_generators = lib.puzzle_generators();
                let generated_puzzles = puzzle_generators
                    .iter()
                    .flat_map(|gen| gen.examples.values())
                    .map(Arc::clone);
                let puzzles = itertools::chain(single_puzzles, generated_puzzles)
                    .sorted()
                    .map(|p| (p.display_name().to_owned(), p))
                    .collect_vec();
                filtered_list(ui, &search_query, &puzzles, "puzzles", |_ui, r, puzzle| {
                    if r.clicked() {
                        crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle.id));
                    }
                });
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
    let mut checkbox = move |ui: &mut egui::Ui, s: &str| {
        // ui.horizontal(|ui| {
        ui.checkbox(&mut false, s);
        // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        //     ui.label(format!("{}", r()));
        // });
        // });
    };

    ui.menu_button("Puzzle engine", |ui| {
        checkbox(ui, "Euclidean");
        checkbox(ui, "Conformal");
        checkbox(ui, "Laminated");
    });
    ui.menu_button("Rank", |ui| {
        checkbox(ui, "Rank 3");
        checkbox(ui, "Rank 4");
        checkbox(ui, "Rank 5");
        checkbox(ui, "Rank 6");
        checkbox(ui, "Rank 7");
    });
    ui.menu_button("Dimension", |ui| {
        checkbox(ui, "5D");
        checkbox(ui, "6D");
        checkbox(ui, "7D");
    });
    ui.horizontal(|ui| {
        checkbox(ui, "2D");
        checkbox(ui, "3D");
        checkbox(ui, "4D");
    });

    ui.separator();

    ui.strong("Shapes");

    ui.menu_button("Platonic solids (3D)", |ui| {
        checkbox(ui, "All");
        checkbox(ui, "Cube");
        checkbox(ui, "Tetrahedon");
        checkbox(ui, "Dodecahedon");
        checkbox(ui, "Octahedon");
        checkbox(ui, "Icosahedon");
    });
    ui.menu_button("Platonic solids (4D)", |ui| {
        checkbox(ui, "All");
        checkbox(ui, "Hypercube");
        checkbox(ui, "4-Simplex");
        checkbox(ui, "16-Cell");
        checkbox(ui, "24-Cell");
        checkbox(ui, "120-Cell");
        checkbox(ui, "600-Cell");
    });
    ui.menu_button("Platonic solids (5D+)", |ui| {
        checkbox(ui, "All");
        checkbox(ui, "N-Simplex");
        checkbox(ui, "N-Cube");
        checkbox(ui, "N-Orthoplex");
    });
    ui.menu_button("Prisms", |ui| {
        checkbox(ui, "Simple prisms");
        checkbox(ui, "Star prism");
        checkbox(ui, "Antiprism");
        checkbox(ui, "Duoprism");
        checkbox(ui, "Stellation");
    });
    ui.menu_button("Compounds", |ui| {
        checkbox(ui, "All");
        checkbox(ui, "5 tetrahedra");
    });
    ui.menu_button("Nonconvex shapes", |ui| {
        checkbox(ui, "All");
        checkbox(ui, "Hemioctahedron");
    });

    ui.separator();

    ui.strong("Twists");

    ui.menu_button("Cut depths", |ui| {
        checkbox(ui, "Shallow-cut");
        checkbox(ui, "Cut to adjacent");
        checkbox(ui, "Half-cut");
        ui.menu_button("Deep-cut", |ui| {
            checkbox(ui, "All");
            checkbox(ui, "Deeper than adjacent");
            checkbox(ui, "Deeper than origin");
        });
    });

    ui.menu_button("Turning elements", |ui| {
        checkbox(ui, "Facet-turning");
        checkbox(ui, "Ridge-turning");
        checkbox(ui, "Edge-turning");
        checkbox(ui, "Vertex-turning");
        checkbox(ui, "Other");
    });

    ui.menu_button("Axis systems", |ui| {
        checkbox(ui, "Cubic");
        checkbox(ui, "Octahedral");
        checkbox(ui, "Tetrahedral");
        checkbox(ui, "Triangular");
        checkbox(ui, "Rhombicuboctahedral");
    });

    ui.menu_button("Properties", |ui| {
        checkbox(ui, "Doctrinaire");
        checkbox(ui, "Jumbling");
        ui.separator();
        checkbox(ui, "Shapeshifting");
        ui.separator();
        checkbox(ui, "Reduced"); // ?
        ui.separator();
        checkbox(ui, "Twisting");
        checkbox(ui, "Sliding");
    });

    ui.separator();

    ui.strong("Solving");

    ui.menu_button("Difficulties", |ui| {
        checkbox(ui, "Trivial");
        checkbox(ui, "Easy");
        checkbox(ui, "3x3x3-like");
        checkbox(ui, "Hard");
        checkbox(ui, "Evil");
        checkbox(ui, "Beyond Luna");
    });

    checkbox(ui, "Solved");

    ui.separator();

    ui.strong("Attribution");

    ui.menu_button("Authors", |ui| {
        for name in ["Milo Jacquet", "Andrew Farkas", "Luna Harran"]
            .iter()
            .sorted()
        {
            checkbox(ui, name);
        }
    });
    ui.menu_button("Inventors", |ui| {
        for name in ["Ernő Rubik", "Oskar van Deventer"].iter().sorted() {
            checkbox(ui, name);
        }
    });

    ui.separator();

    ui.strong("Other");

    ui.menu_button("Families", |ui| {
        ui.menu_button("Cuboid", |ui| {
            checkbox(ui, "All");
            checkbox(ui, "180-only");
            checkbox(ui, "Brick");
            checkbox(ui, "Domino");
            checkbox(ui, "Floppy");
            checkbox(ui, "Pancake");
            checkbox(ui, "Tower");
        });
        ui.menu_button("Gap", |ui| {
            checkbox(ui, "All");
            checkbox(ui, "Sliding gap");
            checkbox(ui, "Rotating gap");
        });
        ui.menu_button("Multicore", |ui| {
            checkbox(ui, "All");
            checkbox(ui, "Siamese");
        });
        checkbox(ui, "Bermuda");
        checkbox(ui, "Bubbloid");
        checkbox(ui, "Fenzy");
        checkbox(ui, "Loopover");
        checkbox(ui, "Mixup");
        checkbox(ui, "Square-1");
        checkbox(ui, "Weirdling");
    });

    ui.menu_button("Variants", |ui| {
        checkbox(ui, "Stickermod");
        checkbox(ui, "Shapemod");
        checkbox(ui, "Super");
        checkbox(ui, "Real");
        checkbox(ui, "Complex");
        checkbox(ui, "Weirdling");
        checkbox(ui, "Bump");
        checkbox(ui, "Master");
        checkbox(ui, "Bandaging");
        checkbox(ui, "Unbandaging");
    });

    ui.menu_button("Color systems", |ui| {
        for color_system in crate::LIBRARY.with(|lib| lib.color_systems()) {
            checkbox(ui, color_system.display_name());
        }
    });

    checkbox(ui, "Generated");
    checkbox(ui, "Canonical");
    checkbox(ui, "Meme");
    checkbox(ui, "WCA");
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

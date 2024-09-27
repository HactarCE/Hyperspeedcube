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
    // crate::LIBRARY.with(|lib| {
    //     for puzzle in lib.puzzles() {
    //         if ui
    //             .button(format!(
    //                 "Load {} v{}.{}.{}",
    //                 puzzle.display_name(),
    //                 puzzle.version[0],
    //                 puzzle.version[1],
    //                 puzzle.version[2],
    //             ))
    //             .clicked()
    //         {
    //             app.load_puzzle(lib, &puzzle.id);
    //         }
    //     }
    // });

    use rand::prelude::*;
    lazy_static! {
        static ref R: Vec<usize> =
            std::iter::from_fn(|| Some(rand::thread_rng().gen_range(0..5840)))
                .take(100)
                .collect();
    }
    let mut rands = R.to_vec();
    let mut r = move || rands.pop().unwrap();

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

        let candidates = match subtab {
            SubTab::Puzzles => crate::LIBRARY
                .with(|lib| lib.puzzles())
                .into_iter()
                .map(|p| p.display_name().to_owned())
                .collect_vec(),
            SubTab::Generators => vec![
                "NxNxN".to_owned(),
                "NxNxNxN".to_owned(),
                "N-Layer Kilominx".to_owned(),
                "N-Layer Megaminx".to_owned(),
                "N-Layer Pentultimate".to_owned(),
                "N-Layer Face-Turning Octahedron".to_owned(),
                "N-Layer Corner-Turning Octahedron".to_owned(),
                "N-Layer Skewb".to_owned(),
            ],
        };

        let plural_noun = match subtab {
            SubTab::Puzzles => "puzzles",
            SubTab::Generators => "puzzle generators",
        };

        filtered_list(ui, &search_query, &candidates, plural_noun);
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

fn filtered_list<'a>(ui: &mut egui::Ui, query: &str, candidates: &[String], plural_noun: &str) {
    let filtered_candidates: Vec<egui::WidgetText> = if query.is_empty() {
        candidates
            .iter()
            .map(|candidate| candidate.into())
            .collect()
    } else {
        candidates
            .iter()
            .sorted()
            .flat_map(|candidate| fuzzy_match(ui, query, candidate))
            .sorted_by_key(|(_text, score)| *score)
            .map(|(text, _score)| text)
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
                for s in filtered_candidates {
                    ui.selectable_label(false, s);
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
        for (id, name) in crate::LIBRARY.with(|lib| lib.color_systems()) {
            checkbox(ui, name.as_ref().unwrap_or(&id));
        }
    });

    checkbox(ui, "Canonical");
    checkbox(ui, "Meme");
    checkbox(ui, "WCA");
}

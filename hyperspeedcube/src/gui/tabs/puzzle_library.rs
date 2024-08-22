use crate::{app::App, L};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if crate::PATHS.is_some() {
        if ui.button(L.library.reload_all_files).clicked()
            || ui.input(|input| input.key_pressed(egui::Key::F5))
        {
            crate::reload_user_puzzles();
        }
        ui.separator();
    }

    crate::LIBRARY.with(|lib| {
        for puzzle in lib.puzzles() {
            if ui
                .button(format!("Load {}", puzzle.display_name()))
                .clicked()
            {
                app.load_puzzle(lib, &puzzle.id);
            }
        }
    });

    // use rand::prelude::*;
    // lazy_static! {
    //     static ref R: Vec<usize> =
    //         std::iter::from_fn(|| Some(rand::thread_rng().gen_range(0..5840)))
    //             .take(100)
    //             .collect();
    // }
    // let mut rands = R.to_vec();
    // let mut r = move || rands.pop().unwrap();
    // let mut checkbox = move |ui: &mut egui::Ui, s: &str| {
    //     ui.style_mut().wrap = Some(false);
    //     ui.horizontal(|ui| {
    //         ui.checkbox(&mut false, s);
    //         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
    //             ui.label(format!("{}", r()));
    //         });
    //     });
    // };
    // ui.text_edit_singleline(&mut "megaminx".to_string());
    // // ui.menu_button("Tags", |ui| {
    // // ui.set_width(250.0);

    // ui.horizontal_wrapped(|ui| {
    //     ui.menu_button("Construction", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Solid");
    //         checkbox(ui, "Tiling");
    //         checkbox(ui, "Soup");
    //     });
    //     ui.menu_button("Dimensions", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "2D");
    //         checkbox(ui, "3D");
    //         checkbox(ui, "4D");
    //         checkbox(ui, "5D");
    //         checkbox(ui, "6D");
    //         checkbox(ui, "7D");
    //     });
    //     ui.menu_button("Ranks", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Rank 3");
    //         checkbox(ui, "Rank 4");
    //         checkbox(ui, "Rank 5");
    //         checkbox(ui, "Rank 6");
    //         checkbox(ui, "Rank 7");
    //     });
    //     ui.menu_button("Shapes", |ui| {
    //         ui.set_width(250.0);

    //         ui.menu_button("Platonic solids", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "Cubes");
    //             checkbox(ui, "Tetrahedra");
    //             checkbox(ui, "Dodecahedra");
    //             checkbox(ui, "Octahedra");
    //             checkbox(ui, "Icosahedra");
    //         });
    //         ui.menu_button("Prisms", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             ui.menu_button("Star prisms", |ui| {
    //                 ui.set_width(250.0);

    //                 checkbox(ui, "All");
    //             });
    //         });
    //         checkbox(ui, "Duoprism");
    //         ui.menu_button("Stellation", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //         });
    //         ui.menu_button("Compound", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "5 tetrahedra");
    //         });
    //         ui.menu_button("Nonconvex", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "Hemioctahedron");
    //         });
    //     });
    //     ui.menu_button("Turns", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Facet-turning");
    //         checkbox(ui, "Ridge-turning");
    //         checkbox(ui, "Edge-turning");
    //         checkbox(ui, "Vertex-turning");
    //         checkbox(ui, "Other");
    //     });
    //     ui.menu_button("Axes", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Cubic");
    //         checkbox(ui, "Octahedral");
    //         checkbox(ui, "Tetrahedral");
    //         checkbox(ui, "Triangular");
    //         checkbox(ui, "Rhombicuboctahedral");
    //     });
    //     ui.menu_button("Cut depths", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Shallow-cut");
    //         checkbox(ui, "Cut to adjacent");
    //         checkbox(ui, "Half-cut");
    //         ui.menu_button("Deep-cut", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "Deeper than adjacent");
    //             checkbox(ui, "Deeper than origin");
    //         });
    //     });
    //     ui.menu_button("Turning properties", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Doctrinaire");
    //         checkbox(ui, "Bandaged");
    //         checkbox(ui, "Unbandaged");
    //         checkbox(ui, "Shapeshifting");
    //         checkbox(ui, "Jumbling");
    //         checkbox(ui, "Reduced");
    //         checkbox(ui, "Twisting");
    //         checkbox(ui, "Sliding");
    //     });
    //     ui.menu_button("Variants", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Stickermod");
    //         checkbox(ui, "Shapemod");
    //         checkbox(ui, "Super");
    //         checkbox(ui, "Real");
    //         checkbox(ui, "Complex");
    //         checkbox(ui, "Weirdling");
    //         checkbox(ui, "Bump");
    //         checkbox(ui, "Master");
    //     });
    //     ui.menu_button("Families", |ui| {
    //         ui.set_width(250.0);

    //         ui.menu_button("Cuboid", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "180-only");
    //             checkbox(ui, "Brick");
    //             checkbox(ui, "Domino");
    //             checkbox(ui, "Floppy");
    //             checkbox(ui, "Pancake");
    //             checkbox(ui, "Tower");
    //         });
    //         checkbox(ui, "Bermuda");
    //         checkbox(ui, "Bubbloid");
    //         checkbox(ui, "Fenzy");
    //         ui.menu_button("Gap", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "Sliding gap");
    //             checkbox(ui, "Rotating gap");
    //         });
    //         checkbox(ui, "Loopover");
    //         checkbox(ui, "Mixup");
    //         ui.menu_button("Multicore", |ui| {
    //             ui.set_width(250.0);

    //             checkbox(ui, "All");
    //             checkbox(ui, "Siamese");
    //         });
    //         checkbox(ui, "Square-1");
    //         checkbox(ui, "Weirdling");
    //     });
    //     ui.menu_button("Difficulty", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Trivial");
    //         checkbox(ui, "Easy");
    //         checkbox(ui, "3x3x3-like");
    //         checkbox(ui, "Hard");
    //         checkbox(ui, "Evil");
    //         checkbox(ui, "Beyond Luna");
    //     });
    //     ui.menu_button("Source", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Physical");
    //         checkbox(ui, "Gelatinbrain");
    //         checkbox(ui, "MC4D");
    //         checkbox(ui, "pCubes");
    //     });
    //     ui.menu_button("Other tags", |ui| {
    //         ui.set_width(250.0);

    //         checkbox(ui, "Canonical");
    //         checkbox(ui, "Memes");
    //         checkbox(ui, "WCA");
    //     });
    //     checkbox(ui, "Solved");
    // });
    // ui.separator();
    // egui_extras::TableBuilder::new(ui)
    //     .column(egui_extras::Column::initial(30.0))
    //     .column(egui_extras::Column::initial(100.0))
    //     .column(egui_extras::Column::initial(1000.0))
    //     .header(15.0, |mut ui| {
    //         ui.col(|ui| {
    //             ui.label("#");
    //         });
    //         ui.col(|ui| {
    //             ui.label("Name");
    //         });
    //         ui.col(|ui| {
    //             ui.label("Tags");
    //         });
    //     })
    //     .body(|mut ui| {
    //         ui.row(15.0, |mut ui| {
    //             ui.col(|ui| {
    //                 ui.label("1");
    //             });
    //             ui.col(|ui| {
    //                 ui.label("Rubik's Cube");
    //             });
    //             ui.col(|ui| {
    //                 ui.label(
    //                     "#solid #3d #rank-3 #cube #facet-turning \
    //                              #cubic-cuts #shallow-cut #doctrinaire \
    //                              #3x3x3-like #physical #canonical #wca",
    //                 );
    //             });
    //         });
    //         ui.row(15.0, |mut ui| {
    //             ui.col(|ui| {
    //                 ui.label("2");
    //             });
    //             ui.col(|ui| {
    //                 ui.label("Square-1");
    //             });
    //             ui.col(|ui| {
    //                 ui.label(
    //                     "#solid #3d #rank-3 #cube #facet-turning \
    //                              #prismic-cuts #half-cut #shapeshifting \
    //                              #bandaged #physical #wca",
    //                 );
    //             });
    //         });
    //     });
}

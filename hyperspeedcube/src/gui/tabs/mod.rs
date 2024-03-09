use std::path::PathBuf;
use std::sync::Arc;

use hypermath::Isometry;
use hyperpuzzle::LuaLogLine;
use itertools::Itertools;
use parking_lot::Mutex;

// mod debug;
// mod puzzle_setup;
mod puzzle_view;

// pub use debug::PolytopeTree;
// pub use puzzle_setup::PuzzleSetup;
use crate::gfx::RenderEngine;
pub use puzzle_view::PuzzleView;

use super::App;

lazy_static! {
    #[rustfmt::skip]
    static ref LUA_PATH: PathBuf = if crate::IS_OFFICIAL_BUILD {
        std::env::current_exe().unwrap()
            .canonicalize().unwrap()
            .parent().unwrap()
            .to_owned()
            .join("lua")
    } else {
        std::env::current_exe().unwrap()
            .canonicalize().unwrap()
            .parent().unwrap()
            .parent().unwrap()
            .parent().unwrap()
            .to_owned()
            .join("lua")
    };
}

#[derive(Debug)]
pub enum Tab {
    AppearanceSettings,
    InteractionSettings,
    ViewSettings,

    PuzzleView(Arc<Mutex<PuzzleView>>),
    // PuzzleSetup(PuzzleSetup),
    // PolytopeTree(PolytopeTree),
    PuzzleLibraryDemo,
    PuzzleLibrary,
    PuzzleInfo,
    LuaLogs,
}
impl Tab {
    pub fn title(&self) -> egui::WidgetText {
        match self {
            Tab::AppearanceSettings => "Appearance".into(),
            Tab::InteractionSettings => "Interaction".into(),
            Tab::ViewSettings => "View".into(),

            Tab::PuzzleView(p) => match &p.lock().puzzle {
                Some(p) => p.name.clone().into(),
                None => "No Puzzle".into(),
            },
            // Tab::PuzzleSetup(_) => "Puzzle Setup".into(),
            // Tab::PolytopeTree(_) => "Polytope Tree".into(),
            Tab::PuzzleLibraryDemo => "Puzzle Library".into(),
            Tab::PuzzleLibrary { .. } => "Puzzle Library".into(),
            Tab::PuzzleInfo { .. } => "Puzzle Info".into(),
            Tab::LuaLogs => "Lua Logs".into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        use crate::gui::components::prefs;

        match self {
            Tab::AppearanceSettings => {
                egui::CollapsingHeader::new("Colors")
                    .default_open(true)
                    .show(ui, |ui| prefs::build_colors_section(ui, app));
                egui::CollapsingHeader::new("Outlines")
                    .default_open(true)
                    .show(ui, |ui| prefs::build_outlines_section(ui, app));
                egui::CollapsingHeader::new("Opacity")
                    .default_open(true)
                    .show(ui, |ui| prefs::build_opacity_section(ui, app));
                egui::CollapsingHeader::new("Performance")
                    .default_open(true)
                    .show(ui, |ui| prefs::build_graphics_section(ui, app));
            }
            Tab::InteractionSettings => {
                prefs::build_interaction_section(ui, app);
            }
            Tab::ViewSettings => {
                if let Some(puzzle_view) = app.active_puzzle_view.upgrade() {
                    let mut puzzle_view = puzzle_view.lock();

                    ui.horizontal(|ui| {
                        let options = [RenderEngine::SinglePass, RenderEngine::MultiPass];
                        let mut i = match puzzle_view.render_engine {
                            RenderEngine::SinglePass => 0,
                            RenderEngine::MultiPass => 1,
                        };
                        let get_fn = |i: usize| options[i].to_string();
                        egui::ComboBox::new(unique_id!(), "Render engine")
                            .show_index(ui, &mut i, 2, get_fn);
                        puzzle_view.render_engine = options[i];
                    });

                    ui.separator();

                    if ui.button("Reset camera").clicked() {
                        puzzle_view.draw_params.rot = Isometry::ident();
                    }

                    ui.separator();
                }

                prefs::build_view_section(ui, app);
            }

            Tab::PuzzleView(puzzle_view) => {
                let mut puzzle_view_mutex_guard = puzzle_view.lock();
                let r = puzzle_view_mutex_guard.ui(ui, &app.prefs);
                if r.gained_focus() {
                    app.active_puzzle_view = Arc::downgrade(puzzle_view);
                }
            }

            Tab::PuzzleLibraryDemo => {
                use rand::prelude::*;
                lazy_static! {
                    static ref R: Vec<usize> =
                        std::iter::from_fn(|| Some(rand::thread_rng().gen_range(0..5840)))
                            .take(100)
                            .collect();
                }
                let mut rands = R.to_vec();
                let mut r = move || rands.pop().unwrap();
                let mut checkbox = move |ui: &mut egui::Ui, s: &str| {
                    ui.style_mut().wrap = Some(false);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut false, s);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{}", r()));
                        });
                    });
                };
                ui.text_edit_singleline(&mut "megaminx".to_string());
                // ui.menu_button("Tags", |ui| {
                // ui.set_width(250.0);

                ui.horizontal_wrapped(|ui| {
                    ui.menu_button("Construction", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Solid");
                        checkbox(ui, "Tiling");
                        checkbox(ui, "Soup");
                    });
                    ui.menu_button("Dimensions", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "2D");
                        checkbox(ui, "3D");
                        checkbox(ui, "4D");
                        checkbox(ui, "5D");
                        checkbox(ui, "6D");
                        checkbox(ui, "7D");
                    });
                    ui.menu_button("Ranks", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Rank 3");
                        checkbox(ui, "Rank 4");
                        checkbox(ui, "Rank 5");
                        checkbox(ui, "Rank 6");
                        checkbox(ui, "Rank 7");
                    });
                    ui.menu_button("Shapes", |ui| {
                        ui.set_width(250.0);

                        ui.menu_button("Platonic solids", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "Cubes");
                            checkbox(ui, "Tetrahedra");
                            checkbox(ui, "Dodecahedra");
                            checkbox(ui, "Octahedra");
                            checkbox(ui, "Icosahedra");
                        });
                        ui.menu_button("Prisms", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            ui.menu_button("Star prisms", |ui| {
                                ui.set_width(250.0);

                                checkbox(ui, "All");
                            });
                        });
                        checkbox(ui, "Duoprism");
                        ui.menu_button("Stellation", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                        });
                        ui.menu_button("Compound", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "5 tetrahedra");
                        });
                        ui.menu_button("Nonconvex", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "Hemioctahedron");
                        });
                    });
                    ui.menu_button("Turns", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Facet-turning");
                        checkbox(ui, "Ridge-turning");
                        checkbox(ui, "Edge-turning");
                        checkbox(ui, "Vertex-turning");
                        checkbox(ui, "Other");
                    });
                    ui.menu_button("Axes", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Cubic");
                        checkbox(ui, "Octahedral");
                        checkbox(ui, "Tetrahedral");
                        checkbox(ui, "Triangular");
                        checkbox(ui, "Rhombicuboctahedral");
                    });
                    ui.menu_button("Cut depths", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Shallow-cut");
                        checkbox(ui, "Cut to adjacent");
                        checkbox(ui, "Half-cut");
                        ui.menu_button("Deep-cut", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "Deeper than adjacent");
                            checkbox(ui, "Deeper than origin");
                        });
                    });
                    ui.menu_button("Turning properties", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Doctrinaire");
                        checkbox(ui, "Bandaged");
                        checkbox(ui, "Unbandaged");
                        checkbox(ui, "Shapeshifting");
                        checkbox(ui, "Jumbling");
                        checkbox(ui, "Reduced");
                        checkbox(ui, "Twisting");
                        checkbox(ui, "Sliding");
                    });
                    ui.menu_button("Variants", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Stickermod");
                        checkbox(ui, "Shapemod");
                        checkbox(ui, "Super");
                        checkbox(ui, "Real");
                        checkbox(ui, "Complex");
                        checkbox(ui, "Weirdling");
                        checkbox(ui, "Bump");
                        checkbox(ui, "Master");
                    });
                    ui.menu_button("Families", |ui| {
                        ui.set_width(250.0);

                        ui.menu_button("Cuboid", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "180-only");
                            checkbox(ui, "Brick");
                            checkbox(ui, "Domino");
                            checkbox(ui, "Floppy");
                            checkbox(ui, "Pancake");
                            checkbox(ui, "Tower");
                        });
                        checkbox(ui, "Bermuda");
                        checkbox(ui, "Bubbloid");
                        checkbox(ui, "Fenzy");
                        ui.menu_button("Gap", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "Sliding gap");
                            checkbox(ui, "Rotating gap");
                        });
                        checkbox(ui, "Loopover");
                        checkbox(ui, "Mixup");
                        ui.menu_button("Multicore", |ui| {
                            ui.set_width(250.0);

                            checkbox(ui, "All");
                            checkbox(ui, "Siamese");
                        });
                        checkbox(ui, "Square-1");
                        checkbox(ui, "Weirdling");
                    });
                    ui.menu_button("Difficulty", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Trivial");
                        checkbox(ui, "Easy");
                        checkbox(ui, "3x3x3-like");
                        checkbox(ui, "Hard");
                        checkbox(ui, "Evil");
                        checkbox(ui, "Beyond Luna");
                    });
                    ui.menu_button("Source", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Physical");
                        checkbox(ui, "Gelatinbrain");
                        checkbox(ui, "MC4D");
                        checkbox(ui, "pCubes");
                    });
                    ui.menu_button("Other tags", |ui| {
                        ui.set_width(250.0);

                        checkbox(ui, "Canonical");
                        checkbox(ui, "Memes");
                        checkbox(ui, "WCA");
                    });
                    checkbox(ui, "Solved");
                });
                ui.separator();
                egui_extras::TableBuilder::new(ui)
                    .column(egui_extras::Column::initial(30.0))
                    .column(egui_extras::Column::initial(100.0))
                    .column(egui_extras::Column::initial(1000.0))
                    .header(15.0, |mut ui| {
                        ui.col(|ui| {
                            ui.label("#");
                        });
                        ui.col(|ui| {
                            ui.label("Name");
                        });
                        ui.col(|ui| {
                            ui.label("Tags");
                        });
                    })
                    .body(|mut ui| {
                        ui.row(15.0, |mut ui| {
                            ui.col(|ui| {
                                ui.label("1");
                            });
                            ui.col(|ui| {
                                ui.label("Rubik's Cube");
                            });
                            ui.col(|ui| {
                                ui.label(
                                    "#solid #3d #rank-3 #cube #facet-turning \
                                     #cubic-cuts #shallow-cut #doctrinaire \
                                     #3x3x3-like #physical #canonical #wca",
                                );
                            });
                        });
                        ui.row(15.0, |mut ui| {
                            ui.col(|ui| {
                                ui.label("2");
                            });
                            ui.col(|ui| {
                                ui.label("Square-1");
                            });
                            ui.col(|ui| {
                                ui.label(
                                    "#solid #3d #rank-3 #cube #facet-turning \
                                     #prismic-cuts #half-cut #shapeshifting \
                                     #bandaged #physical #wca",
                                );
                            });
                        });
                    });
            }
            Tab::PuzzleLibrary => {
                let id = egui::Id::new("hyperspeedcube/files");
                let needs_reload = ui.button("Reload all files").clicked()
                    || ui.data(|data| data.get_temp::<()>(id).is_none())
                    || ui.input(|input| input.key_pressed(egui::Key::F5));
                if needs_reload {
                    ui.data_mut(|data| data.insert_temp(id, ()));
                    log::info!("Loading Lua files from path {}", LUA_PATH.to_string_lossy());
                    crate::LIBRARY.with(|lib| lib.load_directory(&LUA_PATH));
                }
                ui.separator();
                crate::LIBRARY.with(|lib| {
                    for puzzle in lib.puzzles().values().sorted_by_key(|p| &p.name) {
                        if ui.button(format!("Load {}", puzzle.name)).clicked() {
                            app.load_puzzle(lib, &puzzle.id);
                        }
                    }
                });
                ui.separator();
            }
            Tab::PuzzleInfo => {
                if let Some(puzzle_view) = app.active_puzzle_view.upgrade() {
                    if let Some(puzzle) = &puzzle_view.lock().puzzle {
                        ui.label(format!("ID: {}", puzzle.id));
                        ui.label(format!("Name: {}", puzzle.name));
                        ui.label(format!("Piece count: {}", puzzle.pieces.len()));
                        ui.label(format!("Sticker count: {}", puzzle.stickers.len()));
                        ui.label(format!("Color count: {}", puzzle.colors.len()));

                        ui.add_space(10.0);
                        ui.heading("Piece types");
                        for piece_type in puzzle.piece_types.iter_values() {
                            ui.label(format!("• {}", &piece_type.name));
                        }

                        ui.add_space(10.0);
                        ui.heading("Colors");
                        for color in puzzle.colors.iter_values() {
                            let name = &color.name;
                            let default_color_string = match &color.default_color {
                                Some(default) => format!(" (default={default})"),
                                None => String::new(),
                            };
                            ui.label(format!("• {name}{default_color_string}"));
                        }
                    } else {
                        ui.label("No active puzzle");
                    }
                } else {
                    ui.label("No active puzzle");
                }

                #[cfg(debug_assertions)]
                {
                    let mut debug_info =
                        std::mem::take(&mut *crate::debug::FRAME_DEBUG_INFO.lock().unwrap());
                    ui.add(egui::TextEdit::multiline(&mut debug_info).code_editor());
                }
            }
            Tab::LuaLogs => {
                let mut log_lines = crate::LIBRARY_LOG_LINES.lock();
                if ui.button("Clear logs").clicked() {
                    log_lines.clear();
                }

                let filter_string_id = unique_id!();
                let mut filter_string: String =
                    ui.data_mut(|data| data.get_temp(filter_string_id).clone().unwrap_or_default());
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut filter_string);
                });
                ui.data_mut(|data| data.insert_temp(filter_string_id, filter_string.clone()));

                egui::ScrollArea::new([true; 2])
                    .auto_shrink(false)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &**log_lines {
                            if line.matches_filter_string(&filter_string) {
                                colored_log_line(ui, line)
                            }
                        }
                    });
            }
        }
    }
}

fn colored_log_line(ui: &mut egui::Ui, line: &LuaLogLine) {
    let color = match line.level.as_deref() {
        Some("info") => egui::Color32::LIGHT_BLUE,
        Some("warn" | "warning") => egui::Color32::YELLOW,
        Some("error") => egui::Color32::LIGHT_RED,
        _ => egui::Color32::WHITE,
    };
    let s = match &line.file {
        Some(file) => format!("[{}] {}", file, line.msg),
        None => format!("{}", line.msg),
    };
    ui.label(
        egui::RichText::new(s)
            .color(color)
            .text_style(egui::TextStyle::Monospace),
    );
}

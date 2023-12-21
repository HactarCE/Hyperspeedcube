use eyre::Result;
use itertools::Itertools;
use ndpuzzle::{collections::*, geometry::*, math::*, puzzle::*, vector};

use super::App;

#[derive(Debug)]
pub struct PuzzleSetup {
    ndim: u8,
    schlafli: String,
    seeds: Vec<Vector>,
    do_twist_cuts: bool,
    cut_depth: Float,
    ignore_errors: bool,

    construction_steps: Vec<ConstructStep>,
    num_steps_executed: usize,
    error_string: Option<String>,

    first_frame: bool,
}
impl Default for PuzzleSetup {
    fn default() -> Self {
        let mut ret = Self {
            ndim: 4,
            schlafli: "3,4,2".to_string(),
            seeds: vec![vector![0.0, 0.0, 1.0, 0.0], vector![0.0, 0.0, 0.0, 1.0]],
            //  schlafli: "5,2".to_string(),
            // seeds: vec![vector![0.0, 1.0, 0.0], vector![0.0, 0.0, 1.0]],
            // schlafli: "3,4".to_string(),
            // seeds: vec![vector![0.0, 0.0, 1.0]],
            do_twist_cuts: true,
            cut_depth: 0.0,
            ignore_errors: false,

            construction_steps: vec![],
            num_steps_executed: 0,
            error_string: None,

            first_frame: true,
        };

        ret.recompute_steps();

        ret
    }
}
impl PuzzleSetup {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
        use std::f64::consts::SQRT_2;

        egui::CollapsingHeader::new("Example shapes")
            .default_open(true)
            .show(ui, |ui| {
                let examples = [
                    (
                        ("cube", "Rubik's cube"),
                        (3, "4,3", vec![vector![0.0, 0.0, 1.0]], 1.0 / 3.0),
                    ),
                    (
                        ("dodecahedron {5,3}", "megaminx"),
                        (3, "5,3", vec![vector![0.0, 0.0, 1.0]], 0.65),
                    ),
                    (
                        ("tetrahedron {3,3}", "Jing's pyraminx"),
                        (3, "3,3", vec![vector![0.0, 0.0, 1.0]], 0.0),
                    ),
                    (
                        ("octahedron {3,4}", "FTO"),
                        (3, "3,4", vec![vector![0.0, 0.0, 1.0]], 0.5),
                    ),
                    (
                        ("icosahedron {3,5}", "deep-cut FTI"),
                        (3, "3,5", vec![vector![0.0, 0.0, 1.0]], 0.71),
                    ),
                    (
                        ("rhombicuboctahedron", "(with cuts)"),
                        (
                            3,
                            "4,3",
                            vec![
                                vector![0.0, 0.0, 1.0 + SQRT_2],
                                vector![0.0, 1.0 + SQRT_2 / 2.0, 1.0 + SQRT_2 / 2.0],
                                vector![1.0 + SQRT_2 / 3.0, 1.0 + SQRT_2 / 3.0, 1.0 + SQRT_2 / 3.0],
                            ],
                            1.0 / (1.0 + SQRT_2),
                        ),
                    ),
                    (
                        ("hypercube", "3^4"),
                        (4, "4,3,3", vec![vector![0.0, 0.0, 0.0, 1.0]], 1.0 / 3.0),
                    ),
                    (
                        ("4-simplex", "(with cuts)"),
                        (4, "3,3,3", vec![vector![0.0, 0.0, 0.0, 1.0]], 0.5),
                    ),
                    (
                        ("octahedral prism", "(with octahedral cuts)"),
                        (
                            4,
                            "3,4,2",
                            vec![vector![0.0, 0.0, 1.0, 0.0], vector![0.0, 0.0, 0.0, 1.0]],
                            0.5,
                        ),
                    ),
                ];
                for ((name1, name2), (ndim, schlafli, seeds, cut_depth)) in examples {
                    ui.horizontal(|ui| {
                        let load_without_cuts = ui.button(name1).clicked();
                        let load_with_cuts = ui.button(name2).clicked() || self.first_frame;
                        if load_without_cuts || load_with_cuts {
                            self.first_frame = false;
                            self.ndim = ndim;
                            self.schlafli = schlafli.to_string();
                            self.seeds = seeds;
                            self.do_twist_cuts = load_with_cuts;
                            self.cut_depth = cut_depth;
                            self.recompute_steps();
                            let shape = self.try_generate_mesh();
                            self.set_shape(app, shape);
                        }
                    });
                }
                if let Some(s) = &self.error_string {
                    ui.colored_label(egui::Color32::RED, s);
                }
            });
        ui.collapsing("Custom shapes", |ui| self.ui_custom(ui, app));
    }

    fn ui_custom(&mut self, ui: &mut egui::Ui, app: &mut App) {
        let mut changed = false;

        ui.horizontal(|ui| {
            let r = ui.add(
                egui::DragValue::new(&mut self.ndim)
                    .clamp_range(2..=8)
                    .speed(0.05),
            );
            changed |= r.changed();
            ui.label("Dimensions");
        });

        ui.strong("Schlafli symbol");
        let r = ui.text_edit_singleline(&mut self.schlafli);
        changed |= r.changed();
        let ndim = self.ndim;

        ui.separator();

        ui.strong("Seeds");
        let seeds_len = self.seeds.len();
        self.seeds.retain_mut(|v| {
            let mut keep = true;
            ui.horizontal(|ui| {
                ui.add_enabled_ui(seeds_len > 1, |ui| {
                    keep &= !ui.button("-").clicked();
                });
                changed |= vector_edit(ui, v, ndim);
            });
            changed |= !keep;
            keep
        });
        if ui.button("+").clicked() {
            changed |= true;
            self.seeds.push(vector![0.0, 0.0, 1.0]);
        }

        ui.separator();

        let r = ui.checkbox(&mut self.do_twist_cuts, "Twist cuts");
        changed |= r.changed();
        ui.add_enabled_ui(self.do_twist_cuts, |ui| {
            ui.horizontal(|ui| {
                let r = ui.add(
                    egui::DragValue::new(&mut self.cut_depth)
                        .clamp_range(0.0..=1.0)
                        .speed(0.05),
                );
                changed |= r.changed();
                ui.label("Cut depth");
            })
        });

        if changed {
            self.recompute_steps();
        }

        ui.separator();

        let mut new_shape = None;

        let active_view = app.active_puzzle_view.upgrade();
        ui.add_enabled_ui(active_view.is_some(), |ui| {
            ui.checkbox(&mut self.ignore_errors, "Ignore errors");

            if ui.button("Generate!").clicked() {
                new_shape = Some(self.try_generate_mesh());
            }
            if let Some(s) = &self.error_string {
                ui.colored_label(egui::Color32::RED, s);
            }
            ui.collapsing("Construction steps", |ui| {
                let steps = self.construction_steps.clone();
                for (i, step) in steps.into_iter().enumerate() {
                    let r = ui.selectable_label(i < self.num_steps_executed, format!("{step:?}"));
                    if r.clicked() {
                        new_shape = Some(self.try_generate_partial_mesh(i + 1));
                    }
                }
            });
        });

        if let Some(result) = new_shape {
            self.set_shape(app, result);
        }

        ui.separator();
    }

    fn set_shape(&mut self, app: &mut App, result: Result<ShapeArena>) {
        self.error_string = None;
        match result {
            Ok(arena) => {
                let mut mesh = None;
                match Mesh::from_arena(&arena, self.ignore_errors) {
                    Ok(m) => mesh = Some(m),
                    Err(e) => self.error_string = Some(e.chain().join("\n")),
                }

                app.active_puzzle_view.upgrade().unwrap().lock().set_mesh(
                    &app.gfx,
                    arena,
                    mesh.as_ref(),
                );
            }
            Err(e) => self.error_string = Some(e.chain().join("\n")),
        }
    }

    fn recompute_steps(&mut self) {
        let result = self.try_recompute_steps();
        self.error_string = result.err().map(|e| e.to_string());
    }
    fn try_recompute_steps(&mut self) -> Result<()> {
        self.construction_steps.clear();

        let s = SchlafliSymbol::from_string(&self.schlafli);
        let m = Matrix::from_cols(s.mirrors().iter().rev().map(|v| &v.0))
            .inverse()
            .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
            .transpose();
        let g = s.group()?;

        let mut f = 0;
        let mut seen = VectorHashMap::new();
        for elem in g.elements() {
            for seed in &self.seeds {
                // Ignore zero vectors
                if *seed == vector![] {
                    continue;
                }

                let v = g[elem].transform_vector(seed);
                if seen.insert(v.clone(), ()).is_none() {
                    self.construction_steps.push(ConstructStep::CarvePlane {
                        normal: v.clone(),
                        distance: v.mag(),
                        label: f,
                    });
                    f += 1;
                }
            }
        }

        if self.do_twist_cuts {
            let mut seen = VectorHashMap::new();
            for elem in g.elements() {
                let v = g[elem].transform_vector(&self.seeds[0]);
                if seen.insert(v.clone(), ()).is_none() {
                    self.construction_steps.push(ConstructStep::SlicePlane {
                        normal: v.clone(),
                        distance: v.mag() * self.cut_depth,
                    })
                }
            }
        }

        Ok(())
    }
    fn try_generate_mesh(&mut self) -> Result<ShapeArena> {
        self.try_generate_partial_mesh(self.construction_steps.len())
    }
    fn try_generate_partial_mesh(&mut self, num_steps: usize) -> Result<ShapeArena> {
        let mut arena = ShapeArena::new_euclidean_cga(self.ndim);

        let result = || -> Result<()> {
            for step in &self.construction_steps[..num_steps] {
                match step {
                    ConstructStep::CarvePlane {
                        normal,
                        distance,
                        label,
                    } => arena.carve_plane(normal, *distance, *label)?,

                    ConstructStep::SlicePlane { normal, distance } => {
                        arena.slice_plane(normal, *distance)?
                    }
                }
            }

            Ok(())
        }();

        #[cfg(debug_assertions)]
        arena.dump_log_file();

        // println!("{result:?}");
        result?;

        self.num_steps_executed = num_steps;
        Ok(arena)
    }
}

fn vector_edit(ui: &mut egui::Ui, v: &mut Vector, ndim: u8) -> bool {
    v.resize(ndim);
    let mut changed = false;
    ui.horizontal(|ui| {
        for i in 0..ndim {
            let r = ui
                .add(
                    egui::DragValue::new(&mut v[i])
                        .speed(0.01)
                        .fixed_decimals(1),
                )
                .on_hover_text(format!("Dim {i}"));
            changed |= r.changed();
        }
    });
    changed
}

#[derive(Debug, Clone)]
pub enum ConstructStep {
    CarvePlane {
        normal: Vector,
        distance: Float,
        label: u16,
    },
    SlicePlane {
        normal: Vector,
        distance: Float,
    },
}

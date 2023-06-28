use anyhow::Result;
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
}
impl Default for PuzzleSetup {
    fn default() -> Self {
        let mut ret = Self {
            ndim: 3,
            schlafli: "4,2".to_string(),
            seeds: vec![vector![0.0, 1.0, 1.0]],
            do_twist_cuts: false,
            cut_depth: 0.0,
            ignore_errors: false,

            construction_steps: vec![],
            num_steps_executed: 0,
            error_string: None,
        };

        let result = ret.recompute_steps();
        ret.error_string = result.err().map(|e| e.to_string());

        ret
    }
}
impl PuzzleSetup {
    pub fn ui(&mut self, ui: &mut egui::Ui, app: &mut App) {
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
            let result = self.recompute_steps();
            self.error_string = result.err().map(|e| e.to_string());
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
            self.error_string = None;
            match result {
                Ok(arena) => {
                    let mut mesh = None;
                    match Mesh::from_arena(&arena, self.ignore_errors) {
                        Ok(m) => mesh = Some(m),
                        Err(e) => self.error_string = Some(e.to_string()),
                    }

                    active_view
                        .as_ref()
                        .unwrap()
                        .lock()
                        .set_mesh(&app.gfx, arena, mesh.as_ref());
                }
                Err(e) => self.error_string = Some(e.to_string()),
            }
        }

        ui.separator();
    }

    fn recompute_steps(&mut self) -> Result<()> {
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
    fn try_generate_mesh(&mut self) -> Result<ShapeArena<EuclideanCgaManifold>> {
        self.try_generate_partial_mesh(self.construction_steps.len())
    }
    fn try_generate_partial_mesh(
        &mut self,
        num_steps: usize,
    ) -> Result<ShapeArena<EuclideanCgaManifold>> {
        let mut arena = ShapeArena::new_euclidean_cga(self.ndim);

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

        arena.dump_log_file();

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

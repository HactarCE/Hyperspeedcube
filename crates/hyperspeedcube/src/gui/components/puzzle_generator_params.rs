use hyperpuzzle::{CatalogArgValue, CatalogId, GeneratorParamType, GeneratorParamValue};

const GENERATOR_SLIDER_WIDTH: f32 = 200.0;

pub struct PuzzleGeneratorUi<'a> {
    pub puzzle_id: &'a mut CatalogId,
}

impl egui::Widget for PuzzleGeneratorUi<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let catalog = hyperpuzzle::catalog();
        let Some(g) = catalog.puzzles.generators.get(&*self.puzzle_id.base) else {
            return ui.colored_label(
                ui.visuals().error_fg_color,
                format!("No puzzle or generator with ID {:?}", self.puzzle_id.base),
            );
        };

        // Ensure correct number of parameters
        if self.puzzle_id.args.len() != g.params.len() {
            *self.puzzle_id = g.default_id();
        }

        ui.scope(|ui| {
            for (param, param_value) in std::iter::zip(&g.params, &mut self.puzzle_id.args) {
                ui.label(&param.name);

                match param.ty {
                    GeneratorParamType::Int { min, max } => {
                        let mut i = param_value.to_int().unwrap_or(min);
                        ui.spacing_mut().slider_width = GENERATOR_SLIDER_WIDTH;
                        ui.add(egui::Slider::new(&mut i, min..=max).logarithmic(true));
                        ui.separator();
                        *param_value = i.into();
                    }

                    GeneratorParamType::Puzzle => {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "puzzle parameter is not yet supported",
                        );
                    }
                }
            }
        })
        .response
    }
}

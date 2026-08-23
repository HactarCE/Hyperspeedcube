use hyperpuzzle::{
    CatalogId, CatalogIdValue, CatalogWord, GeneratorParamType, Puzzle, TypedCatalogIdValue,
};
use itertools::Itertools;

use crate::gui::components::catalog_menu::PuzzleCatalogMenuUi;

pub const GENERATOR_SLIDER_WIDTH: f32 = 200.0;

#[derive(Debug, Clone)]
pub struct PuzzleGeneratorUi {
    pub generator_id: CatalogWord,
    pub param_uis: Vec<(Option<String>, GeneratorParamUi)>,
}

impl egui::Widget for &mut PuzzleGeneratorUi {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let catalog = hyperpuzzle::catalog();
        let Some(g) = catalog.get_generator::<Puzzle>(&self.generator_id) else {
            return ui.colored_label(
                ui.visuals().error_fg_color,
                format!(
                    "no puzzle or puzzle generator with ID `{}`",
                    self.generator_id,
                ),
            );
        };

        // Ensure correct number of parameters
        if self.param_uis.len() != g.params.len() {
            self.param_uis = g
                .params
                .iter()
                .map(|p| GeneratorParamUi::new(&p.ty, p.default.clone(), Some(p.name.clone())))
                .collect();
        }

        ui.scope(|ui| {
            ui.spacing_mut().slider_width = GENERATOR_SLIDER_WIDTH;
            let egui_id = egui::Id::new(&self.generator_id);
            for (i, (label, param_ui)) in self.param_uis.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    if let Some(l) = label {
                        ui.label(&*l);
                    }
                    ui.add(param_ui);
                    ui.separator();
                });
            }
        })
        .response
    }
}

impl PuzzleGeneratorUi {
    pub fn new(generator_id: CatalogWord) -> Self {
        Self {
            generator_id,
            param_uis: vec![],
        }
    }

    pub fn from_generated_id(puzzle_id: CatalogId) -> Self {
        Self {
            generator_id: puzzle_id.base.word,
            param_uis: vec![], // TODO: fill in defaults
        }
    }

    pub fn generated_id(&self) -> Option<CatalogId> {
        Some(CatalogId::new(
            self.generator_id.clone(),
            self.param_uis
                .iter()
                .map(|(_label, param_ui)| param_ui.to_id_value())
                .collect::<Option<Vec<_>>>()?,
            None, // TODO subset
        ))
    }
}

#[derive(Debug, Clone)]
pub enum GeneratorParamUi {
    Bool {
        label: String,
        current: bool,
    },
    Int {
        min: i64,
        max: i64,
        current: i64,
    },
    Puzzle {
        menu_ui: PuzzleCatalogMenuUi,
    },
    List {
        ty: GeneratorParamType,
        elements: Vec<GeneratorParamUi>,
    },
}

impl GeneratorParamUi {
    pub fn new(
        ty: &GeneratorParamType,
        default: CatalogIdValue,
        mut label: Option<String>,
    ) -> (Option<String>, Self) {
        let this = match ty {
            GeneratorParamType::Bool => Self::Bool {
                label: label.take().unwrap_or_else(|| "Value".to_string()),
                current: default.to_bool().unwrap_or(false),
            },
            &GeneratorParamType::Int { min, max } => Self::Int {
                min,
                max,
                current: default.to_int().unwrap_or(min),
            },
            GeneratorParamType::Puzzle { menu } => Self::Puzzle {
                menu_ui: PuzzleCatalogMenuUi::new(menu.clone(), default.into_id().ok()),
            },
            GeneratorParamType::Id { ty } => todo!("ID parameter"),
            GeneratorParamType::List(inner) => Self::List {
                ty: (**inner).clone(),
                elements: default
                    .into_list()
                    .unwrap_or(vec![])
                    .into_iter()
                    .enumerate()
                    .map(|(i, e)| Self::new(inner, e, None).1)
                    .collect(),
            },
        };
        (label, this)
    }

    pub fn to_id_value(&self) -> Option<CatalogIdValue> {
        match self {
            &GeneratorParamUi::Bool { current, .. } => Some(current.into()),
            &GeneratorParamUi::Int { current, .. } => Some(current.into()),
            GeneratorParamUi::Puzzle { menu_ui } => {
                menu_ui.get_selected_puzzle().map(CatalogIdValue::Id)
            }
            GeneratorParamUi::List { elements, .. } => elements
                .iter()
                .map(|e| e.to_id_value())
                .collect::<Option<_>>()
                .map(CatalogIdValue::List),
        }
    }
}

impl egui::Widget for &mut GeneratorParamUi {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.scope(|ui| match self {
            GeneratorParamUi::Bool { label, current } => {
                ui.checkbox(current, &*label);
            }
            GeneratorParamUi::Int { min, max, current } => {
                ui.add(egui::Slider::new(current, *min..=*max).logarithmic(true));
            }
            GeneratorParamUi::Puzzle { menu_ui } => {
                ui.add(menu_ui);
            }
            GeneratorParamUi::List { ty, elements } => {
                let mut i = 0;
                elements.retain_mut(|elem| {
                    let mut keep = true;
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            keep &= !ui.button(mdi!(ui, TRASH_CAN_OUTLINE)).clicked();
                            ui.vertical(|ui| ui.add(elem));
                        });
                        ui.separator();
                    });
                    i += 1;
                    keep
                });
                if ui.button("Add").clicked() {
                    elements.push(GeneratorParamUi::new(ty, CatalogIdValue::Error, None).1);
                }
            }
        })
        .response
    }
}

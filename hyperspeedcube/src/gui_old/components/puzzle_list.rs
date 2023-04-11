use crate::puzzle::{rubiks_3d, rubiks_4d, PuzzleType, PuzzleTypeEnum};

pub fn puzzle_type_menu(ui: &mut egui::Ui) -> Option<PuzzleTypeEnum> {
    let mut ret = None;

    let default = PuzzleTypeEnum::Rubiks3D {
        layer_count: rubiks_3d::DEFAULT_LAYER_COUNT,
    };
    let r = ui.menu_button(default.family_display_name(), |ui| {
        for layer_count in rubiks_3d::MIN_LAYER_COUNT..=rubiks_3d::MAX_LAYER_COUNT {
            let ty = PuzzleTypeEnum::Rubiks3D { layer_count };
            if ui.button(ty.name()).clicked() {
                ui.close_menu();
                ret = Some(ty);
            }
        }
    });
    if r.response.clicked() {
        ui.close_menu();
        ret = Some(default);
    }

    let default = PuzzleTypeEnum::Rubiks4D {
        layer_count: rubiks_4d::DEFAULT_LAYER_COUNT,
    };
    let r = ui.menu_button(default.family_display_name(), |ui| {
        for layer_count in rubiks_4d::LAYER_COUNT_RANGE {
            let ty = PuzzleTypeEnum::Rubiks4D { layer_count };
            if ui.button(ty.name()).clicked() {
                ui.close_menu();
                ret = Some(ty);
            }
        }
    });
    if r.response.clicked() {
        ui.close_menu();
        ret = Some(default);
    }

    ret
}

use itertools::Itertools;

pub fn puzzle_type_menu(ui: &mut egui::Ui) -> Option<String> {
    let ret = crate::LIBRARY.with(|lib| {
        lib.puzzles()
            .values()
            .sorted_by_key(|puzzle| &puzzle.name)
            .find(|puzzle| ui.button(&puzzle.name).clicked())
            .map(|puzzle| puzzle.id.clone())
    });

    if ret.is_some() {
        ui.close_menu()
    }

    ret
}

use hyperprefs::ext::reorderable::ReorderableCollection;

use crate::app::App;
use crate::gui::components::DragAndDrop;
use crate::gui::util::EguiTempValue;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    ui.add_enabled_ui(app.active_puzzle.has_puzzle(), |ui| {
        let v = EguiTempValue::new(ui);
        let mut keybind_groups = v.get().unwrap_or(vec![
            KeybindGroup {
                name: "Global".to_string(),
                keybinds: vec![Keybind {}, Keybind {}, Keybind {}, Keybind {}],
            },
            KeybindGroup {
                name: "Hypercube".to_string(),
                keybinds: vec![Keybind {}, Keybind {}, Keybind {}, Keybind {}],
            },
            KeybindGroup {
                name: "Cube".to_string(),
                keybinds: vec![Keybind {}, Keybind {}, Keybind {}, Keybind {}],
            },
        ]);

        let mut group_dnd = DragAndDrop::new(ui);
        let mut keybind_dnd = DragAndDrop::new(ui);
        for (group_index, group) in keybind_groups.iter().enumerate() {
            group_dnd.vertical_reorder_by_handle(ui, group_index, |ui, is_dragging| {
                ui.vertical(|ui| {
                    ui.strong(&group.name);
                    for (keybind_index, keybind) in group.keybinds.iter().enumerate() {
                        let index = (group_index, keybind_index);
                        keybind_dnd.vertical_reorder_by_handle(ui, index, |ui, is_dragging| {
                            ui.label("hi");
                        });
                    }
                });
            });
        }
        keybind_dnd.end_reorder(ui, &mut A);
        group_dnd.end_reorder(ui, &mut keybind_groups);

        v.set(Some(keybind_groups));
    });
}

struct A;
impl ReorderableCollection<(usize, usize)> for A {
    fn reorder(
        &mut self,
        drag: hyperprefs::ext::reorderable::DragAndDropResponse<(usize, usize), (usize, usize)>,
    ) {
        // TODO: handle this
    }
}

#[derive(Debug, Default, Clone)]
struct KeybindGroup {
    name: String,
    keybinds: Vec<Keybind>,
}

#[derive(Debug, Default, Clone)]
struct Keybind {}

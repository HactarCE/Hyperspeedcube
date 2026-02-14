use hcegui::dnd::Dnd;

use crate::app::App;
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

        let mut group_dnd = Dnd::new(ui.ctx(), "group_dnd");
        let mut keybind_dnd = Dnd::new(ui.ctx(), "keybind_dnd");
        for (group_index, group) in keybind_groups.iter().enumerate() {
            group_dnd.reorderable_with_handle(ui, group_index, |ui, is_dragging| {
                ui.vertical(|ui| {
                    ui.strong(&group.name);
                    for (keybind_index, keybind) in group.keybinds.iter().enumerate() {
                        let index = (group_index, keybind_index);
                        keybind_dnd.reorderable_with_handle(ui, index, |ui, is_dragging| {
                            ui.label("hi");
                        });
                    }
                });
            });
        }
        keybind_dnd.finish(ui);
        if let Some(r) = group_dnd.finish(ui).if_done_dragging() {
            r.reorder(&mut keybind_groups);
        }

        v.set(Some(keybind_groups));
    });
}

#[derive(Debug, Default, Clone)]
struct KeybindGroup {
    name: String,
    keybinds: Vec<Keybind>,
}

#[derive(Debug, Default, Clone)]
struct Keybind {}

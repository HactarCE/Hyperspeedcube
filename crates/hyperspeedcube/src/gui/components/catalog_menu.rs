use std::{
    any::{Any, TypeId},
    cmp::Reverse,
    collections::HashMap,
    i32,
    ops::{Deref, DerefMut, Range, RangeInclusive},
};

use egui::{NumExt, TextBuffer};
use hyperpuzzle::{
    CatalogId, FloatMinMaxIteratorExt,
    catalog::{Menu, MenuContent, MenuPath},
    symmetric::hps::HpsSymmetric,
};
use itertools::Itertools;

use crate::{
    app::App,
    gui::{
        EguiValue,
        components::PuzzleGeneratorUi,
        util::{EguiTempValue, text_width},
    },
};

const SECTION_TEXT_SIZE: f32 = 15.0;
const PARAMETERS_HEADING: &str = "Parameters";
const OTHER_SECTION_TITLE: &str = "Other";
const HEIGHT: f32 = 300.0;
const BIG_BUTTON_HEIGHT: f32 = 32.0;

#[derive(Debug, Default, Clone)]
struct PuzzleCatalogMenuState {
    puzzle_id: String,
    menu_path: String,
    popup_open: bool,
}

#[derive(Debug)]
pub struct PuzzleCatalogMenu {
    ctx: egui::Context,
    id: egui::Id,
    menu_id: TypeId,
    state: EguiValue<PuzzleCatalogMenuState>,
}

impl PuzzleCatalogMenu {
    pub fn new(ctx: &egui::Context, id: egui::Id, menu_id: TypeId) -> Self {
        Self {
            ctx: ctx.clone(),
            id,
            menu_id,
            state: EguiValue::load_or_default(ctx, id),
        }
    }

    pub fn reset(self) {
        EguiValue::remove(self.state);
    }
}

impl egui::Widget for &mut PuzzleCatalogMenu {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let state = &mut *self.state;

        let r = ui.text_edit_singleline(&mut state.puzzle_id);
        if r.has_focus() {
            state.popup_open = false;
        }
        if r.changed()
            && let Some(menu) = hyperpuzzle::catalog().menus.get(&self.menu_id)
            && let Some(menu_path) = menu.puzzle_id_to_path(&state.puzzle_id)
        {
            state.menu_path = menu_path.to_string();
        }

        let r = ui.button("Puzzle selector");
        if r.clicked() {
            state.popup_open ^= true;
        }

        // TODO: manual popup so I can have control over the width

        egui::Popup::new(
            self.id,
            ui.ctx().clone(),
            egui::PopupAnchor::from(&r),
            egui::LayerId::new(egui::Order::Foreground, unique_id!()),
        )
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .open_bool(&mut state.popup_open)
        .show(|ui| {
            let catalog = hyperpuzzle::catalog();
            let Some(menu) = catalog.menus.get(&self.menu_id) else {
                ui.colored_label(ui.visuals().error_fg_color, "Unknown puzzle menu");
                return;
            };

            ui.style_mut().interaction.selectable_labels = false;

            ui.horizontal(|ui| {
                ui.take_available_width();
                ui.set_height(HEIGHT);

                let mut selected_path = MenuPath::from_str(&state.menu_path)
                    .or_else(|| menu.puzzle_id_to_path(&state.puzzle_id))
                    .unwrap_or_default();

                let mut index = 0;
                while index <= selected_path.len() {
                    if show_menu_column(ui, menu, &mut selected_path, index, &mut state.puzzle_id) {
                        break;
                    }
                    index += 1;
                }

                state.menu_path = selected_path.to_string();
            });
        });

        r
    }
}

/// Shows a menu column and returns `true` if it is the last column, which
/// consumes all the remaining width.
fn show_menu_column<'a>(
    ui: &mut egui::Ui,
    menu: &'a Menu,
    selected_path: &mut MenuPath<'a>,
    index: usize,
    puzzle_id: &mut String,
) -> bool {
    let Some((heading, ui_elements)) = layout_menu_column(ui, menu, selected_path, index) else {
        return false; // skip
    };

    let is_final = !ui_elements
        .iter()
        .any(|e| matches!(e, MenuUiElement::PathComponent { .. }));

    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        if !is_final {
            let max_text_width = ui_elements
                .iter()
                .map(|elem| elem.min_width(ui))
                .max_float()
                .unwrap_or(0.0);
            let ui_width = (max_text_width + ui.spacing().scroll.allocated_width())
                .at_least(text_width(ui, egui::RichText::new(heading).heading()));
            ui.set_width(ui_width);
        }
        ui.heading(heading);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(&selected_path.truncate(index))
            .show(ui, |ui| {
                for elem in ui_elements {
                    elem.show(ui, menu, selected_path, puzzle_id);
                }
            });
    });

    if !is_final {
        ui.separator();
    }

    is_final
}

fn menu_path_button<'a>(
    ui: &mut egui::Ui,
    menu: &'a Menu,
    selected_path: &mut MenuPath<'a>,
    path: MenuPath<'a>,
) {
    let is_selected = selected_path.starts_with(path);
    let last_component = path.last_component();
    let mut r = ui.selectable_label(is_selected, last_component);
    r = match last_component {
        "FT" => r.on_hover_text("Facet-Turning"),
        "RT" => r.on_hover_text("Ridge-Turning"),
        "PT" => r.on_hover_text("Peak-Turning"),
        "ET" => r.on_hover_text("Edge-Turning"),
        "VT" => r.on_hover_text("Vertex-Turning"),
        "FVT" => r.on_hover_text("Facet/Vertex-Turning"),
        "RET" => r.on_hover_text("Ridge/Edge-Turning"),
        _ => r,
    };
    if r.clicked() || r.double_clicked() {
        *selected_path = menu.default_descendent(path);
    }
    if r.double_clicked()
        && let Some(content) = menu.get_content(*selected_path)
        && let MenuContent::End { .. } = content
    {
        ui.close();
    }
}

fn layout_menu_column<'a>(
    ui: &mut egui::Ui,
    menu: &'a Menu,
    selected_path: &mut MenuPath<'a>,
    mut index: usize,
) -> Option<(&'a str, Vec<MenuUiElement<'a>>)> {
    let mut partial_path = selected_path.truncate(index);

    let heading;
    let mut ui_elements = vec![];

    match menu.get_content(partial_path)? {
        MenuContent::Column { title } => {
            heading = title.as_str();
            let sections = menu
                .children(partial_path)
                .filter(|&child| menu.is_section(child));
            let non_sections = menu
                .children(partial_path)
                .filter(|&child| !menu.is_section(child));

            let mut needs_other_section_title = false;
            for section_path in sections {
                needs_other_section_title = true;
                ui_elements.push(MenuUiElement::SectionTitle(section_path.last_component()));
                for grandchild_path in menu.children(section_path) {
                    ui_elements.push(MenuUiElement::PathComponent(grandchild_path));
                }
            }

            for child_path in non_sections {
                if std::mem::take(&mut needs_other_section_title) {
                    ui_elements.push(MenuUiElement::OtherSectionTitle);
                }
                ui_elements.push(MenuUiElement::PathComponent(child_path));
            }
        }

        MenuContent::Section => return None, // shown in previous column

        MenuContent::Inline { .. } | MenuContent::End { .. } => {
            heading = PARAMETERS_HEADING;
            while index <= partial_path.len()
                && let Some(content) = menu.get_content(partial_path)
            {
                match content {
                    MenuContent::Column { .. } | MenuContent::Section => {
                        ui_elements.push(MenuUiElement::Error(
                            "inline nodes must not be followed by column or section nodes",
                        ));
                    }
                    MenuContent::Inline { label } => {
                        let options = menu.children(partial_path).collect();
                        ui_elements.push(MenuUiElement::Inline { label, options });
                    }
                    MenuContent::End { id } => {
                        ui_elements.push(MenuUiElement::End { id });
                    }
                }
                index += 1;
                partial_path = selected_path.truncate(index);
            }
        }
    }

    Some((heading, ui_elements))
}

enum MenuUiElement<'a> {
    SectionTitle(&'a str),
    OtherSectionTitle,
    PathComponent(MenuPath<'a>),
    Inline {
        label: &'a str,
        options: Vec<MenuPath<'a>>,
    },
    End {
        id: &'a CatalogId,
    },
    Error(&'a str),
}

impl<'a> MenuUiElement<'a> {
    pub fn min_width(&self, ui: &mut egui::Ui) -> f32 {
        match self {
            MenuUiElement::SectionTitle(s) => text_width(ui, Self::section_text(s)),
            MenuUiElement::OtherSectionTitle => {
                text_width(ui, Self::section_text(OTHER_SECTION_TITLE))
            }
            MenuUiElement::PathComponent(path) => {
                text_width(ui, path.last_component()) + ui.spacing().button_padding.x * 2.0
            }
            MenuUiElement::Inline { label, options } => ui.available_width(),
            MenuUiElement::End { id } => ui.available_width(),
            MenuUiElement::Error(_) => ui.available_width(),
        }
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        menu: &'a Menu,
        selected_path: &mut MenuPath<'a>,
        puzzle_id: &mut String,
    ) {
        match self {
            MenuUiElement::SectionTitle(s) => {
                ui.strong(Self::section_text(s));
            }
            MenuUiElement::OtherSectionTitle => {
                ui.strong(Self::section_text(OTHER_SECTION_TITLE));
            }
            MenuUiElement::PathComponent(path) => menu_path_button(ui, menu, selected_path, path),
            MenuUiElement::Inline { label, options } => {
                ui.strong(label);
                ui.horizontal_wrapped(|ui| {
                    for path in options {
                        menu_path_button(ui, menu, selected_path, path);
                    }
                });
                ui.separator();
            }
            MenuUiElement::End { id } => {
                if id.args.is_empty() {
                    let mut parsed_puzzle_id = puzzle_id
                        .parse::<CatalogId>()
                        .ok()
                        .filter(|old| old.base == id.base)
                        .unwrap_or_else(|| id.clone());
                    ui.add(PuzzleGeneratorUi {
                        puzzle_id: &mut parsed_puzzle_id,
                    });
                    *puzzle_id = parsed_puzzle_id.to_string();
                } else {
                    *puzzle_id = id.to_string();
                }
                ui.vertical_centered_justified(|ui| {
                    let big_button_text = egui::RichText::new("Select puzzle");
                    let big_button_size = egui::vec2(ui.available_width(), BIG_BUTTON_HEIGHT);
                    if ui
                        .add(egui::Button::new(big_button_text).min_size(big_button_size))
                        .clicked()
                    {
                        ui.close();
                    };
                });
            }
            MenuUiElement::Error(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
        }
    }

    fn section_text(s: &str) -> egui::RichText {
        egui::RichText::new(s).size(SECTION_TEXT_SIZE)
    }
}

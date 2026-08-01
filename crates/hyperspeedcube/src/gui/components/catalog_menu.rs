use std::any::TypeId;
use std::time::Duration;

use egui::NumExt;
use hyperpuzzle::catalog::{Menu, MenuContent, MenuPath};
use hyperpuzzle::{CatalogId, FloatMinMaxIteratorExt};

use crate::gui::EguiValue;
use crate::gui::components::{GENERATOR_SLIDER_WIDTH, PuzzleGeneratorUi};
use crate::gui::util::text_width;

const SECTION_TEXT_SIZE: f32 = 15.0;
const PARAMETERS_HEADING: &str = "Parameters";
const OTHER_SECTION_TITLE: &str = "Other";
const MIN_WIDTH: f32 = 800.0;
const HEIGHT: f32 = 300.0;
const BIG_BUTTON_HEIGHT: f32 = 32.0;

#[derive(Debug, Clone)]
pub struct PuzzleCatalogMenuUi {
    menu_id: String,
    puzzle_id: String,
    menu_path: String,
    is_open: bool,
    generator_ui: Option<PuzzleGeneratorUi>,
}

impl PuzzleCatalogMenuUi {
    pub fn new(menu_id: String, default_selected_puzzle: Option<CatalogId>) -> Self {
        let mut ret = Self {
            menu_id,
            puzzle_id: default_selected_puzzle
                .map(|id| id.to_string())
                .unwrap_or_default(),
            menu_path: String::new(),
            is_open: false,
            generator_ui: None,
        };
        ret.set_menu_path_from_puzzle_id();
        ret
    }

    pub fn set_selected_puzzle(&mut self, id: CatalogId) {
        self.puzzle_id = id.to_string();
    }
    pub fn get_selected_puzzle(&self) -> Option<CatalogId> {
        self.puzzle_id.parse().ok()
    }

    fn set_menu_path_from_puzzle_id(&mut self) {
        if let Some(menu) = hyperpuzzle::catalog().menus.get(self.menu_id.as_str())
            && let Ok(puzzle_id) = self.puzzle_id.parse::<CatalogId>()
            && let Some(menu_path) = menu.puzzle_id_to_path(&puzzle_id.base)
        {
            self.menu_path = menu_path.to_string();
        }
    }
}

impl egui::Widget for &mut PuzzleCatalogMenuUi {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id_salt = 0;

        let collapsing_response = egui::CollapsingHeader::new("Select puzzle")
            .id_salt(id_salt)
            .open(ui.is_sizing_pass().then_some(self.is_open))
            .show_background(true)
            .show_unindented(ui, |ui| {
                ui.group(|ui| {
                    let catalog = hyperpuzzle::catalog();
                    let Some(menu) = catalog.menus.get(self.menu_id.as_str()) else {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            format!("unknown puzzle menu {:?}", self.menu_id),
                        );
                        return;
                    };

                    ui.style_mut().interaction.selectable_labels = false;

                    // ui.set_min_width(MIN_WIDTH);
                    ui.horizontal(|ui| {
                        ui.take_available_width();
                        ui.set_height(HEIGHT);

                        let mut selected_path = MenuPath::from_str(&self.menu_path)
                            .or_else(|| menu.puzzle_id_to_path(&self.puzzle_id))
                            .unwrap_or_default();

                        let mut index = 0;
                        while index <= selected_path.len() {
                            if show_menu_column(
                                ui,
                                menu,
                                &mut selected_path,
                                index,
                                &mut self.puzzle_id,
                                &mut self.generator_ui,
                            ) {
                                break;
                            }
                            index += 1;
                        }

                        self.menu_path = selected_path.to_string();
                    });
                });
            });

        let collapsing_state_id = ui
            .id()
            .with(egui::Id::from("child"))
            .with(egui::Id::new(id_salt));

        // Reserve some minimal amount of space for the textedit.
        ui.set_min_width(collapsing_response.header_response.rect.width() + 200.0);

        let rect = collapsing_response.header_response.rect;
        let text_rect = rect
            .with_min_x(rect.right() + ui.spacing().item_spacing.x)
            .with_max_x(ui.max_rect().max.x);
        let text_edit_response =
            ui.place(text_rect, egui::TextEdit::singleline(&mut self.puzzle_id));
        if text_edit_response.changed() {
            self.set_menu_path_from_puzzle_id();
        }

        if !ui.is_sizing_pass() {
            // `CollapsingHeader` provides no way to tell the state directly from
            // the response, so we have to do this instead.
            if let Some(mut collapsing_state) =
                egui::collapsing_header::CollapsingState::load(ui, collapsing_state_id)
            {
                if text_edit_response.has_focus() && !ui.input(|i| i.pointer.any_down()) {
                    collapsing_state.set_open(false); // TODO: remove this. only set puzzle ID when something changed
                }
                self.is_open = collapsing_state.is_open();
                collapsing_state.store(ui);
            } else {
                log::warn!("collapsing_state_id is incorrect");
            }
        }

        text_edit_response
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
    generator_ui: &mut Option<PuzzleGeneratorUi>,
) -> bool {
    let Some((heading, ui_elements)) = layout_menu_column(menu, selected_path, index) else {
        return false; // skip
    };

    let is_final = !ui_elements
        .iter()
        .any(|e| matches!(e, MenuUiElement::PathComponent { .. }));

    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
        let max_text_width = ui_elements
            .iter()
            .map(|elem| elem.min_width(ui))
            .max_float()
            .unwrap_or(0.0);
        let ui_width = (max_text_width + ui.spacing().scroll.allocated_width())
            .at_least(text_width(ui, egui::RichText::new(heading).heading()))
            .at_least(if is_final { ui.available_width() } else { 0.0 });
        ui.set_width(ui_width);
        if ui.is_sizing_pass() {
            return;
        }
        ui.heading(heading);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt(selected_path.truncate(index))
            .show(ui, |ui| {
                for elem in ui_elements {
                    elem.show(ui, menu, selected_path, puzzle_id, generator_ui);
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
}

fn layout_menu_column<'a>(
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
            MenuUiElement::Inline { label, options } => options
                .iter()
                .map(|option| text_width(ui, option.last_component()))
                .max_float()
                .unwrap_or(0.0)
                .at_least(text_width(ui, *label)),
            MenuUiElement::End { .. } => {
                GENERATOR_SLIDER_WIDTH + ui.spacing().item_spacing.x + ui.spacing().interact_size.x // TODO: just use a constant for the whole thing
            }
            MenuUiElement::Error(_) => 0.0,
        }
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        menu: &'a Menu,
        selected_path: &mut MenuPath<'a>,
        puzzle_id: &mut String,
        generator_ui: &mut Option<PuzzleGeneratorUi>,
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
                    if let Some(g) = generator_ui
                        && *g.generator_id != *id.base
                    {
                        *generator_ui = None;
                    }
                    let g =
                        generator_ui.get_or_insert_with(|| PuzzleGeneratorUi::new(id.base.clone()));
                    ui.add(&mut *g);
                    if let Some(new_puzzle_id) = g.generated_id() {
                        *puzzle_id = new_puzzle_id.to_string();
                    }
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

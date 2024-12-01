use std::borrow::Cow;
use std::collections::HashMap;

use itertools::Itertools;

use super::{FilterCheckbox, FilterCheckboxState};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum TagFilterState {
    Included,
    Excluded,
    Mixed,
}

pub struct TagMenu {
    tag_states: HashMap<String, TagFilterState>,
    show_experimental: bool,
}
impl TagMenu {
    pub fn new<'a>(
        included: impl IntoIterator<Item = String>,
        excluded: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut tag_states = HashMap::new();

        let specified_tags = itertools::chain(
            std::iter::repeat(TagFilterState::Included).zip(included),
            std::iter::repeat(TagFilterState::Excluded).zip(excluded),
        );
        for (state, tag) in specified_tags {
            // Mark the ancestor tags as mixed unless they are explicitly
            // specified as well.
            for parent in hyperpuzzle::TAGS.ancestors(&tag) {
                tag_states
                    .entry(parent.to_owned())
                    .or_insert(TagFilterState::Mixed);
            }

            tag_states.insert(tag.to_owned(), state);
        }

        Self {
            tag_states,
            show_experimental: true,
        }
    }

    pub fn with_experimental(mut self, show_experimental: bool) -> Self {
        self.show_experimental = show_experimental;
        self
    }

    #[must_use]
    pub fn show(self, ui: &mut egui::Ui) -> egui::InnerResponse<Option<TagFilterAction>> {
        ui.scope(|ui| {
            let tags = hyperpuzzle::TAGS.menu();
            egui::ScrollArea::vertical()
                .show(ui, |ui| self.show_tag_menu_node(ui, tags))
                .inner
        })
    }

    #[must_use]
    fn show_tag_menu_node(
        &self,
        ui: &mut egui::Ui,
        node: &hyperpuzzle::TagMenuNode,
    ) -> Option<TagFilterAction> {
        match node {
            hyperpuzzle::TagMenuNode::Heading(heading_text) => {
                ui.strong(&**heading_text);
                None
            }

            hyperpuzzle::TagMenuNode::Separator => {
                ui.separator();
                None
            }

            hyperpuzzle::TagMenuNode::Tag {
                name,
                ty: _,
                display,
                subtags,

                auto: _,
                expected: _,
                list,
            } => {
                if name.as_deref() == Some("experimental") && !self.show_experimental {
                    return None;
                }

                let show_contents = |ui: &mut egui::Ui| {
                    if *list {
                        let name = name.as_deref().unwrap_or("");

                        crate::LIBRARY.with(|lib| {
                            if name == "colors/system" {
                                lib.color_systems()
                                    .iter()
                                    // Sort by name (could sort by ID instead)
                                    .sorted_unstable_by(|a, b| {
                                        Ord::cmp(a.display_name(), b.display_name())
                                    })
                                    .map(|color_system| {
                                        let name = color_system.display_name();
                                        self.tag_checkbox(ui, name, Some(&color_system.id), name)
                                    })
                                    .reduce(Option::or)
                                    .flatten()
                            } else {
                                itertools::chain(
                                    lib.puzzles().iter().map(|p| &p.tags),
                                    lib.puzzle_generators().iter().map(|g| &g.tags),
                                )
                                .filter_map(|tags| tags.get(name))
                                .flat_map(|tag_value| tag_value.to_strings())
                                .unique()
                                .sorted()
                                .map(|tag_value| {
                                    self.tag_checkbox(ui, name, Some(&tag_value), &tag_value)
                                })
                                .reduce(Option::or)
                                .flatten()
                            }
                        })
                    } else {
                        self.show_tag_menu_nodes(ui, &subtags)
                    }
                };

                match display {
                    hyperpuzzle::TagDisplay::Inline => self.show_tag_menu_nodes(ui, &subtags),
                    hyperpuzzle::TagDisplay::Submenu(label) => {
                        if subtags.is_empty() && !*list {
                            if let Some(name) = name {
                                self.tag_checkbox(ui, name, None, &label)
                            } else {
                                None // show nothing
                            }
                        } else {
                            self.tag_checkbox_menu(ui, name.as_deref(), &label, |ui| {
                                egui::ScrollArea::vertical().show(ui, show_contents).inner
                            })
                        }
                    }
                }
            }
        }
    }

    #[must_use]
    fn show_tag_menu_nodes(
        &self,
        ui: &mut egui::Ui,
        nodes: &[hyperpuzzle::TagMenuNode],
    ) -> Option<TagFilterAction> {
        nodes
            .iter()
            .map(|node| self.show_tag_menu_node(ui, node))
            .reduce(Option::or)
            .flatten()
    }

    #[must_use]
    fn tag_checkbox_menu(
        &self,
        ui: &mut egui::Ui,
        name: Option<&str>,
        display: &str,
        show_contents: impl FnOnce(&mut egui::Ui) -> Option<TagFilterAction>,
    ) -> Option<TagFilterAction> {
        ui.horizontal(|ui| {
            let r1;
            if let Some(name) = name {
                r1 = self.tag_checkbox(ui, name, None, "");
                ui.add_space(-ui.spacing().item_spacing.x - ui.spacing().button_padding.x);
            } else {
                r1 = None;
            }

            let r2 = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.menu_button(display, show_contents).inner
            });

            r1.or(r2.inner.flatten())
        })
        .inner
    }

    #[must_use]
    fn tag_checkbox(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        value: Option<&str>,
        display: &str,
    ) -> Option<TagFilterAction> {
        let mut tag_state = None;
        let tag_subtree_state = match self
            .tag_states
            .get(&format_tag_and_value(&(name, value.map(|s| s.to_owned()))))
        {
            Some(TagFilterState::Included) => {
                tag_state = Some(true);
                FilterCheckboxState::Coherent(&mut tag_state)
            }
            Some(TagFilterState::Excluded) => {
                tag_state = Some(false);
                FilterCheckboxState::Coherent(&mut tag_state)
            }
            Some(TagFilterState::Mixed) => FilterCheckboxState::Mixed,
            None => FilterCheckboxState::Coherent(&mut tag_state),
        };

        let r = ui.add(FilterCheckbox::new(
            super::FilterCheckboxAllowedStates::NeutralShowHide,
            tag_subtree_state,
            display,
        ));

        r.changed().then(|| TagFilterAction {
            tag: name.to_owned(),
            value: value.map(|s| s.to_owned()),
            new_state: tag_state,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFilterAction {
    pub tag: String,
    pub value: Option<String>,
    pub new_state: Option<bool>,
}
impl TagFilterAction {
    pub fn segment_string(&self) -> Option<String> {
        let mut ret = match self.new_state? {
            true => "#",
            false => "!#",
        }
        .to_owned();
        ret += &self.tag;
        if let Some(value) = &self.value {
            ret += "=";
            ret += &*escape_tag_value(value);
        }
        Some(ret)
    }
}

pub(crate) fn escape_tag_value(value_str: &str) -> Cow<'_, str> {
    if value_str.chars().any(|c| c.is_whitespace()) || value_str.contains('"') {
        Cow::Owned(format!("\"{}\"", value_str.replace("\"", "\\\"")))
    } else {
        Cow::Borrowed(value_str)
    }
}

pub(crate) fn unescape_tag_value(value_str: &str) -> Option<String> {
    // IIFE to mimic try_block
    if value_str.starts_with('"') && value_str.ends_with('"') {
        Some(
            value_str
                .strip_prefix('"')?
                .strip_suffix('"')?
                .replace("\\\"", "\""),
        )
    } else {
        Some(value_str.to_owned())
    }
}

pub(crate) fn format_tag_and_value((tag, value): &(&str, Option<String>)) -> String {
    match value {
        Some(v) => format!("{tag}={}", escape_tag_value(v)),
        None => tag.to_string(),
    }
}

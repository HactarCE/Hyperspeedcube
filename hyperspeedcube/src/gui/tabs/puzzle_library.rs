use std::{collections::HashMap, ops::Range, sync::Arc};

use hyperpuzzle::{
    lua::{GeneratorParamType, GeneratorParamValue, PuzzleGeneratorSpec, PuzzleSpec},
    TagValue,
};
use itertools::Itertools;

use crate::{
    app::App,
    gui::{
        components::{FilterCheckbox, FilterCheckboxState},
        markdown::{md, md_escape},
        util::EguiTempValue,
    },
    L,
};

pub const ID_MATCH_PENALTY: isize = 60;
pub const ALIAS_MATCH_PENALTY: isize = 50;
pub const ADDITIONAL_MATCH_INDENT: &str = "    ";

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let stored_search_query = EguiTempValue::new(ui);
    let mut search_query: String = stored_search_query.get().unwrap_or_default();

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    enum SubTab {
        #[default]
        Puzzles,
        Generators,
    }

    let subtab_stored = EguiTempValue::new(ui);
    let mut subtab = subtab_stored.get().unwrap_or_default();

    ui.group(|ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.selectable_value(&mut subtab, SubTab::Puzzles, "Puzzles");
            ui.selectable_value(&mut subtab, SubTab::Generators, "Puzzle generators");
            subtab_stored.set(Some(subtab));

            if crate::paths::lua_dir().is_ok() {
                if ui.button(L.library.reload_all_files).clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::F5))
                {
                    crate::reload_user_puzzles();
                }
            }
        });
    });

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.menu_button("Filter", |ui| {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| show_tags_filter_menu(ui, app, &mut search_query));
            });

            ui.add(
                egui::TextEdit::singleline(&mut search_query)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut |ui, string, wrap_width| {
                        let font_id = egui::TextStyle::Body.resolve(ui.style());
                        let basic_text_format = egui::TextFormat::simple(
                            font_id.clone(),
                            ui.visuals().widgets.inactive.text_color(),
                        );
                        let symbol_text_format =
                            egui::TextFormat::simple(font_id.clone(), egui::Color32::LIGHT_BLUE);
                        let bad_symbol_text_format =
                            egui::TextFormat::simple(font_id, egui::Color32::LIGHT_RED);

                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = wrap_width;
                        let query = Query::from_str(string);
                        let mut i = 0;
                        for (symbol_start, tag_name_start, tag_name_end) in query.tag_spans {
                            job.append(&string[i..symbol_start], 0.0, basic_text_format.clone());

                            job.append(
                                &string[symbol_start..tag_name_end],
                                0.0,
                                if hyperpuzzle::TAGS
                                    .contains_key(&string[tag_name_start..tag_name_end])
                                {
                                    symbol_text_format.clone()
                                } else {
                                    bad_symbol_text_format.clone()
                                },
                            );
                            i = tag_name_end;
                        }
                        job.append(&string[i..], 0.0, basic_text_format);
                        ui.fonts(|fonts| fonts.layout_job(job))
                    }),
            );
        });

        ui.add_space(ui.spacing().item_spacing.y);

        crate::LIBRARY.with(|lib| match subtab {
            SubTab::Puzzles => {
                filtered_list(
                    ui,
                    &mut search_query,
                    &lib.puzzles(),
                    "puzzles",
                    |_ui, mut r, puzzle| {
                        r = r.on_hover_ui(|ui| {
                            fn comma_list(strings: &[String]) -> String {
                                md_escape(&strings.iter().join(", ")).into_owned()
                            }

                            ui.strong(puzzle.display_name());

                            let inventors = puzzle.inventors();
                            if !inventors.is_empty() {
                                md(ui, &format!("**Inventors:** {}", comma_list(inventors)));
                            }

                            let authors = puzzle.authors();
                            if !authors.is_empty() {
                                md(ui, &format!("**Authors:** {}", comma_list(authors)));
                            }

                            let aliases = &puzzle.aliases;
                            if !aliases.is_empty() {
                                md(ui, &format!("**Aliases:** {}", comma_list(aliases)));
                            }

                            if let Some(url) = puzzle.wca_url() {
                                ui.hyperlink_to("WCA leaderboards", url);
                            }
                        });
                        if r.clicked() {
                            crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle.id));
                        }
                    },
                );
            }
            SubTab::Generators => {
                let popup_data_stored = EguiTempValue::<PuzzleGeneratorPopupData>::new(ui);
                let mut popup_data = popup_data_stored.get();

                filtered_list(
                    ui,
                    &mut search_query,
                    &lib.puzzle_generators(),
                    "puzzle generators",
                    |ui, r, puzzle_generator| {
                        let popup_id = popup_data_stored.id.with(&puzzle_generator.id);

                        if r.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            popup_data = Some(PuzzleGeneratorPopupData::new(puzzle_generator));
                        }

                        let close_behavior = egui::PopupCloseBehavior::CloseOnClickOutside;
                        if let Some(popup_data) = popup_data
                            .as_mut()
                            .filter(|data| data.id == puzzle_generator.id)
                        {
                            egui::popup_below_widget(ui, popup_id, &r, close_behavior, |ui| {
                                ui.strong(puzzle_generator.display_name());

                                for (param, value) in
                                    std::iter::zip(&puzzle_generator.params, &mut popup_data.params)
                                {
                                    match param.ty {
                                        GeneratorParamType::Int { min, max } => {
                                            let GeneratorParamValue::Int(i) = value;
                                            ui.horizontal(|ui| {
                                                ui.add(egui::Slider::new(i, min..=max));
                                                ui.label(&param.name);
                                            });
                                        }
                                    }
                                }

                                if ui.button("Generate puzzle").clicked() {
                                    ui.memory_mut(|mem| mem.close_popup());
                                    let puzzle_id = hyperpuzzle::generated_puzzle_id(
                                        &puzzle_generator.id,
                                        &popup_data.params,
                                    );
                                    crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle_id));
                                };
                            });
                        }
                    },
                );

                popup_data_stored.set(popup_data);
            }
        });
    });

    stored_search_query.set(Some(search_query.clone()));
}

fn filtered_list<'a, T: TagSearchable>(
    ui: &mut egui::Ui,
    query_string: &mut String,
    candidates: &[Arc<T>],
    plural_noun: &str,
    mut handle_resp: impl FnMut(&mut egui::Ui, egui::Response, &Arc<T>),
) {
    let query = Query::from_str(query_string);

    let filtered_candidates = candidates
        .iter()
        .filter_map(|obj| query.try_match(obj))
        .sorted_by_key(|m| -m.score);

    let n = filtered_candidates.len();
    let m = candidates.len();
    if query.is_empty() {
        md(ui, &format!("_**{m}** {plural_noun}_"));
    } else {
        md(ui, &format!("_**{n}/{m}** {plural_noun}_"));
    }

    ui.separator();

    let mut replacement_query_string = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                let mut any_incomplete_tags = false;
                for (symbol_start, tag_name_start, tag_name_end) in query.tag_spans {
                    let tag_name = &query_string[tag_name_start..tag_name_end];
                    if !hyperpuzzle::TAGS.contains_key(tag_name) {
                        any_incomplete_tags = true;

                        if tag_name.is_empty() {
                            for real_tag_name in hyperpuzzle::TAGS.keys().sorted() {
                                let r = ui.selectable_label(false, format!("#{real_tag_name}"));
                                if r.clicked() {
                                    let mut s = query_string.clone();
                                    s.replace_range(tag_name_start..tag_name_end, real_tag_name);
                                    replacement_query_string = Some(s);
                                }
                            }
                        }

                        for (real_tag_name, match_info) in hyperpuzzle::TAGS // TODO: pre-sort?
                            .keys()
                            .sorted()
                            .filter_map(|k| Some((k, sublime_fuzzy::best_match(tag_name, &k)?)))
                            .sorted_by_key(|(_, m)| -m.score())
                        {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                "#",
                                0.0,
                                egui::TextFormat::simple(
                                    egui::TextStyle::Button.resolve(ui.style()),
                                    ui.visuals().widgets.inactive.text_color(),
                                ),
                            );
                            render_fuzzy_match(
                                ui,
                                &mut job,
                                real_tag_name,
                                &match_info,
                                egui::TextStyle::Button,
                            );
                            let r = ui.selectable_label(false, job);
                            if r.clicked() {
                                let mut s = query_string.clone();
                                s.replace_range(tag_name_start..tag_name_end, real_tag_name);
                                replacement_query_string = Some(s);
                            }
                        }
                    }
                }

                if any_incomplete_tags {
                    ui.separator();
                }

                for query_match in filtered_candidates {
                    let obj = Arc::clone(&query_match.object);
                    let r = ui.add(query_match);
                    handle_resp(ui, r, &obj);
                }
            })
        });

    if let Some(s) = replacement_query_string {
        *query_string = s;
    }
}

fn show_tags_filter_menu(ui: &mut egui::Ui, app: &mut App, query_string: &mut String) {
    let mut new_query_string = query_string.clone();
    for node in &*hyperpuzzle::TAGS_MENU {
        show_tags_recursive(
            ui,
            app,
            &Query::from_str(&query_string),
            &mut new_query_string,
            node,
        );
    }
    *query_string = new_query_string;
}

fn show_tags_recursive(
    ui: &mut egui::Ui,
    app: &mut App,
    query: &Query<'_>,
    new_query_string: &mut String,
    node: &hyperpuzzle::TagMenuNode,
) {
    match node {
        hyperpuzzle::TagMenuNode::Heading(s) => {
            ui.strong(*s);
        }
        hyperpuzzle::TagMenuNode::Separator => {
            ui.separator();
        }
        hyperpuzzle::TagMenuNode::Tag {
            name,
            display,
            ty: _,
            subtags: children,

            section,
            checkbox,
            flatten,
            hidden,
            expected: _,
            list,
            auto: _,
        } => {
            if *hidden {
                return;
            }

            if name.as_deref() == Some("experimental") && !app.prefs.show_experimental_puzzles {
                return;
            }

            if name.as_ref().is_some_and(|&name| {
                crate::LIBRARY.with(|lib| {
                    !lib.puzzles()
                        .iter()
                        .any(|puzzle| match puzzle.tags.get(name) {
                            None | Some(TagValue::False) => false,
                            _ => true,
                        })
                })
            }) {
                return;
            }

            if *list {
                tag_checkbox_menu(
                    ui,
                    *&display,
                    name.expect("TODO expect"),
                    query,
                    new_query_string,
                    |ui, new_query_string: &mut String| {
                        match name {
                            Some("colors/system") => crate::LIBRARY.with(|lib| {
                                let mut color_systems = lib.color_systems();
                                color_systems.sort_by_key(|cs| cs.display_name().to_owned()); // TODO: precompute this or something!
                                for color_system in color_systems {
                                    ui.colored_label(ui.visuals().error_fg_color, "TODO");
                                    // tag_checkbox(ui, color_system.display_name());
                                }
                            }),
                            Some("author") => crate::LIBRARY.with(|lib| {
                                for author in lib
                                    .puzzles()
                                    .iter()
                                    .flat_map(|puzzle| puzzle.authors())
                                    .unique()
                                    .sorted()
                                {
                                    ui.colored_label(ui.visuals().error_fg_color, "TODO");
                                    // tag_checkbox(ui, &author);
                                }
                            }),
                            Some("inventor") => crate::LIBRARY.with(|lib| {
                                for inventor in lib
                                    .puzzles()
                                    .iter()
                                    .flat_map(|puzzle| puzzle.inventors())
                                    .unique()
                                    .sorted()
                                {
                                    ui.colored_label(ui.visuals().error_fg_color, "TODO");
                                    // tag_checkbox(ui, &inventor);
                                }
                            }),
                            _ => {
                                ui.colored_label(ui.visuals().error_fg_color, "invalid auto tag");
                            }
                        }
                    },
                );
                return;
            }

            let mut show_contents = |ui: &mut egui::Ui, new_query_string: &mut String| {
                for child in children {
                    show_tags_recursive(ui, app, query, new_query_string, child);
                }
            };

            if *flatten {
                show_contents(ui, new_query_string)
            } else if *section {
                ui.strong(*display);
                show_contents(ui, new_query_string);
            } else if *checkbox {
                if children.is_empty() {
                    tag_checkbox(
                        ui,
                        *display,
                        name.expect("TODO expect"),
                        query,
                        new_query_string,
                    );
                } else {
                    tag_checkbox_menu(
                        ui,
                        *display,
                        name.expect("TODO expect"),
                        query,
                        new_query_string,
                        show_contents,
                    );
                }
            } else {
                tag_submenu(ui, *display, |ui| show_contents(ui, new_query_string));
            }
        }
    }
}

fn tag_checkbox(
    ui: &mut egui::Ui,
    label: &str,
    tag_name: &str,
    query: &Query<'_>,
    query_string: &mut String,
) {
    let include = query.included_tags.contains(&tag_name);
    let exclude = query.excluded_tags.contains(&tag_name);
    let mixed = !include
        && !exclude
        && itertools::chain(&query.included_tags, &query.excluded_tags)
            .any(|tag| tag.starts_with(tag_name));

    let mut coherent_state = None;
    let state = if mixed || (include && exclude) {
        FilterCheckboxState::Mixed
    } else if include {
        coherent_state = Some(true);
        FilterCheckboxState::Coherent(&mut coherent_state)
    } else if exclude {
        coherent_state = Some(false);
        FilterCheckboxState::Coherent(&mut coherent_state)
    } else {
        coherent_state = None;
        FilterCheckboxState::Coherent(&mut coherent_state)
    };

    let r = ui.add(FilterCheckbox::new(
        crate::gui::components::FilterCheckboxAllowedStates::NeutralShowHide,
        state,
        label,
    ));
    if r.changed() {
        // TODO: remove child tags

        let span = query.tag_spans.iter().copied().find(
            |&(_symbol_start, tag_name_start, tag_name_end)| {
                &query_string[tag_name_start..tag_name_end] == tag_name
            },
        );
        let replacement = match coherent_state {
            None => String::new(),
            Some(false) => format!("~#{tag_name}"),
            Some(true) => format!("#{tag_name}"),
        };
        if let Some((symbol_start, _tag_name_start, tag_name_end)) = span {
            query_string.replace_range(symbol_start..tag_name_end, &replacement);
        } else {
            *query_string = query_string.trim_end().to_owned();
            query_string.push(' ');
            query_string.push_str(&replacement);
            query_string.push(' ');
        }
    }

    r.on_hover_ui(|ui| {
        md(
            ui,
            "TODO accurate info on hover\n\n**15** puzzles matching current filter\n\n**30** puzzles total",
        );
    });
}

fn tag_checkbox_menu(
    ui: &mut egui::Ui,
    s: &str,
    tag_name: &str,
    query: &Query<'_>,
    query_string: &mut String,
    contents: impl FnOnce(&mut egui::Ui, &mut String),
) {
    ui.horizontal(|ui| {
        tag_checkbox(ui, "", tag_name, query, query_string);
        ui.add_space(-6.0);
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.menu_button(s, |ui| contents(ui, query_string))
        });
    });
}
fn tag_submenu(ui: &mut egui::Ui, s: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.menu_button(s, contents);
}

#[derive(Debug, Default, Clone, PartialEq)]
struct PuzzleGeneratorPopupData {
    id: String,
    params: Vec<GeneratorParamValue>,
}
impl PuzzleGeneratorPopupData {
    fn new(puzzle_generator: &PuzzleGeneratorSpec) -> Self {
        Self {
            id: puzzle_generator.id.clone(),
            params: puzzle_generator
                .params
                .iter()
                .map(|param| param.default.clone())
                .collect(),
        }
    }
}

trait TagSearchable {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn aliases(&self) -> &[String];
    fn tags(&self) -> &HashMap<String, TagValue>;
}
impl TagSearchable for PuzzleSpec {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        self.display_name()
    }
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
    fn tags(&self) -> &HashMap<String, TagValue> {
        &self.tags
    }
}
impl TagSearchable for PuzzleGeneratorSpec {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        self.display_name()
    }
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
    fn tags(&self) -> &HashMap<String, TagValue> {
        &self.tags
    }
}

struct Query<'a> {
    /// Combined text portion of the query string.
    text: String,
    /// Tags included in the search.
    included_tags: Vec<&'a str>,
    /// Tags excluded from the search.
    excluded_tags: Vec<&'a str>,

    /// Spans of tags in the original query. The three numbers are:
    /// - the index of the start of the tag symbol (e.g., the start of `~#cube`)
    /// - the index of the start of the tag name (e.g., the start of `cube`)
    /// - the index of the end of the tag name
    tag_spans: Vec<(usize, usize, usize)>,
}
impl<'a> Query<'a> {
    fn from_str(s: &'a str) -> Self {
        let mut text = String::new();
        let mut included_tags = vec![];
        let mut excluded_tags = vec![];
        let mut tag_spans = vec![];
        let tag_span = |word: &str, tag_name: &str| {
            let base1 = word.as_ptr() as usize - s.as_ptr() as usize;
            let base2 = tag_name.as_ptr() as usize - s.as_ptr() as usize;
            (base1, base2, base2 + tag_name.len())
        };
        for word in s.split_whitespace() {
            if let Some(tag_name) = word.strip_prefix("!#") {
                excluded_tags.push(tag_name);
                tag_spans.push(tag_span(word, tag_name));
            } else if let Some(tag_name) = word.strip_prefix("~#") {
                excluded_tags.push(tag_name);
                tag_spans.push(tag_span(word, tag_name));
            } else if let Some(tag_name) = word.strip_prefix('#') {
                included_tags.push(tag_name);
                tag_spans.push(tag_span(word, tag_name));
            } else {
                text += word;
            }
        }
        Query {
            text,
            included_tags,
            excluded_tags,
            tag_spans,
        }
    }

    fn try_match<T: TagSearchable>(&self, object: &Arc<T>) -> Option<QueryMatch<T>> {
        let mut include = self.included_tags.iter();
        let mut exclude = self.excluded_tags.iter();
        if !(include.all(|&tag| object.tags().get(tag).is_some_and(|v| v.is_present()))
            && exclude.all(|&tag| !object.tags().get(tag).is_some_and(|v| v.is_present())))
        {
            return None;
        }

        if self.text.is_empty() {
            return Some(QueryMatch {
                object: Arc::clone(object),
                name_match: None,
                additional_match: None,
                score: 0,
            });
        }

        let name_match = sublime_fuzzy::best_match(&self.text, object.name());

        let additional_match = itertools::chain!(
            [("ID", object.id(), ID_MATCH_PENALTY)], // TODO: localize this
            object
                .aliases()
                .iter()
                .map(|alias| ("Alias", alias.as_str(), ALIAS_MATCH_PENALTY))
        )
        .filter_map(|(property_name, property_text, penalty)| {
            let match_info = sublime_fuzzy::best_match(&self.text, &property_text)?;
            Some(AdditionalQueryMatch {
                property_name: property_name.to_owned(),
                property_text: property_text.to_owned(),
                match_info,
                penalty,
            })
        })
        .max_by_key(|additional_match| additional_match.match_info.score())
        .filter(|additional_match| match &name_match {
            Some(main_match) => additional_match.match_info.score() > main_match.score(),
            None => true,
        });

        let score = if let Some(m) = &name_match {
            m.score()
        } else if let Some(m) = &additional_match {
            m.match_info.score() - m.penalty
        } else {
            return None;
        };

        Some(QueryMatch {
            object: Arc::clone(object),
            name_match,
            additional_match,
            score,
        })
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty() && self.included_tags.is_empty() && self.excluded_tags.is_empty()
    }
}

struct QueryMatch<T> {
    /// Matched object.
    object: Arc<T>,
    /// Info about the fuzzy match for the display name, or `None` if the text
    /// portion of the query is empty.
    name_match: Option<sublime_fuzzy::Match>,
    /// Additional property that best matched the query, if better than the
    /// display name.
    additional_match: Option<AdditionalQueryMatch>,
    /// Total match score.
    score: isize,
}

struct AdditionalQueryMatch {
    /// Human-friendly name of the property that matched.
    property_name: String,
    /// Contents of the property.
    property_text: String,
    /// Fuzzy match info.
    match_info: sublime_fuzzy::Match,
    /// Penalty to apply on top of `match_info`. This number is typically
    /// positive, and should be subtracted from `match_info.score()`.
    penalty: isize,
}

impl<T: TagSearchable> egui::Widget for QueryMatch<T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let name = self.object.name();
        let mut job = egui::text::LayoutJob::default();

        if let Some(m) = &self.name_match {
            render_fuzzy_match(ui, &mut job, name, m, egui::TextStyle::Button);
        } else if self.additional_match.is_some() {
            job.append(
                name,
                0.0,
                egui::TextFormat::simple(
                    egui::TextStyle::Button.resolve(ui.style()),
                    ui.visuals().widgets.inactive.text_color(),
                ),
            );
        } else {
            job.append(
                name,
                0.0,
                egui::TextFormat::simple(
                    egui::TextStyle::Button.resolve(ui.style()),
                    ui.visuals().widgets.inactive.text_color(),
                ),
            );
        }

        if let Some(m) = self.additional_match {
            let text_format = egui::TextFormat::simple(
                egui::TextStyle::Small.resolve(ui.style()),
                ui.visuals().widgets.inactive.text_color(),
            );
            job.append(
                &format!("\n{ADDITIONAL_MATCH_INDENT}{}: ", m.property_name),
                0.0,
                text_format,
            );
            render_fuzzy_match(
                ui,
                &mut job,
                &m.property_text,
                &m.match_info,
                egui::TextStyle::Small,
            );
        }

        ui.selectable_label(false, job)
    }
}

fn render_fuzzy_match(
    ui: &mut egui::Ui,
    job: &mut egui::text::LayoutJob,
    s: &str,
    match_info: &sublime_fuzzy::Match,
    text_style: egui::TextStyle,
) {
    let font_id = text_style.resolve(ui.style());
    let basic_text_format = egui::TextFormat::simple(font_id.clone(), ui.visuals().text_color());
    let match_text_format = egui::TextFormat::simple(font_id, ui.visuals().strong_text_color());

    let mut i = 0;
    for c in match_info.continuous_matches() {
        job.append(&s[i..c.start()], 0.0, basic_text_format.clone());
        job.append(
            &s[c.start()..c.start() + c.len()],
            0.0,
            match_text_format.clone(),
        );
        i = c.start() + c.len();
    }
    job.append(&s[i..], 0.0, basic_text_format);
}

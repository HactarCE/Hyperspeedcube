use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use egui::AtomExt;
use egui::containers::menu::{MenuButton, MenuConfig};
use egui::emath::GuiRounding;
use hyperpuzzle::prelude::*;
use itertools::Itertools;
use regex::Regex;

use crate::L;
use crate::app::App;
use crate::gui::components::{
    BIG_ICON_BUTTON_SIZE, escape_tag_value, format_tag_and_value, unescape_tag_value,
};
use crate::gui::markdown::{md, md_escape};
use crate::gui::util::EguiTempValue;

pub const ID_MATCH_PENALTY: isize = 60;
pub const ALIAS_MATCH_PENALTY: isize = 50;
pub const ADDITIONAL_MATCH_INDENT: &str = "    ";

const GENERATOR_SLIDER_WIDTH: f32 = 200.0;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let stored_search_query_string = EguiTempValue::new(ui);
    let mut search_query_string: String = stored_search_query_string.get().unwrap_or_default();

    let catalog = hyperpuzzle::catalog();

    ui.group(|ui| {
        ui.horizontal(|ui| {
            if hyperpaths::hps_dir().is_ok() {
                let r = ui.add(egui::Button::new("🔃").min_size(BIG_ICON_BUTTON_SIZE));
                // TODO: global F5 keybind
                if r.on_hover_text(L.catalog.refresh).clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::F5))
                {
                    hyperpuzzle::load_global_catalog();
                }
            }

            let mut tag_action = None;
            ui.scope(|ui| {
                ui.set_max_size(BIG_ICON_BUTTON_SIZE);
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center)
                        .with_cross_justify(true)
                        .with_main_justify(true),
                    |ui| {
                        let query = Query::from_str(&search_query_string);

                        let (r, _) = MenuButton::new("🏷")
                            .config(
                                MenuConfig::default()
                                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                            )
                            .ui(ui, |ui| {
                                tag_action = crate::gui::components::TagMenu::new(
                                    query.included_tags.iter().map(format_tag_and_value),
                                    query.excluded_tags.iter().map(format_tag_and_value),
                                )
                                .with_experimental(app.prefs.show_experimental_puzzles)
                                .show(ui)
                                .inner;
                            });
                        r.on_hover_text(L.catalog.filter_by_tag);
                    },
                )
            });

            if let Some(action) = tag_action {
                let mut found = false;

                let mut new_search_query_string = String::new();
                for segment in Query::from_str(&search_query_string).segments {
                    if let QuerySegment::Tag {
                        tag_name, value, ..
                    } = segment
                    {
                        if tag_name == action.tag
                            && value.map(Cow::Borrowed)
                                == action.value.as_deref().map(|s| escape_tag_value(s))
                        {
                            if found {
                                // delete segment because it's redundant
                                continue;
                            } else {
                                // replace segment
                                found = true;
                                if let Some(segment_string) = action.segment_string() {
                                    new_search_query_string += &segment_string;
                                }
                                continue;
                            }
                        } else if tag_name.starts_with(&action.tag)
                            && action.value.is_none()
                            && action.new_state.is_none()
                        {
                            // delete segment to remove subtag filter
                            continue;
                        } else {
                            new_search_query_string += &segment.to_string();
                        }
                    } else {
                        new_search_query_string += &segment.to_string();
                    }
                }

                if !found {
                    if let Some(segment_string) = action.segment_string() {
                        if !new_search_query_string.is_empty()
                            && !new_search_query_string.ends_with(" ")
                        {
                            new_search_query_string += " ";
                        }
                        new_search_query_string += &segment_string;
                        new_search_query_string += " ";
                    }
                }

                search_query_string = new_search_query_string.trim().to_owned();
            }

            ui.add(
                egui::TextEdit::singleline(&mut search_query_string)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut Query::text_layouter),
            );
        });

        ui.add_space(ui.spacing().item_spacing.y);
        ui.separator();
        ui.add_space(ui.spacing().item_spacing.y);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let max_size = ui.max_rect().size().floor(); // needed to avoid "unaligned" visual warnings
                let layout = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.allocate_ui_with_layout(max_size, layout, |ui| {
                    let mut query = Query::from_str(&search_query_string);

                    if let Some(incomplete_tag) = query
                        .segments
                        .iter_mut()
                        .find_map(QuerySegment::as_incomplete_tag_mut)
                    {
                        // Show tag search
                        let search_query = *incomplete_tag;
                        let mut changed = false;
                        for query_result in hyperpuzzle::TAGS
                            .all_tags()
                            .iter()
                            .filter_map(|tag| SubstringQueryMatch::try_from(search_query, tag))
                        {
                            let tag = query_result.string;
                            if ui.add(query_result).clicked() {
                                *incomplete_tag = tag;
                                changed = true;
                            }
                        }
                        if changed {
                            search_query_string = query.to_string();
                        }
                    } else {
                        // Show puzzles & generators search

                        let generator_popup_data_stored =
                            EguiTempValue::<PuzzleGeneratorPopupData>::new(ui);
                        let mut generator_popup_data = generator_popup_data_stored.get();

                        let show_experimental = app.prefs.show_experimental_puzzles
                            || query
                                .included_tags
                                .iter()
                                .any(|(tag, _value)| *tag == "experimental");

                        let puzzle_list_entries = catalog.puzzle_list_entries();
                        let query_results = puzzle_list_entries
                            .iter()
                            .filter(|entry| show_experimental || !entry.tags.is_experimental())
                            .filter_map(|entry| query.try_match(entry))
                            .sorted_unstable();

                        for query_result in query_results {
                            let obj = query_result.object.clone();
                            let r = ui.add(query_result);
                            match catalog.get_generator::<Puzzle>(&obj.id) {
                                None => {
                                    if r.clicked() {
                                        app.load_puzzle(&obj.id);
                                    }
                                }

                                Some(puzzle_generator) => {
                                    let popup_id = generator_popup_data_stored.id.with(&obj.id);

                                    if r.clicked() {
                                        generator_popup_data =
                                            Some(PuzzleGeneratorPopupData::new(&puzzle_generator));
                                    }

                                    if let Some(popup_data) = generator_popup_data
                                        .as_mut()
                                        .filter(|data| data.id == obj.id)
                                    {
                                        egui::Popup::from_toggle_button_response(&r)
                                            .close_behavior(
                                                egui::PopupCloseBehavior::CloseOnClickOutside,
                                            )
                                            .show(|ui| {
                                                show_puzzle_generator_ui(
                                                    ui,
                                                    popup_id,
                                                    app,
                                                    &puzzle_generator,
                                                    popup_data,
                                                );
                                            });
                                    }
                                }
                            }
                        }

                        generator_popup_data_stored.set(generator_popup_data);
                    }
                });
            });
    });

    stored_search_query_string.set(Some(search_query_string.clone()));
}

#[derive(Debug, Default, Clone, PartialEq)]
struct PuzzleGeneratorPopupData {
    id: String,
    params: Vec<GeneratorParamValue>,
}
impl PuzzleGeneratorPopupData {
    fn new(puzzle_generator: &PuzzleSpecGenerator) -> Self {
        Self {
            id: puzzle_generator.meta.id.clone(),
            params: puzzle_generator
                .params
                .iter()
                .map(|param| param.default.clone())
                .collect(),
        }
    }
}

fn show_puzzle_generator_ui(
    ui: &mut egui::Ui,
    popup_id: egui::Id,
    app: &mut App,
    puzzle_generator: &PuzzleSpecGenerator,
    popup_data: &mut PuzzleGeneratorPopupData,
) {
    ui.strong(&puzzle_generator.meta.name);

    for (param, value) in std::iter::zip(&puzzle_generator.params, &mut popup_data.params) {
        match param.ty {
            GeneratorParamType::Int { min, max } => {
                let GeneratorParamValue::Int(i) = value;
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = GENERATOR_SLIDER_WIDTH;
                    ui.add(egui::Slider::new(i, min..=max).logarithmic(true));
                    ui.label(&param.name);
                });
            }
        }
    }

    if ui.button(L.catalog.generate_puzzle).clicked() {
        ui.close();
        let puzzle_id = hyperpuzzle::generated_id(&puzzle_generator.meta.id, &popup_data.params);
        app.load_puzzle(&puzzle_id);
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum QuerySegment<'a> {
    Whitespace(&'a str),
    Word(&'a str),
    Tag {
        prefix: &'a str,
        tag_name: &'a str,
        value: Option<&'a str>,
    },
}
impl fmt::Display for QuerySegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuerySegment::Whitespace(s) | QuerySegment::Word(s) => write!(f, "{s}"),
            QuerySegment::Tag {
                prefix,
                tag_name,
                value,
            } => {
                write!(f, "{prefix}#{tag_name}")?;
                if let Some(value) = value {
                    write!(f, "={value}")?;
                }
                Ok(())
            }
        }
    }
}
impl<'a> QuerySegment<'a> {
    fn as_incomplete_tag_mut(&mut self) -> Option<&'_ mut &'a str> {
        match self {
            QuerySegment::Whitespace(_) | QuerySegment::Word(_) => None,
            QuerySegment::Tag { tag_name, .. } => {
                hyperpuzzle::TAGS.get(tag_name).is_err().then_some(tag_name)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<'a> {
    /// Parsed segments of the query string.
    segments: Vec<QuerySegment<'a>>,

    /// Combined text portion of the query string.
    text: String,
    /// Tags (and optional values) included in the search.
    included_tags: Vec<(&'a str, Option<String>)>,
    /// Tags (and optional values) excluded from the search.
    excluded_tags: Vec<(&'a str, Option<String>)>,
}
impl fmt::Display for Query<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            write!(f, "{segment}")?;
        }
        Ok(())
    }
}
impl<'a> Query<'a> {
    pub fn from_str(s: &'a str) -> Self {
        lazy_static! {
            static ref SEGMENT_REGEX: Regex =
                Regex::new(r#"\s+|([^"\s]|"([^"]*|\\")"?)+"#).expect("bad regex");
        }

        let mut segments = vec![];
        let mut included_tags = vec![];
        let mut excluded_tags = vec![];
        for regex_match in SEGMENT_REGEX.find_iter(s) {
            let segment_str = regex_match.as_str();
            let segment = if segment_str
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace())
            {
                QuerySegment::Whitespace(segment_str)
            } else if let Some((prefix, mut tag_name)) = segment_str.split_once('#') {
                let mut value = None;
                let mut unescaped_value = None;
                if let Some((left, right)) = tag_name.split_once('=') {
                    tag_name = left;
                    value = Some(right);
                    unescaped_value = unescape_tag_value(right);
                }
                match prefix {
                    "" => {
                        included_tags.push((tag_name, unescaped_value));
                        QuerySegment::Tag {
                            prefix,
                            tag_name,
                            value,
                        }
                    }
                    "!" | "~" => {
                        excluded_tags.push((tag_name, unescaped_value));
                        QuerySegment::Tag {
                            prefix,
                            tag_name,
                            value,
                        }
                    }
                    _ => QuerySegment::Word(segment_str),
                }
            } else {
                QuerySegment::Word(segment_str)
            };
            segments.push(segment);
        }

        let text = segments
            .iter()
            .filter_map(|segment| match segment {
                QuerySegment::Word(word) => Some(word),
                _ => None,
            })
            .join(" ");

        Query {
            segments,

            text,
            included_tags,
            excluded_tags,
        }
    }

    fn text_layouter(
        ui: &egui::Ui,
        buffer: &dyn egui::TextBuffer,
        wrap_width: f32,
    ) -> Arc<egui::Galley> {
        let text_font_id = egui::TextStyle::Body.resolve(ui.style());
        let tag_font_id = egui::TextStyle::Monospace.resolve(ui.style());

        let basic_text_format =
            egui::TextFormat::simple(text_font_id, ui.visuals().widgets.inactive.text_color());

        let symbol_text_color = match ui.visuals().dark_mode {
            true => egui::Color32::LIGHT_BLUE,
            false => egui::Color32::DARK_BLUE,
        };
        let symbol_text_format = egui::TextFormat::simple(tag_font_id.clone(), symbol_text_color);

        let value_text_color = match ui.visuals().dark_mode {
            true => egui::Color32::YELLOW,
            false => egui::Color32::DARK_GREEN,
        };
        let value_text_format = egui::TextFormat::simple(tag_font_id.clone(), value_text_color);

        let error_text_color = match ui.visuals().dark_mode {
            true => egui::Color32::LIGHT_RED,
            false => egui::Color32::DARK_RED,
        };
        let error_text_format = egui::TextFormat::simple(tag_font_id, error_text_color);

        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;

        let query = Query::from_str(buffer.as_str());
        for segment in &query.segments {
            match segment {
                QuerySegment::Whitespace(s) | QuerySegment::Word(s) => {
                    append_to_job(&mut job, &segment.to_string(), basic_text_format.clone());
                }
                QuerySegment::Tag {
                    prefix,
                    tag_name,
                    value,
                } => {
                    let tag_name_text_format = match hyperpuzzle::TAGS.get(tag_name) {
                        Ok(_) => &symbol_text_format,
                        Err(_) => &error_text_format,
                    };
                    append_to_job(&mut job, prefix, value_text_format.clone());
                    append_to_job(&mut job, "#", tag_name_text_format.clone());
                    append_to_job(&mut job, tag_name, tag_name_text_format.clone());
                    if let Some(value) = value {
                        let value_text_format =
                            match value.starts_with('"') && !value.ends_with('"') {
                                true => &error_text_format,
                                false => &value_text_format,
                            };
                        append_to_job(&mut job, "=", value_text_format.clone());
                        append_to_job(&mut job, value, value_text_format.clone());
                    }
                }
            }
        }

        ui.fonts(|fonts| fonts.layout_job(job))
    }

    pub fn try_match<'b>(&self, object: &'b PuzzleListMetadata) -> Option<FuzzyQueryMatch<'b>> {
        let tags = &object.tags;
        let mut include = self.included_tags.iter();
        let mut exclude = self.excluded_tags.iter();
        if !(include.all(|(tag, value)| tags.meets_search_criterion(tag, value.as_deref()))
            && exclude.all(|(tag, value)| !tags.meets_search_criterion(tag, value.as_deref())))
        {
            return None;
        }

        if self.text.is_empty() {
            return Some(FuzzyQueryMatch {
                object,
                name_match: None,
                additional_match: None,
                score: 0,
            });
        }

        let name_match = sublime_fuzzy::best_match(&self.text, &object.name);

        let additional_match = itertools::chain(
            [("ID", &object.id, ID_MATCH_PENALTY)], // TODO: localize this
            object
                .aliases
                .iter()
                .map(|alias| ("Alias", alias, ALIAS_MATCH_PENALTY)),
        )
        .filter_map(|(property_name, property_text, penalty)| {
            let match_info = sublime_fuzzy::best_match(&self.text, property_text)?;
            Some(AdditionalFuzzyQueryMatch {
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

        Some(FuzzyQueryMatch {
            object,
            name_match,
            additional_match,
            score,
        })
    }

    /// Returns whether the search query is totally empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.included_tags.is_empty() && self.excluded_tags.is_empty()
    }
}

struct SubstringQueryMatch<'a> {
    /// Matched string.
    string: &'a str,
    /// Matched substring range.
    range: Range<usize>,
}
impl<'a> SubstringQueryMatch<'a> {
    fn try_from(search_text: &str, string: &'a str) -> Option<Self> {
        let start = string.find(search_text)?;
        let end = start + search_text.len();
        Some(Self {
            string,
            range: start..end,
        })
    }
}
impl egui::Widget for SubstringQueryMatch<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
        let unmatch_text_format = unmatch_text_format(ui, font_id.clone());
        let match_text_format = match_text_format(ui, font_id);
        let mut job = egui::text::LayoutJob::default();
        let start = self.range.start;
        let end = self.range.end;
        append_to_job(&mut job, "#", unmatch_text_format.clone());
        append_to_job(&mut job, &self.string[..start], unmatch_text_format.clone());
        append_to_job(&mut job, &self.string[self.range], match_text_format);
        append_to_job(&mut job, &self.string[end..], unmatch_text_format);
        ui.selectable_label(false, job)
    }
}

pub struct FuzzyQueryMatch<'a> {
    /// Matched object.
    pub object: &'a PuzzleListMetadata,
    /// Info about the fuzzy match for the display name, or `None` if the text
    /// portion of the query is empty.
    name_match: Option<sublime_fuzzy::Match>,
    /// Additional property that best matched the query, if better than the
    /// display name.
    additional_match: Option<AdditionalFuzzyQueryMatch>,
    /// Total match score.
    score: isize,
}
impl PartialEq for FuzzyQueryMatch<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}
impl Eq for FuzzyQueryMatch<'_> {}
impl PartialOrd for FuzzyQueryMatch<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FuzzyQueryMatch<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&(-self.score, &self.object), &(-other.score, &other.object))
    }
}

struct AdditionalFuzzyQueryMatch {
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

impl egui::Widget for FuzzyQueryMatch<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let name = &self.object.name;
        let mut job = egui::text::LayoutJob::default();

        if let Some(m) = &self.name_match {
            render_fuzzy_match(ui, &mut job, name, m, egui::TextStyle::Button);
        } else {
            let normal_text_format = egui::TextFormat::simple(
                egui::TextStyle::Button.resolve(ui.style()),
                ui.visuals().widgets.inactive.text_color(),
            );
            append_to_job(&mut job, name, normal_text_format);
        }

        if let Some(m) = self.additional_match {
            let text_format = egui::TextFormat::simple(
                egui::TextStyle::Small.resolve(ui.style()),
                ui.visuals().widgets.inactive.text_color(),
            );
            append_to_job(
                &mut job,
                &format!("\n{ADDITIONAL_MATCH_INDENT}{}: ", m.property_name),
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

        let mut atoms = egui::Atoms::new((job, egui::Atom::grow()));
        let icons = crate::gui::icons::CatalogIcon::icons_from_tags(&self.object.tags);
        for icon in &icons {
            icon.add_to(ui, &mut atoms);
        }

        ui.selectable_label(false, atoms).on_hover_ui(|ui| {
            ui.strong(&self.object.name);

            fn comma_list(strings: &[String]) -> String {
                md_escape(&strings.iter().join(", ")).into_owned()
            }

            let inventors = self.object.tags.inventors();
            if !inventors.is_empty() {
                md(ui, format!("**Inventors:** {}", comma_list(inventors)));
            }

            let authors = self.object.tags.authors();
            if !authors.is_empty() {
                md(ui, format!("**Authors:** {}", comma_list(authors)));
            }

            let aliases = &self.object.aliases;
            if !aliases.is_empty() {
                md(ui, format!("**Aliases:** {}", comma_list(aliases)));
            }

            if let Some(url) = self.object.tags.wca_url() {
                ui.hyperlink_to("WCA leaderboards", url);
            }

            ui.separator();

            for icon in icons {
                // TODO: when ui.label() supports atoms, use that here instead
                ui.horizontal(|ui| {
                    ui.add(icon.to_image(ui));
                    ui.label(icon.description);
                });
            }
        })
    }
}

fn render_fuzzy_match(
    ui: &egui::Ui,
    job: &mut egui::text::LayoutJob,
    s: &str,
    match_info: &sublime_fuzzy::Match,
    text_style: egui::TextStyle,
) {
    let font_id = text_style.resolve(ui.style());
    let unmatch_text_format = unmatch_text_format(ui, font_id.clone());
    let match_text_format = match_text_format(ui, font_id);

    let mut i = 0;
    for c in match_info.continuous_matches() {
        append_to_job(job, &s[i..c.start()], unmatch_text_format.clone());
        append_to_job(
            job,
            &s[c.start()..c.start() + c.len()],
            match_text_format.clone(),
        );
        i = c.start() + c.len();
    }
    append_to_job(job, &s[i..], unmatch_text_format);
}

fn unmatch_text_format(ui: &egui::Ui, font_id: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font_id.clone(), ui.visuals().text_color())
}
fn match_text_format(ui: &egui::Ui, font_id: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font_id.clone(), ui.visuals().strong_text_color())
}

fn append_to_job(job: &mut egui::text::LayoutJob, s: &str, format: egui::TextFormat) {
    // Workaround for https://github.com/emilk/egui/issues/7378
    if !s.is_empty() {
        job.append(s, 0.0, format);
    }
}

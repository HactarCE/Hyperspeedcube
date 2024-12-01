use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use hyperpuzzle::lua::{GeneratorParamType, GeneratorParamValue, PuzzleGeneratorSpec, PuzzleSpec};
use hyperpuzzle::TagSet;
use itertools::Itertools;
use regex::Regex;

use crate::app::App;
use crate::gui::components::{
    escape_tag_value, format_tag_and_value, unescape_tag_value, BIG_ICON_BUTTON_SIZE,
};
use crate::gui::markdown::{md, md_escape};
use crate::gui::util::EguiTempValue;
use crate::L;

pub const ID_MATCH_PENALTY: isize = 60;
pub const ALIAS_MATCH_PENALTY: isize = 50;
pub const ADDITIONAL_MATCH_INDENT: &str = "    ";

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let stored_search_query_string = EguiTempValue::new(ui);
    let mut search_query_string: String = stored_search_query_string.get().unwrap_or_default();

    ui.group(|ui| {
        ui.horizontal(|ui| {
            if hyperprefs::paths::lua_dir().is_ok() {
                let r = ui.add(egui::Button::new("🔃").min_size(BIG_ICON_BUTTON_SIZE));
                // TODO: global F5 keybind
                if r.on_hover_text(L.library.refresh).clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::F5))
                {
                    crate::load_user_puzzles();
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

                        let r = ui.menu_button("🏷", |ui| {
                            tag_action = crate::gui::components::TagMenu::new(
                                query.included_tags.iter().map(format_tag_and_value),
                                query.excluded_tags.iter().map(format_tag_and_value),
                            )
                            .with_experimental(app.prefs.show_experimental_puzzles)
                            .show(ui)
                            .inner;
                        });
                        r.response.on_hover_text(L.library.filter_by_tag);
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
                        if tag_name == &action.tag
                            && value.as_deref().map(Cow::Borrowed)
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
                            && action.new_state == None
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
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
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
                            .filter_map(|tag| SubstringQueryMatch::try_from(search_query, &tag))
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

                        let query_results = crate::LIBRARY
                            .with(|lib| {
                                itertools::chain(
                                    lib.puzzles().into_iter().map(ListEntry::Puzzle),
                                    lib.puzzle_generators()
                                        .into_iter()
                                        .map(ListEntry::PuzzleGenerator),
                                )
                            })
                            .filter(|entry| show_experimental || !entry.tags().is_experimental())
                            .filter_map(|entry| query.try_match(entry))
                            .sorted_unstable_by(|a, b| {
                                Ord::cmp(&(-a.score, a.object.id()), &(-b.score, b.object.id()))
                            });

                        for query_result in query_results {
                            let obj = query_result.object.clone();
                            let r = ui.add(query_result);
                            match obj {
                                ListEntry::Puzzle(puzzle) => {
                                    if r.clicked() {
                                        crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle.id));
                                    }
                                }

                                ListEntry::PuzzleGenerator(puzzle_generator) => {
                                    let popup_id =
                                        generator_popup_data_stored.id.with(&puzzle_generator.id);

                                    if r.clicked() {
                                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                        generator_popup_data =
                                            Some(PuzzleGeneratorPopupData::new(&puzzle_generator));
                                    }

                                    let close_behavior =
                                        egui::PopupCloseBehavior::CloseOnClickOutside;
                                    if let Some(popup_data) = generator_popup_data
                                        .as_mut()
                                        .filter(|data| data.id == puzzle_generator.id)
                                    {
                                        egui::popup_below_widget(
                                            ui,
                                            popup_id,
                                            &r,
                                            close_behavior,
                                            |ui| {
                                                show_puzzle_generator_ui(
                                                    ui,
                                                    app,
                                                    &puzzle_generator,
                                                    popup_data,
                                                )
                                            },
                                        );
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

fn show_puzzle_generator_ui(
    ui: &mut egui::Ui,
    app: &mut App,
    puzzle_generator: &PuzzleGeneratorSpec,
    popup_data: &mut PuzzleGeneratorPopupData,
) {
    ui.strong(puzzle_generator.display_name());

    for (param, value) in std::iter::zip(&puzzle_generator.params, &mut popup_data.params) {
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

    if ui.button(L.library.generate_puzzle).clicked() {
        ui.memory_mut(|mem| mem.close_popup());
        let puzzle_id = hyperpuzzle::generated_puzzle_id(&puzzle_generator.id, &popup_data.params);
        crate::LIBRARY.with(|lib| app.load_puzzle(lib, &puzzle_id));
    };
}

#[derive(Debug, Clone)]
pub enum ListEntry {
    Puzzle(Arc<PuzzleSpec>),
    PuzzleGenerator(Arc<PuzzleGeneratorSpec>),
}
impl ListEntry {
    fn id(&self) -> &str {
        match self {
            ListEntry::Puzzle(p) => &p.id,
            ListEntry::PuzzleGenerator(g) => &g.id,
        }
    }
    fn name(&self) -> &str {
        match self {
            ListEntry::Puzzle(p) => &p.display_name(),
            ListEntry::PuzzleGenerator(g) => &g.display_name(),
        }
    }
    fn aliases(&self) -> &[String] {
        match self {
            ListEntry::Puzzle(p) => &p.aliases,
            ListEntry::PuzzleGenerator(g) => &g.aliases,
        }
    }
    fn tags(&self) -> &TagSet {
        match self {
            ListEntry::Puzzle(p) => &p.tags,
            ListEntry::PuzzleGenerator(g) => &g.tags,
        }
    }
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
struct Query<'a> {
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
    fn from_str(s: &'a str) -> Self {
        lazy_static! {
            static ref SEGMENT_REGEX: Regex =
                Regex::new(r#"\s+|([^"\s]|"([^"]*|\\")"?)+"#).unwrap();
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

    fn text_layouter(ui: &egui::Ui, string: &str, wrap_width: f32) -> Arc<egui::Galley> {
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

        let query = Query::from_str(string);
        for segment in &query.segments {
            match segment {
                QuerySegment::Whitespace(_) | QuerySegment::Word(_) => {
                    job.append(&segment.to_string(), 0.0, basic_text_format.clone());
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
                    job.append(prefix, 0.0, value_text_format.clone());
                    job.append("#", 0.0, tag_name_text_format.clone());
                    job.append(tag_name, 0.0, tag_name_text_format.clone());
                    if let Some(value) = value {
                        let value_text_format =
                            match value.starts_with('"') && !value.ends_with('"') {
                                true => &error_text_format,
                                false => &value_text_format,
                            };
                        job.append("=", 0.0, value_text_format.clone());
                        job.append(*value, 0.0, value_text_format.clone());
                    }
                }
            }
        }

        ui.fonts(|fonts| fonts.layout_job(job))
    }

    fn try_match(&self, object: ListEntry) -> Option<FuzzyQueryMatch> {
        let tags = object.tags();
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
    fn is_empty(&self) -> bool {
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
        job.append("#", 0.0, unmatch_text_format.clone());
        job.append(&self.string[..start], 0.0, unmatch_text_format.clone());
        job.append(&self.string[self.range], 0.0, match_text_format);
        job.append(&self.string[end..], 0.0, unmatch_text_format);
        ui.selectable_label(false, job)
    }
}

struct FuzzyQueryMatch {
    /// Matched object.
    object: ListEntry,
    /// Info about the fuzzy match for the display name, or `None` if the text
    /// portion of the query is empty.
    name_match: Option<sublime_fuzzy::Match>,
    /// Additional property that best matched the query, if better than the
    /// display name.
    additional_match: Option<AdditionalFuzzyQueryMatch>,
    /// Total match score.
    score: isize,
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

impl egui::Widget for FuzzyQueryMatch {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let name = self.object.name();
        let mut job = egui::text::LayoutJob::default();

        if let Some(m) = &self.name_match {
            render_fuzzy_match(ui, &mut job, name, m, egui::TextStyle::Button);
        } else {
            let normal_text_format = egui::TextFormat::simple(
                egui::TextStyle::Button.resolve(ui.style()),
                ui.visuals().widgets.inactive.text_color(),
            );
            job.append(name, 0.0, normal_text_format);
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

        ui.selectable_label(false, job).on_hover_ui(|ui| {
            ui.strong(self.object.name());

            fn comma_list(strings: &[String]) -> String {
                md_escape(&strings.iter().join(", ")).into_owned()
            }

            let inventors = self.object.tags().inventors();
            if !inventors.is_empty() {
                md(ui, &format!("**Inventors:** {}", comma_list(inventors)));
            }

            let authors = self.object.tags().authors();
            if !authors.is_empty() {
                md(ui, &format!("**Authors:** {}", comma_list(authors)));
            }

            let aliases = self.object.aliases();
            if !aliases.is_empty() {
                md(ui, &format!("**Aliases:** {}", comma_list(aliases)));
            }

            if let Some(url) = self.object.tags().wca_url() {
                ui.hyperlink_to("WCA leaderboards", url);
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
        job.append(&s[i..c.start()], 0.0, unmatch_text_format.clone());
        job.append(
            &s[c.start()..c.start() + c.len()],
            0.0,
            match_text_format.clone(),
        );
        i = c.start() + c.len();
    }
    job.append(&s[i..], 0.0, unmatch_text_format);
}

fn unmatch_text_format(ui: &egui::Ui, font_id: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font_id.clone(), ui.visuals().text_color())
}
fn match_text_format(ui: &egui::Ui, font_id: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font_id.clone(), ui.visuals().strong_text_color())
}

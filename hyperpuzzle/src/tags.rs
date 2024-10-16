use std::{collections::HashMap, sync::Arc};

use strum::EnumString;

lazy_static! {
    /// Library of known tags, defined in `tags.kdl`.
    pub static ref TAGS: TagLibrary = TagLibrary::load();
}

/// Library of known tags, defined in `tags.kdl`.
#[derive(Debug)]
pub struct TagLibrary {
    menu: TagMenuNode,
    list: Vec<Arc<str>>,
    tag_data: HashMap<Arc<str>, TagData>,
    expected: Vec<Vec<Arc<str>>>,
}
impl TagLibrary {
    /// Returns the data for a tag, or `None` if no such tag exists.
    pub fn get(&self, tag_name: &str) -> Option<TagData> {
        self.tag_data.get(tag_name).copied()
    }

    /// Returns a list of tags as they should be arranged in the menu.
    pub fn menu(&self) -> &TagMenuNode {
        &self.menu
    }

    /// Returns a list of all tag names, in menu order.
    pub fn all_tags(&self) -> &[Arc<str>] {
        &self.list
    }

    /// Returns a list of tag sets. Every puzzle is expected to have at least
    /// one tag from each set.
    pub fn expected_tag_sets(&self) -> &[Vec<Arc<str>>] {
        &self.expected
    }

    /// Returns the ancestors of a tag, not including the tag itself. For example,
    /// the ancestors of `"shape/3d/cube"` are `["shape/3d", "shape"]`.
    pub fn ancestors<'a>(&self, tag: &'a str) -> impl Iterator<Item = &'a str> {
        std::iter::successors(Some(tag), |s| Some(s.rsplit_once(&['/', '='])?.0)).skip(1)
    }

    fn load() -> Self {
        let top_level_menu_items = match include_str!("tags.kdl").parse() {
            Ok(kdl_document) => kdl_to_tag_menu_node(&kdl_document, ""),
            Err(e) => {
                eprintln!("{e:#?}");
                panic!("error parsing tags.kdl");
            }
        };
        let menu = TagMenuNode::Tag {
            name: None,
            ty: TagType::Bool,
            display: TagDisplay::Inline,
            subtags: top_level_menu_items,
            auto: false,
            expected: false,
            list: false,
        };

        // Recursively assemble the othe,r tag indexes from the menu
        // specification. Order is important for `tags_list`.
        let mut stack = vec![&menu];
        let mut tags_list = vec![];
        let mut tag_data_map = HashMap::new();
        let mut expected_tag_sets = vec![];
        while let Some(node) = stack.pop() {
            match node {
                TagMenuNode::Heading(_) | TagMenuNode::Separator => (),
                TagMenuNode::Tag {
                    name,
                    ty,
                    display: _,
                    subtags,
                    auto,
                    expected,
                    list: _,
                } => {
                    if let Some(name) = name {
                        tags_list.push(Arc::clone(name));

                        let data = TagData {
                            ty: *ty,
                            auto: *auto,
                        };
                        tag_data_map.insert(Arc::clone(name), data);

                        if *expected {
                            expected_tag_sets.push(vec![Arc::clone(name)]);
                        }
                    } else if *expected {
                        expected_tag_sets.push(
                            subtags
                                .iter()
                                .filter_map(|subtag| subtag.name())
                                .map(Arc::clone)
                                .collect(),
                        );
                    }

                    // Add subtags to the queue.
                    stack.extend(subtags.iter().rev());
                }
            }
        }

        Self {
            menu,
            list: tags_list,
            tag_data: tag_data_map,
            expected: expected_tag_sets,
        }
    }

    /// Returns the authors list, given a map of tag values.
    pub fn authors<'a>(&self, tag_values: &'a HashMap<String, TagValue>) -> &'a [String] {
        tag_values
            .get("author")
            .and_then(|v| v.as_str_list())
            .unwrap_or(&[])
    }
    /// Returns the inventors list, given a map of tag values.
    pub fn inventors<'a>(&self, tag_values: &'a HashMap<String, TagValue>) -> &'a [String] {
        tag_values
            .get("inventor")
            .and_then(|v| v.as_str_list())
            .unwrap_or(&[])
    }
    /// Returns the URL of the puzzle's WCA page, given a map of tag values.
    pub fn wca_url(&self, tag_values: &HashMap<String, TagValue>) -> Option<String> {
        Some(format!(
            "https://www.worldcubeassociation.org/results/rankings/{}/single",
            tag_values.get("external/wca")?.as_str()?,
        ))
    }
    /// Returns whether the tag set contains the "experimental" tag.
    pub fn is_experimental(&self, tag_values: &HashMap<String, TagValue>) -> bool {
        tag_values
            .get("experimental")
            .is_some_and(|v| v.is_present())
    }
}

/// How to display a node in the tag menu.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagDisplay {
    /// Display child tags inline.
    Inline,
    /// Display a submenu with this label.
    Submenu(Arc<str>),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum TagValue {
    /// Not present (allowed for any type)
    #[default]
    False,
    /// Present (allowed only for boolean)
    True,
    /// Present due to subtag
    Inherited,
    /// Integer
    Int(i64),
    /// String
    Str(String),
    /// List of strings
    StrList(Vec<String>),
    /// Puzzle ID
    Puzzle(String),
}
impl TagValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            TagValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_str_list(&self) -> Option<&[String]> {
        match self {
            TagValue::StrList(vec) => Some(vec),
            _ => None,
        }
    }
    pub fn to_strings(&self) -> &[String] {
        match self {
            TagValue::Str(s) => std::slice::from_ref(s),
            TagValue::StrList(vec) => &vec,
            TagValue::Puzzle(p) => std::slice::from_ref(p),
            _ => &[],
        }
    }
    pub fn as_str_list_mut(&mut self) -> Option<&mut Vec<String>> {
        match self {
            TagValue::StrList(vec) => Some(vec),
            _ => None,
        }
    }
    pub fn is_present(&self) -> bool {
        match self {
            TagValue::False => false,
            _ => true,
        }
    }
    pub fn is_present_with_value(&self, value_str: Option<&str>) -> bool {
        if let Some(value_str) = value_str {
            match self {
                TagValue::False => value_str.eq_ignore_ascii_case("false") || value_str == "0",
                TagValue::True | TagValue::Inherited => {
                    value_str.eq_ignore_ascii_case("true") || value_str == "1"
                }
                TagValue::Int(i) => value_str == i.to_string(),
                TagValue::Str(s) => value_str.eq_ignore_ascii_case(s),
                TagValue::StrList(vec) => vec.iter().any(|s2| value_str.eq_ignore_ascii_case(&s2)),
                TagValue::Puzzle(s) => value_str.eq_ignore_ascii_case(s),
            }
        } else {
            self.is_present()
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TagType {
    /// Boolean (present vs. not present)
    #[default]
    Bool,
    /// Integer
    Int,
    /// String
    Str,
    /// List of strings
    StrList,
    /// Puzzle ID
    Puzzle,
}
impl TagType {
    pub(crate) fn may_be_table(self) -> bool {
        match self {
            TagType::Bool | TagType::Int | TagType::Str | TagType::Puzzle => false,
            TagType::StrList => true,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TagData {
    /// Type of data to store for the tag value.
    pub ty: TagType,
    /// Whether the tag can only be added automatically.
    pub auto: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagMenuNode {
    Heading(Arc<str>),
    Separator,
    Tag {
        /// Tag name, if there is one.
        name: Option<Arc<str>>,
        /// Type of data for the tag.
        ty: TagType,
        /// Display mode for the tag or category.
        display: TagDisplay,
        /// Subtags inside this category.
        subtags: Vec<TagMenuNode>,

        /// Whether the value for this tag may only be autogenerated.
        auto: bool,
        /// Whether this tag is expected to be specified on all puzzles built
        /// into the program.
        expected: bool,
        /// Whether this tag should display a list of all possible values in the menu.
        list: bool,
    },
}
impl TagMenuNode {
    fn name(&self) -> Option<&Arc<str>> {
        match self {
            TagMenuNode::Heading(_) | TagMenuNode::Separator => None,
            TagMenuNode::Tag { name, .. } => name.as_ref(),
        }
    }
}

fn kdl_to_tag_menu_node(kdl: &kdl::KdlDocument, prefix: &str) -> Vec<TagMenuNode> {
    kdl.nodes()
        .iter()
        .map(|node| {
            let name = node.name().value();
            match name {
                "__heading" => TagMenuNode::Heading(Arc::from(
                    node.entries()[0]
                        .value()
                        .as_string()
                        .expect("expected string value"),
                )),
                "__separator" => TagMenuNode::Separator,
                s if s.starts_with("__") => panic!("unknown special tag node {s:?}"),
                s => {
                    let name = format!("{prefix}{}", s.strip_prefix('_').unwrap_or(s));

                    let tag_type = node
                        .ty()
                        .map(|type_name| {
                            type_name
                                .value()
                                .parse()
                                .unwrap_or_else(|_| panic!("bad tag type: {type_name:?}"))
                        })
                        .unwrap_or_default();

                    let mut display = None;
                    let mut auto = false;
                    let mut expected = false;
                    let mut inline = false;
                    let mut list = false;
                    let mut include_in_tag = true; // default `true`!
                    for entry in node.entries() {
                        match entry.name().map(|ident| ident.value()) {
                            None => {
                                display = Some(Arc::from(
                                    entry.value().as_string().expect("expected string value"),
                                ));
                            }
                            Some("auto") => {
                                auto = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("expected") => {
                                expected = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("inline") => {
                                inline = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("list") => {
                                list = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("include_in_tag") => {
                                include_in_tag =
                                    entry.value().as_bool().expect("expected bool value");
                            }
                            Some(k) => panic!("unknown tag property key {k:?}"),
                        }
                    }

                    let children = match node.children() {
                        Some(children) => {
                            if include_in_tag && !name.is_empty() {
                                kdl_to_tag_menu_node(children, &format!("{name}/"))
                            } else {
                                kdl_to_tag_menu_node(children, prefix)
                            }
                        }
                        None => vec![],
                    };

                    let display =
                        match inline {
                            true => TagDisplay::Inline,
                            false => TagDisplay::Submenu(display.unwrap_or_else(|| {
                                panic!("tag {name:?} is missing display string")
                            })),
                        };

                    TagMenuNode::Tag {
                        name: include_in_tag.then(|| Arc::from(name)),
                        display,
                        ty: tag_type,
                        subtags: children,

                        auto,
                        expected,
                        list,
                    }
                }
            }
        })
        .collect()
}

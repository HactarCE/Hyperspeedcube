use std::collections::HashMap;

use strum::EnumString;

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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TagData {
    pub ty: TagType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagMenuNode {
    Heading(&'static str),
    Separator,
    Tag {
        /// Tag name, if there is one.
        name: Option<&'static str>,
        /// Display name for the tag or category.
        display: &'static str,
        /// Type of data for the tag.
        ty: TagType,
        /// Tags inside this category.
        subtags: Vec<TagMenuNode>,

        // TODO: consolidate these bools into a single "display" value with enum variants
        /// Whether to display the tag family as an inline section instead of a
        /// submenu.
        section: bool,
        /// Whether to display a checkbox for the tag.
        checkbox: bool,
        /// Whether to display the contents of this tag inline.
        flatten: bool,
        /// Whether to hide the tag in the list.
        hidden: bool,
        /// Whether this tag is expected to be specified on all puzzles built
        /// into the program.
        expected: bool,
        /// Whether this tag should display a list of all possible values in the menu.
        list: bool,
    },
}
impl TagMenuNode {
    fn name(&self) -> Option<&'static str> {
        match self {
            TagMenuNode::Heading(_) | TagMenuNode::Separator => None,
            TagMenuNode::Tag { name, .. } => *name,
        }
    }
}

lazy_static! {
    /// List of tags as they should be arranged in the menu.
    pub static ref TAGS_MENU: Vec<TagMenuNode> = load_tag_menu();

    /// Set of known tags, programmed in `tags.kdl`.
    pub static ref TAGS: HashMap<String, TagData> = assemble_tag_data();

    /// List of tags that should be explicitly specified on every puzzle.
    pub(crate) static ref EXPECTED_TAGS: Vec<Vec<String>> = assemble_expected_tags();
}

fn load_tag_menu() -> Vec<TagMenuNode> {
    match include_str!("tags.kdl").parse() {
        Ok(kdl_document) => kdl_to_tag_list_node(&kdl_document, ""),
        Err(e) => {
            eprintln!("{e:#?}");
            panic!("error parsing tags.kdl");
        }
    }
}

fn kdl_to_tag_list_node(kdl: &kdl::KdlDocument, prefix: &str) -> Vec<TagMenuNode> {
    kdl.nodes()
        .iter()
        .map(|node| {
            let name = node.name().value();
            match name {
                "__heading" => TagMenuNode::Heading(leak_str(
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
                    let mut section = false;
                    let mut include_in_tag = true;
                    let mut checkbox = true;
                    let mut flatten = false;
                    let mut hidden = false;
                    let mut expected = false;
                    let mut list = false;
                    for entry in node.entries() {
                        match entry.name().map(|ident| ident.value()) {
                            None => {
                                display = Some(leak_str(
                                    entry.value().as_string().expect("expected string value"),
                                ));
                            }
                            Some("section") => {
                                section = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("include_in_tag") => {
                                include_in_tag =
                                    entry.value().as_bool().expect("expected bool value");
                            }
                            Some("checkbox") => {
                                checkbox = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("flatten") => {
                                flatten = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("hidden") => {
                                hidden = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("expected") => {
                                expected = entry.value().as_bool().expect("expected bool value");
                            }
                            Some("list") => {
                                list = entry.value().as_bool().expect("expected bool value");
                            }
                            Some(k) => panic!("unknown tag property key {k:?}"),
                        }
                    }

                    checkbox &= include_in_tag;

                    let display = display.unwrap_or_else(|| {
                        if flatten {
                            ""
                        } else {
                            panic!("tag {name:?} is missing display string")
                        }
                    });

                    let children = match node.children() {
                        Some(children) => {
                            if include_in_tag && !name.is_empty() {
                                kdl_to_tag_list_node(children, &format!("{name}/"))
                            } else {
                                kdl_to_tag_list_node(children, prefix)
                            }
                        }
                        None => vec![],
                    };

                    TagMenuNode::Tag {
                        name: include_in_tag.then(|| leak_str(&name)),
                        display,
                        ty: tag_type,
                        subtags: children,

                        section,
                        checkbox,
                        flatten,
                        hidden,
                        expected,
                        list,
                    }
                }
            }
        })
        .collect()
}

fn assemble_tag_data() -> HashMap<String, TagData> {
    let mut all_tags = HashMap::new();
    for node in &*TAGS_MENU {
        assemble_tag_data_recursive(&mut all_tags, node);
    }
    all_tags
}
fn assemble_tag_data_recursive(all_tags: &mut HashMap<String, TagData>, node: &TagMenuNode) {
    match node {
        TagMenuNode::Heading(_) | TagMenuNode::Separator => (),
        TagMenuNode::Tag {
            name, subtags, ty, ..
        } => {
            if let Some(name) = *name {
                all_tags.insert(name.to_owned(), TagData { ty: *ty });
            }
            for subtag in subtags {
                assemble_tag_data_recursive(all_tags, subtag);
            }
        }
    }
}

fn assemble_expected_tags() -> Vec<Vec<String>> {
    let mut expected_tags = vec![];
    for node in &*TAGS_MENU {
        assemble_expected_tags_recursive(&mut expected_tags, node);
    }
    expected_tags.sort();
    expected_tags
}
fn assemble_expected_tags_recursive(expected_tags: &mut Vec<Vec<String>>, node: &TagMenuNode) {
    match node {
        TagMenuNode::Heading(_) | TagMenuNode::Separator => (),
        TagMenuNode::Tag {
            name,
            subtags,
            expected,
            ..
        } => {
            if *expected {
                match *name {
                    Some(name) => expected_tags.push(vec![name.to_owned()]),
                    None => expected_tags.push(
                        subtags
                            .iter()
                            .filter_map(|subtag| subtag.name())
                            .map(|s| s.to_owned())
                            .collect(),
                    ),
                }
            }
            for subtag in subtags {
                assemble_expected_tags_recursive(expected_tags, subtag);
            }
        }
    }
}

fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

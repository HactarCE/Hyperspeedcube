use std::fmt;

use super::*;

/// Catalog of known tags, defined in `tags.kdl`.
#[derive(Debug)]
pub struct AllTags {
    menu: TagMenuNode,
    list: Vec<Arc<str>>,
    tag_data: HashMap<Arc<str>, TagData>,
    expected: Vec<Vec<Arc<str>>>,
}
impl AllTags {
    /// Returns the data for a tag, or `None` if no such tag exists.
    pub fn get(&self, tag_name: &str) -> Result<&TagData, UnknownTag> {
        self.tag_data.get(tag_name).ok_or_else(|| UnknownTag {
            name: tag_name.to_owned(),
        })
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

    /// Returns the ancestors of a tag, not including the tag itself. For
    /// example, the ancestors of `"shape/3d/cube"` are `["shape/3d",
    /// "shape"]`.
    pub fn ancestors<'a>(&'a self, tag: &'a str) -> impl 'a + Iterator<Item = &'a str> {
        std::iter::successors(Some(tag), |s| Some(s.rsplit_once(['/', '='])?.0)).skip(1)
    }

    pub(super) fn load() -> Self {
        let top_level_menu_items = match include_str!("../tags.kdl").parse() {
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
            hidden: false,
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
                    hidden: _,
                    list: _,
                } => {
                    if let Some(name) = name {
                        tags_list.push(Arc::clone(name));

                        let data = TagData {
                            name: Arc::clone(name),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTag {
    pub name: String,
}
impl std::error::Error for UnknownTag {}
impl fmt::Display for UnknownTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown tag {:?}", self.name)
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
                    let mut hidden = false;
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
                            Some("hidden") => {
                                hidden = entry.value().as_bool().expect("expected bool value");
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
                        hidden,
                        list,
                    }
                }
            }
        })
        .collect()
}

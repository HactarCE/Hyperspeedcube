use std::collections::BTreeSet;

use super::*;

/// Nested menu structure containing puzzles.
///
/// Multiple menus may be defined. They are indexed by [`TypeId`]s.
///
/// Menu path components are separated by `|` and components must not be empty.
/// There is no leading `|` or trailing `|`, and `||` is not allowed.
#[derive(Debug)]
pub struct Menu {
    /// Human-friendly name for the kind of puzzle contained in the menu.
    pub name: String,

    /// Menu node styles, indexed by menu path.
    contents: HashMap<String, MenuContent>,
    /// Children for each menu node, indexed by menu path.
    children: HashMap<String, BTreeSet<MenuNodeChild>>,
    /// Default child of each menu node, indexed by menu path.
    default_child: HashMap<String, MenuPathBuf>,

    /// Partial map from puzzle ID to menu path.
    puzzle_id_to_path: HashMap<String, MenuPathBuf>,
}

impl Menu {
    /// Constructs a new empty menu.
    pub fn new(name: String) -> Self {
        Self {
            name,
            contents: HashMap::default(),
            children: HashMap::default(),
            default_child: HashMap::default(),
            puzzle_id_to_path: HashMap::default(),
        }
    }

    /// Adds a node to the menu.
    ///
    /// Returns an error if the path is invalid or if a node already exists at
    /// the given path.
    pub fn add_node(
        &mut self,
        path: String,
        content: MenuContent,
        priority: i64,
        default: bool,
    ) -> Result<()> {
        let menu_path = MenuPath::from_str(&path)
            .ok_or_eyre("menu path must not contain empty components")?
            .to_path_buf();

        match self.contents.entry(path.clone()) {
            hash_map::Entry::Occupied(e) => bail!("duplicate menu entry at {:?}", e.key()),
            hash_map::Entry::Vacant(e) => {
                if let MenuContent::End { id } = &content {
                    // This may clobber an existing path. That's ok.
                    self.puzzle_id_to_path
                        .insert(id.to_string(), menu_path.clone());
                }

                e.insert(content)
            }
        };

        if !path.is_empty() {
            let (parent, last_component) = path.rsplit_once('|').unwrap_or(("", &path));

            self.children
                .entry(parent.to_string())
                .or_default()
                .insert(MenuNodeChild {
                    priority,
                    next_path_component: last_component.to_string(),
                    full_path: menu_path.clone(),
                });

            if default {
                match self.default_child.entry(parent.to_string()) {
                    hash_map::Entry::Occupied(e) => {
                        bail!("conflicting default menu entries for {:?}", e.key());
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(menu_path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns the default descendent of a path. If there is no default child,
    /// then the path is returned unmodified.
    pub fn default_descendent<'a>(&'a self, mut path: MenuPath<'a>) -> MenuPath<'a> {
        while let Some(default) = self.default_child.get(path.as_str()) {
            path = default.as_path_ref();
        }
        path
    }

    /// Returns whether there is a node at the given path.
    pub fn contains(&self, path: MenuPath<'_>) -> bool {
        self.contents.contains_key(path.as_str())
    }

    /// Returns the content for the node at the given path, or `None` if there
    /// is no node.
    pub fn get_content(&self, path: MenuPath<'_>) -> Option<&MenuContent> {
        self.contents.get(path.as_str())
    }

    /// Returns the children of a menu node, in sorted order.
    pub fn children<'a>(&'a self, path: MenuPath<'_>) -> impl Iterator<Item = MenuPath<'a>> {
        self.children
            .get(path.as_str())
            .unwrap_or(const { &BTreeSet::new() })
            .iter()
            .map(|child| child.full_path.as_path_ref())
    }

    /// Returns whether a menu node is displayed as a section.
    pub fn is_section(&self, path: MenuPath<'_>) -> bool {
        matches!(self.get_content(path), Some(&MenuContent::Section))
    }

    /// Returns the path of a menu entry containing a specific puzzle or puzzle
    /// generator, or `None` if it isn't present.
    pub fn puzzle_id_to_path(&self, puzzle_id: &str) -> Option<MenuPath<'_>> {
        Some(self.puzzle_id_to_path.get(puzzle_id)?.as_path_ref())
    }

    /// Returns an iterator over nodes that do not have a parent and thus do not
    /// appear in the menu.
    ///
    /// Exhausting this iterator takes _O(n)_ time with respect to the number of
    /// nodes in the menu, even if there are no orphans.
    pub fn orphans(&self) -> impl Iterator<Item = MenuPath<'_>> {
        self.contents
            .keys()
            .filter_map(|s| MenuPath::from_str(s))
            .filter(|path| path.parent().is_some_and(|parent| !self.contains(parent)))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MenuNodeChild {
    priority: i64,
    next_path_component: String,
    full_path: MenuPathBuf,
}

impl PartialOrd for MenuNodeChild {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MenuNodeChild {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (Ord::cmp(&self.priority, &other.priority).reverse())
            .then_with(|| numeric_sort::cmp(&self.next_path_component, &other.next_path_component))
    }
}

/// Content of a menu component.
#[derive(Debug)]
pub enum MenuContent {
    /// Display the next component in a column.
    Column {
        /// Title to display above the options.
        title: String,
    },
    /// Display the next components in a section beneath this one.
    ///
    /// A section titled "Other" will be automatically created for sibling nodes
    /// that are not sections.
    Section,
    /// Display all subsequent components inline.
    Inline {
        /// Label to display above the options.
        label: String,
    },
    /// Puzzle or generator at the end of the menu path.
    End {
        /// Puzzle or generator ID.
        id: CatalogId,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct MenuPathBuf {
    str_repr: String,
    len: usize,
}

impl MenuPathBuf {
    fn as_path_ref(&self) -> MenuPath<'_> {
        let &Self { ref str_repr, len } = self;
        MenuPath { str_repr, len }
    }
}

/// Path to an entry in a [`Menu`].
///
/// Path components are delimited using `|`. Paths cannot begin or end with `|`
/// and cannot contain `||`. All other strings are valid paths.
///
/// The empty string represents the root path.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MenuPath<'a> {
    /// `|`-delimited components.
    str_repr: &'a str,
    /// Number of components in the path.
    len: usize,
}

impl fmt::Display for MenuPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.str_repr)
    }
}

impl<'a> MenuPath<'a> {
    /// Empty path corresponding to the root node in the menu.
    pub const ROOT: Self = Self {
        str_repr: "",
        len: 0,
    };

    /// Returns a menu path from a string, or `None` if the path is invalid.
    pub fn from_str(str_repr: &'a str) -> Option<Self> {
        Self::is_valid(str_repr).then(|| {
            let len = if str_repr.is_empty() {
                0
            } else {
                str_repr.chars().filter(|&c| c == '|').count() + 1
            };
            Self { str_repr, len }
        })
    }

    fn to_path_buf(self) -> MenuPathBuf {
        let Self { str_repr, len } = self;
        let str_repr = str_repr.to_string();
        MenuPathBuf { str_repr, len }
    }

    /// Returns the string representation of the path.
    pub fn as_str(self) -> &'a str {
        self.str_repr
    }

    /// Returns whether the path is empty.
    pub fn is_root(self) -> bool {
        self.str_repr.is_empty()
    }

    /// Returns the number of compenents in the path.
    pub fn len(self) -> usize {
        self.len
    }

    /// Truncates a path to the given number of components.
    #[must_use]
    pub fn truncate(self, len: usize) -> Self {
        match len.checked_sub(1) {
            Some(n) => match self
                .str_repr
                .char_indices()
                .filter(|&(_, c)| c == '|')
                .nth(n)
            {
                Some((i, _)) => Self {
                    str_repr: &self.str_repr[..i],
                    len,
                },
                None => self,
            },
            None => Self::ROOT,
        }
    }

    /// Returns whether a path starts with a prefix.
    pub fn starts_with(self, prefix: Self) -> bool {
        // TODO: most calls to this function don't need to check most of the
        // components. consider optimizing by starting closer to the end of the
        // string, or by only comparing near the end.
        self.str_repr.starts_with(prefix.str_repr) && {
            let remaining = &self.str_repr[prefix.str_repr.len()..];
            remaining.is_empty() || remaining.starts_with('|')
        }
    }

    /// Returns the entire path except for the last component, or `None` for the
    /// root path.
    pub fn parent(self) -> Option<Self> {
        match self.str_repr.rsplit_once('|') {
            Some((parent, _)) => Some(Self {
                str_repr: parent,
                len: self.len.strict_sub(1),
            }),
            None if self.is_root() => None,
            None => Some(Self::ROOT),
        }
    }

    /// Returns the last component of a path, or an empty string for the root
    /// path.
    pub fn last_component(self) -> &'a str {
        match self.str_repr.rsplit_once('|') {
            Some((_, last_component)) => last_component,
            None => self.str_repr,
        }
    }

    pub(super) fn is_valid(s: &str) -> bool {
        !(s.starts_with('|') || s.ends_with('|') || s.contains("||"))
    }
}

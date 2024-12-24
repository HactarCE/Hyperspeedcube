use eyre::{bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;

use super::{PieceType, PieceTypeMask};

/// Piece type hierarchy or subhierarchy.
#[derive(Debug)]
pub struct PieceTypeHierarchy {
    /// Child nodes in the hierarchy.
    pub nodes: IndexMap<String, PieceTypeHierarchyNode>,
    /// Mask of piece types that are children of the node.
    pub types: PieceTypeMask,
}
impl PieceTypeHierarchy {
    /// Returns an empty piece type hierarchy, given the number of piece types
    /// in the puzzle.
    pub fn new(piece_type_count: usize) -> Self {
        Self {
            nodes: IndexMap::new(),
            types: PieceTypeMask::new_empty(piece_type_count),
        }
    }

    /// Returns a (possibly nested) node in the piece type hierarchy.
    pub fn get(&self, mut path: &str) -> Option<&PieceTypeHierarchyNode> {
        let mut hierarchy = self;
        loop {
            if let Some((l, r)) = path.split_once('/') {
                match &hierarchy.nodes.get(l)?.contents {
                    PieceTypeHierarchyNodeContents::Category(sub) => {
                        path = r;
                        hierarchy = sub;
                    }
                    PieceTypeHierarchyNodeContents::Type(_) => break None,
                }
            } else {
                break hierarchy.nodes.get(path);
            }
        }
    }

    /// Returns an entry in the hierarchy. This should only be used during
    /// initial construction of a piece type hierarchy.
    fn entry(
        &mut self,
        path: &str,
    ) -> Result<indexmap::map::Entry<'_, String, PieceTypeHierarchyNode>> {
        let mut segments = path.split('/');
        let init_segment = segments.next().expect("path has no segments").to_owned();
        let mut entry = self.nodes.entry(init_segment.clone());
        let mut partial_path = init_segment;
        for segment in segments {
            partial_path += segment;
            entry = match entry {
                indexmap::map::Entry::Occupied(e) => match &mut e.into_mut().contents {
                    PieceTypeHierarchyNodeContents::Category(sub) => &mut sub.nodes,
                    PieceTypeHierarchyNodeContents::Type(_) => {
                        bail!("piece type {partial_path:?} cannot have subtype {path:?}")
                    }
                },
                indexmap::map::Entry::Vacant(e) => {
                    &mut e
                        .insert(PieceTypeHierarchyNode {
                            display: None,
                            contents: PieceTypeHierarchyNodeContents::Category(Box::new(
                                PieceTypeHierarchy::new(self.types.max_len()),
                            )),
                        })
                        .contents
                        .unwrap_category_mut()
                        .nodes
                }
            }
            .entry(segment.to_owned());
        }
        Ok(entry)
    }
    /// Sets the display name for a piece type or piece type category.
    pub fn set_display(&mut self, path: &str, display: String) -> Result<()> {
        let piece_type_count = self.types.max_len();
        let node = self
            .entry(path)?
            .or_insert_with(|| PieceTypeHierarchyNode::new(piece_type_count));
        node.display = Some(display);
        Ok(())
    }
    /// Sets the piece ID for a piece type.
    pub fn set_piece_type_id(&mut self, path: &str, piece_type_id: PieceType) -> Result<()> {
        let piece_type_count = self.types.max_len();
        let node = self
            .entry(path)?
            .or_insert_with(|| PieceTypeHierarchyNode::new(piece_type_count));
        if let PieceTypeHierarchyNodeContents::Category(sub) = &node.contents {
            if !sub.nodes.is_empty() {
                let subtype_names = sub.nodes.keys().collect_vec();
                bail!("piece type {path:?} cannot have subtypes {subtype_names:?}");
            }
        }
        node.contents = PieceTypeHierarchyNodeContents::Type(piece_type_id);
        self.add_piece_type_along_path(path, piece_type_id);
        Ok(())
    }
    fn add_piece_type_along_path(&mut self, path: &str, piece_type_id: PieceType) {
        for prefix in Self::path_prefixes(path) {
            if let Ok(indexmap::map::Entry::Occupied(e)) = self.entry(prefix) {
                if let PieceTypeHierarchyNodeContents::Category(cat) = &mut e.into_mut().contents {
                    cat.types.insert(piece_type_id);
                }
            }
        }
        self.types.insert(piece_type_id);
    }

    pub(crate) fn path_prefixes(path: &str) -> impl Iterator<Item = &str> {
        std::iter::successors(Some(path), |s| Some(s.rsplit_once('/')?.0))
    }

    pub(crate) fn paths_with_no_display_name(&self) -> Vec<String> {
        let mut ret = vec![];
        let mut stack = self.nodes.iter().map(|(k, v)| (k.clone(), v)).collect_vec();
        while let Some((k, v)) = stack.pop() {
            if v.display.is_none() {
                ret.push(k.clone());
            }
            match &v.contents {
                PieceTypeHierarchyNodeContents::Category(sub) => {
                    for (k2, v2) in &sub.nodes {
                        stack.push((format!("{k}/{k2}"), v2));
                    }
                }
                PieceTypeHierarchyNodeContents::Type(_) => (),
            }
        }
        ret
    }
}

/// Node in the tree representing the hierarchy of piece types in a puzzle.
#[derive(Debug)]
pub struct PieceTypeHierarchyNode {
    /// User-friendly display name for the piece type or category. (e.g.,
    /// "Oblique (1, 2) (left)")
    pub display: Option<String>,
    /// Piece type or list of subtypes.
    pub contents: PieceTypeHierarchyNodeContents,
}
impl PieceTypeHierarchyNode {
    fn new(piece_type_count: usize) -> Self {
        Self {
            display: None,
            contents: PieceTypeHierarchyNodeContents::Category(Box::new(PieceTypeHierarchy::new(
                piece_type_count,
            ))),
        }
    }
}

/// Piece type or list of subtypes.
#[derive(Debug)]
pub enum PieceTypeHierarchyNodeContents {
    /// Piece category, with a list of subtypes.
    Category(Box<PieceTypeHierarchy>),
    /// Single piece type.
    Type(PieceType),
}
impl PieceTypeHierarchyNodeContents {
    fn unwrap_category_mut(&mut self) -> &mut PieceTypeHierarchy {
        match self {
            PieceTypeHierarchyNodeContents::Category(sub) => sub,
            PieceTypeHierarchyNodeContents::Type(_) => panic!("expected piece type category"),
        }
    }
}

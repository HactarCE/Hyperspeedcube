//! Notation structures without attached span information.
//!
//! Use these when you do not care about mapping notation elements to the
//! original input string, or when you want to construct notation elements from
//! scratch.

use std::fmt;

use crate::{Features, InvertError, ParseError, Str};

pub use crate::common::*;

/// Parses a string containing puzzle notation into a list of [`Node`]s.
pub fn parse_notation(s: &str, features: Features) -> Result<NodeList, Vec<ParseError<'_>>> {
    Ok(crate::spanned::parse_notation(s, features)?.to_unspanned(s))
}

/// List of notation elements.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct NodeList(pub Vec<Node>);

impl fmt::Display for NodeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated_list(f, &self.0, " ")
    }
}

impl NodeList {
    /// Returns a list with all nodes inverted, in reverse order.
    pub fn inv(&self) -> Result<Self, InvertError> {
        self.0
            .iter()
            .rev()
            .map(|n| n.inv())
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

/// Notation element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    /// Notation element that can be repeated.
    RepeatedNode {
        /// The notation element that is repeated.
        inner: RepeatableNode,
        /// Multiplier, which defaults to `1`.
        multiplier: Multiplier,
    },
    /// Pause, written using `.`.
    Pause,
    /// Square-1 move.
    Sq1Move(Sq1Move),
    /// WCA Megaminx scrambling move.
    MegaminxScrambleMove(MegaminxScrambleMove),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::RepeatedNode { inner, multiplier } => write!(f, "{inner}{multiplier}"),
            Node::Pause => write!(f, "."),
            Node::Sq1Move(sq1_move) => write!(f, "{sq1_move}"),
            Node::MegaminxScrambleMove(megaminx_scramble_move) => {
                write!(f, "{megaminx_scramble_move}")
            }
        }
    }
}

impl From<RepeatableNode> for Node {
    fn from(value: RepeatableNode) -> Self {
        value.with_multiplier(1)
    }
}

impl From<Sq1Move> for Node {
    fn from(value: Sq1Move) -> Self {
        Self::Sq1Move(value)
    }
}

impl From<MegaminxScrambleMove> for Node {
    fn from(value: MegaminxScrambleMove) -> Self {
        Self::MegaminxScrambleMove(value)
    }
}

impl Node {
    /// Returns the inverse node.
    ///
    /// Returns an error if there are any NISS nodes.
    pub fn inv(&self) -> Result<Self, InvertError> {
        match self {
            Node::RepeatedNode { inner, multiplier } => {
                Ok(inner.clone().with_multiplier(multiplier.inv()?))
            }
            Node::Pause => Ok(Node::Pause),
            Node::Sq1Move(sq1_move) => Ok(Node::Sq1Move(sq1_move.inv()?)),
            Node::MegaminxScrambleMove(megaminx_scramble_move) => {
                Ok(Node::MegaminxScrambleMove(megaminx_scramble_move.inv()))
            }
        }
    }
}

/// Notation element that can be repeated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RepeatableNode {
    /// Move, such as `x` or `IR2` or `{1..-1}U[ R->F ]`
    Move {
        /// Layer mask.
        ///
        /// Example `{1..-1}` in the move `{1..-1}U[ R->F ]`
        layer_mask: LayerMask,
        /// Move family, which must not be empty.
        ///
        /// Example: `U` in the move `{1..-1}U[R->F]`
        family: Str,
        /// Bracketed transform, if present. Not including the brackets `[]` or
        /// surrounding whitespace.
        ///
        /// Example: `R->F` in the move `{1..-1}U[ R->F ]`
        transform: Option<Str>,
    },
    /// Rotation using `@`, such as `@U[R->F]`.
    Rotation {
        /// Move family, which may be empty.
        family: Str,
        /// Bracketed transform, if present. Not including the brackets `[]` or
        /// surrounding whitespace.
        ///
        /// Example: `R->F` in the rotation `@U[ R->F ]`
        transform: Option<Str>,
    },
    /// List of nodes surrounded by `()`.
    Group {
        /// Kind of group.
        kind: GroupKind,
        /// Nodes inside the group.
        contents: NodeList,
    },
    /// Two lists of nodes surrounded by `[]` with a symbol between them. This
    /// is used for conjugate & commutator notation.
    BinaryGroup {
        /// Kind of group.
        kind: BinaryGroupKind,
        /// Nodes inside each half of the group.
        contents: [NodeList; 2],
    },
}

impl fmt::Display for RepeatableNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepeatableNode::Move {
                layer_mask,
                family,
                transform,
            } => {
                write!(f, "{layer_mask}{family}")?;
                if let Some(tf) = transform {
                    write!(f, "[{tf}]")?;
                }
                Ok(())
            }
            RepeatableNode::Rotation { family, transform } => {
                write!(f, "@{family}")?;
                if let Some(tf) = transform {
                    write!(f, "[{tf}]")?;
                }
                Ok(())
            }
            RepeatableNode::Group { kind, contents } => {
                if let Some(prefix) = kind.prefix() {
                    write!(f, "{prefix}")?;
                }
                write!(f, "({contents})")?;
                Ok(())
            }
            RepeatableNode::BinaryGroup { kind, contents } => {
                let [a, b] = contents;
                let sep = kind.separator();
                write!(f, "[{a}{sep} {b}]")
            }
        }
    }
}

impl RepeatableNode {
    /// Returns a `Node` that contains this node followed by a multiplier.
    pub fn with_multiplier(self, multiplier: impl Into<Multiplier>) -> Node {
        Node::RepeatedNode {
            inner: self,
            multiplier: multiplier.into(),
        }
    }
}

/// Layer mask for a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct LayerMask {
    /// Whether the layer mask is inverted using `~`.
    pub invert: bool,
    /// Contents of the layer mask.
    pub contents: Option<LayerMaskContents>,
}

impl fmt::Display for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.invert {
            write!(f, "~")?;
        }
        if let Some(contents) = &self.contents {
            write!(f, "{contents}")?;
        }
        Ok(())
    }
}

impl From<Option<LayerMaskContents>> for LayerMask {
    fn from(contents: Option<LayerMaskContents>) -> Self {
        let invert = false;
        Self { invert, contents }
    }
}

/// Contents of a layer mask for a move.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum LayerMaskContents {
    /// Single positive layer.
    ///
    /// Example: `3`
    Single(u16),
    /// Positive layer range.
    ///
    /// Example: `3-4`
    Range(u16, u16),
    /// Layer set, which supports negative numbers.
    ///
    /// Example: `{1..-2,6}`
    Set(Vec<LayerMaskSetElement>),
}

impl fmt::Display for LayerMaskContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerMaskContents::Single(i) => write!(f, "{i}"),
            LayerMaskContents::Range(i, j) => write!(f, "{i}-{j}"),
            LayerMaskContents::Set(elements) => {
                write!(f, "{{")?;
                write_separated_list(f, elements, ",")?;
                write!(f, "}}")?;
                Ok(())
            }
        }
    }
}

impl LayerMaskContents {
    /// Resolves the layer mask to a definite set of layer ranges.
    ///
    /// Layers are 1-indexed. Each range is inclusive and in order (i.e., `[lo,
    /// hi]` where `lo <= hi`). Ranges are in arbitrary order.
    ///
    /// - `1` is the outermost layer
    /// - `n` is the innermost layer, where `n` is `layer_count`
    /// - There is no layer zero `0`
    pub fn to_ranges(&self, layer_count: u16) -> Vec<[u16; 2]> {
        match self {
            LayerMaskContents::Single(l) => {
                if (1..=layer_count).contains(l) {
                    vec![[*l; 2]]
                } else {
                    return vec![];
                }
            }
            LayerMaskContents::Range(l1, l2) => {
                let lo = std::cmp::min(*l1, *l2);
                let hi = std::cmp::max(*l1, *l2);
                if hi >= 1 && lo <= layer_count {
                    vec![[lo.max(1), hi.min(layer_count)]]
                } else {
                    vec![]
                }
            }
            LayerMaskContents::Set(layer_mask_set_elements) => layer_mask_set_elements
                .iter()
                .filter_map(|elem| elem.to_range(layer_count))
                .collect(),
        }
    }
}

/// Element of a layer set.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum LayerMaskSetElement {
    /// Signed layer in a layer set.
    Single(i16),
    /// Signed layer in a layer set.
    ///
    /// Example: `1..-2`
    Range(i16, i16),
}

impl fmt::Display for LayerMaskSetElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerMaskSetElement::Single(i) => write!(f, "{i}"),
            LayerMaskSetElement::Range(i, j) => write!(f, "{i}..{j}"),
        }
    }
}

impl LayerMaskSetElement {
    fn to_range(self, layer_count: u16) -> Option<[u16; 2]> {
        let range = match self {
            LayerMaskSetElement::Single(l) => [l, l],
            LayerMaskSetElement::Range(l1, l2) => [l1, l2],
        };
        crate::resolve_signed_layer_range(layer_count, range)
    }
}

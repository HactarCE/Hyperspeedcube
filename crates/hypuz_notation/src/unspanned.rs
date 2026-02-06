//! Notation structures without attached span information.
//!
//! Use these when you do not care about mapping notation elements to the
//! original input string, or when you want to construct notation elements from
//! scratch.

use std::fmt;
use std::ops::{Deref, DerefMut};

pub use crate::common::*;
use crate::{Features, InvertError, ParseError, Str};

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

impl Deref for NodeList {
    type Target = Vec<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeList {
    /// Constructs a new empty node list.
    pub fn new() -> Self {
        Self(Vec::new())
    }

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

impl<T: Into<LayerMaskContents>> From<T> for LayerMask {
    fn from(value: T) -> Self {
        let invert = false;
        let contents = Some(value.into());
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
    Single(Layer),
    /// Positive layer range.
    ///
    /// Example: `3-4`
    Range(LayerRange),
    /// Layer set, which supports negative numbers.
    ///
    /// Example: `{1..-2,6}`
    Set(Vec<LayerMaskSetElement>),
}

impl fmt::Display for LayerMaskContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerMaskContents::Single(i) => write!(f, "{i}"),
            LayerMaskContents::Range(range) => write!(f, "{range}"),
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
    /// Resolves the layer mask to a definite set of layer ranges in arbitrary
    /// order.
    pub fn to_ranges(&self, layer_count: u16) -> Vec<LayerRange> {
        match self {
            LayerMaskContents::Single(l) => (*l <= layer_count)
                .then_some(LayerRange::from_layer(*l))
                .into_iter()
                .collect(),
            LayerMaskContents::Range(range) => range
                .clamp_to_layer_count(layer_count)
                .into_iter()
                .collect(),
            LayerMaskContents::Set(layer_mask_set_elements) => layer_mask_set_elements
                .iter()
                .filter_map(|elem| elem.to_range(layer_count))
                .collect(),
        }
    }

    /// Returns the set of layers specified by the layer mask.
    pub fn layer_set(&self, layer_count: u16) -> LayerSet {
        match self {
            LayerMaskContents::Single(l) => LayerSet::from_layer(*l),
            LayerMaskContents::Range(range) => LayerSet::from_range(*range),
            LayerMaskContents::Set(elements) => {
                let mut ret = LayerSet::new();
                for elem in elements {
                    if let Some(range) = elem.to_range(layer_count) {
                        ret.insert_range(range);
                    }
                }
                ret
            }
        }
    }

    /// Canonicalizes the layer mask.
    ///
    /// Returns `None` if the layer mask does not include any layers.
    pub fn simplify(&self, layer_count: u16) -> Option<Self> {
        Some(Self::from_ranges(self.to_ranges(layer_count))).filter(|contents| !contents.is_empty())
    }

    /// Returns whether the layer mask is an empty set.
    pub fn is_empty(&self) -> bool {
        matches!(self, LayerMaskContents::Set(list) if list.is_empty())
    }

    fn from_ranges(mut ranges: Vec<LayerRange>) -> Self {
        let mut ret: Vec<LayerRange> = vec![];
        ranges.sort();
        for r2 in ranges {
            if let Some(r1) = ret.last_mut()
                && let Some(combined) = r1.union(r2)
            {
                *r1 = combined;
            } else {
                ret.push(r2);
            }
        }
        match ret.as_slice() {
            [range] => match range.to_single_layer() {
                Some(layer) => Self::Single(layer),
                None => Self::Range(*range),
            },
            _ => Self::Set(ret.into_iter().map(|r| r.into()).collect()),
        }
    }
}

impl FromIterator<LayerRange> for LayerMaskContents {
    fn from_iter<T: IntoIterator<Item = LayerRange>>(iter: T) -> Self {
        Self::from_ranges(iter.into_iter().collect())
    }
}

impl From<LayerSet> for LayerMaskContents {
    fn from(value: LayerSet) -> Self {
        Self::from(&value)
    }
}

impl From<&LayerSet> for LayerMaskContents {
    fn from(value: &LayerSet) -> Self {
        value.iter().map(LayerRange::from_layer).collect()
    }
}

impl FromIterator<Layer> for LayerMaskContents {
    fn from_iter<T: IntoIterator<Item = Layer>>(iter: T) -> Self {
        iter.into_iter().collect()
    }
}

/// Element of a layer set.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum LayerMaskSetElement {
    /// Signed layer in a layer set.
    Single(SignedLayer),
    /// Signed layer in a layer set.
    ///
    /// Example: `1..-2`
    Range([SignedLayer; 2]),
}

impl fmt::Display for LayerMaskSetElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerMaskSetElement::Single(i) => write!(f, "{i}"),
            LayerMaskSetElement::Range([i, j]) => write!(f, "{i}..{j}"),
        }
    }
}

impl From<LayerRange> for LayerMaskSetElement {
    fn from(range: LayerRange) -> Self {
        match range.to_single_layer() {
            Some(layer) => LayerMaskSetElement::Single(layer.to_signed()),
            None => LayerMaskSetElement::Range([range.start(), range.end()].map(Layer::to_signed)),
        }
    }
}

impl LayerMaskSetElement {
    fn to_range(self, layer_count: u16) -> Option<LayerRange> {
        match self {
            LayerMaskSetElement::Single(l) => l.resolve(layer_count).map(LayerRange::from_layer),
            LayerMaskSetElement::Range(range) => SignedLayer::resolve_range(range, layer_count),
        }
    }
}

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
    /// Move, such as `x` or `IR2` or `{1..-1}U[R->F]`
    Move(Move),
    /// Rotation using `@`, such as `@U[R->F]`.
    Rotation(Rotation),
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
            RepeatableNode::Move(mv) => write!(f, "{mv}"),
            RepeatableNode::Rotation(rot) => write!(f, "{rot}"),
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

impl From<Move> for RepeatableNode {
    fn from(value: Move) -> Self {
        Self::Move(value)
    }
}

impl From<Rotation> for RepeatableNode {
    fn from(value: Rotation) -> Self {
        Self::Rotation(value)
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

/// Move containing a layer prefix and a rotation.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Move {
    /// Layer prefix, which may be empty.
    ///
    /// An empty layer prefix is typically equivalent to a layer prefix that
    /// includes only layer 1.
    pub layers: LayerPrefix,
    /// Move family and transform.
    ///
    /// If the family is empty, then it displays as a single underscore `_`.
    pub rot: Rotation,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { layers, rot } = self;
        let Rotation { family, transform } = rot;
        write!(f, "{layers}{family}")?;
        if family.is_empty() {
            // If family name is empty, add an underscore so that at
            // least it parses back correctly.
            write!(f, "_")?;
        }
        if let Some(tf) = transform {
            write!(f, "[{tf}]")?;
        }
        Ok(())
    }
}

impl From<Rotation> for Move {
    fn from(value: Rotation) -> Self {
        value.into_move(LayerPrefix::default())
    }
}

impl Move {
    /// Returns a `Node` that contains this move followed by a multiplier.
    pub fn with_multiplier(self, multiplier: impl Into<Multiplier>) -> Node {
        RepeatableNode::from(self).with_multiplier(multiplier)
    }
}

/// Rotation, which may be displayed on its own or as part of a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Rotation {
    /// Move family, which may be empty for a rotation but must be nonempty for
    /// a transform.
    ///
    /// Example: `U` in the move `{1..-1}U[ R -> F ]` or the rotation `@U[ R ->
    /// F ]`
    pub family: Str,
    /// Bracketed transform, if present. Not including the brackets `[]` or
    /// surrounding whitespace.
    ///
    /// Example: `R -> F` in the move `{1..-1}U[ R -> F ]` or the rotation `@U[
    /// R -> F ]`
    pub transform: Option<Str>,
}

impl fmt::Display for Rotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { family, transform } = self;
        write!(f, "@{family}")?;
        if let Some(tf) = transform {
            write!(f, "[{tf}]")?;
        }
        Ok(())
    }
}

impl Rotation {
    /// Constructs a move.
    pub fn into_move(self, layers: LayerPrefix) -> Move {
        let rotation = self;
        Move {
            layers,
            rot: rotation,
        }
    }

    /// Returns a `Node` that contains this rotation followed by a multiplier.
    pub fn with_multiplier(self, multiplier: impl Into<Multiplier>) -> Node {
        RepeatableNode::from(self).with_multiplier(multiplier)
    }
}

/// Layer prefix for a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct LayerPrefix {
    /// Whether the layer set is inverted using `~`.
    pub invert: bool,
    /// Contents of the layer set.
    pub contents: Option<LayerPrefixContents>,
}

impl fmt::Display for LayerPrefix {
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

impl From<Option<LayerPrefixContents>> for LayerPrefix {
    fn from(contents: Option<LayerPrefixContents>) -> Self {
        let invert = false;
        Self { invert, contents }
    }
}

impl<T: Into<LayerPrefixContents>> From<T> for LayerPrefix {
    fn from(value: T) -> Self {
        let invert = false;
        let contents = Some(value.into());
        Self { invert, contents }
    }
}

impl LayerPrefix {
    /// Default (empty) layer prefix, which typically corresponds to layer 1.
    pub const DEFAULT: Self = Self {
        invert: false,
        contents: None,
    };
}

/// Contents of a layer prefix for a move.
///
/// This type directly corresponds to notation. When computing layer masks,
/// prefer [`LayerMask`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum LayerPrefixContents {
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
    Set(LayerSet),
}

impl fmt::Display for LayerPrefixContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerPrefixContents::Single(i) => write!(f, "{i}"),
            LayerPrefixContents::Range(range) => write!(f, "{range}"),
            LayerPrefixContents::Set(set) => write!(f, "{set}"),
        }
    }
}

impl LayerPrefixContents {
    /// Resolves the layer prefix to a list of positive layer ranges in
    /// arbitrary order.
    pub fn to_ranges(&self, layer_count: u16) -> Vec<LayerRange> {
        match self {
            LayerPrefixContents::Single(l) => (*l <= layer_count)
                .then_some(LayerRange::from_layer(*l))
                .into_iter()
                .collect(),
            LayerPrefixContents::Range(range) => range
                .clamp_to_layer_count(layer_count)
                .into_iter()
                .collect(),
            LayerPrefixContents::Set(set) => set.to_ranges(layer_count),
        }
    }

    /// Converts the layer prefix to a bitmask of layers.
    pub fn to_layer_mask(&self, layer_count: u16) -> LayerMask {
        match self {
            LayerPrefixContents::Single(l) => LayerMask::from_layer(*l),
            LayerPrefixContents::Range(range) => LayerMask::from_range(*range),
            LayerPrefixContents::Set(elements) => elements.to_layer_mask(layer_count),
        }
    }

    /// Canonicalizes the layer prefix.
    ///
    /// Returns `None` if the layer prefix does not include any layers.
    pub fn simplify(&self, layer_count: u16) -> Option<Self> {
        Some(Self::from_ranges(self.to_ranges(layer_count))).filter(|contents| !contents.is_empty())
    }

    /// Returns whether the layer prefix is an empty set `{}`.
    ///
    /// This is different from an empty layer prefix, which typically represents
    /// the set containing only layer 1.
    pub fn is_empty(&self) -> bool {
        matches!(self, LayerPrefixContents::Set(list) if list.is_empty())
    }

    fn from_ranges(mut ranges: Vec<LayerRange>) -> Self {
        let mut simplified_ranges: Vec<LayerRange> = vec![];
        ranges.sort();
        for r2 in ranges {
            if let Some(r1) = simplified_ranges.last_mut()
                && let Some(combined) = r1.union(r2)
            {
                *r1 = combined;
            } else {
                simplified_ranges.push(r2);
            }
        }
        match simplified_ranges.as_slice() {
            [range] => match range.to_single_layer() {
                Some(layer) => Self::Single(layer),
                None => Self::Range(*range),
            },
            _ => Self::Set(LayerSet::from_iter(simplified_ranges)),
        }
    }
}

impl FromIterator<LayerRange> for LayerPrefixContents {
    fn from_iter<T: IntoIterator<Item = LayerRange>>(iter: T) -> Self {
        Self::from_ranges(iter.into_iter().collect())
    }
}

impl From<LayerMask> for LayerPrefixContents {
    fn from(value: LayerMask) -> Self {
        Self::from(&value)
    }
}

impl From<&LayerMask> for LayerPrefixContents {
    fn from(value: &LayerMask) -> Self {
        value.iter().map(LayerRange::from_layer).collect()
    }
}

impl FromIterator<Layer> for LayerPrefixContents {
    fn from_iter<T: IntoIterator<Item = Layer>>(iter: T) -> Self {
        iter.into_iter().collect()
    }
}

/// Set of signed layers and layer ranges.
///
/// This type directly corresponds to notation. When computing layer masks,
/// prefer [`LayerMask`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct LayerSet {
    /// Elements of the set, in arbitrary order.
    ///
    /// These may overlap. The set may be empty.
    pub elements: Vec<LayerSetElement>,
}

impl fmt::Display for LayerSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        write_separated_list(f, &self.elements, ",")?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl LayerSet {
    /// Resolves the layer set to a list of positive layer ranges in arbitrary
    /// order.
    pub fn to_ranges(&self, layer_count: u16) -> Vec<LayerRange> {
        self.elements
            .iter()
            .filter_map(|elem| elem.to_range(layer_count))
            .collect()
    }

    /// Converts the layer prefix to a bitmask of layers.
    pub fn to_layer_mask(&self, layer_count: u16) -> LayerMask {
        let mut ret = LayerMask::new();
        for &elem in &self.elements {
            if let Some(range) = elem.to_range(layer_count) {
                ret.insert_range(range);
            }
        }
        ret
    }

    /// Returns whether the layer set is empty `{}`.
    ///
    /// This is different from an empty [`LayerPrefix`], which typically
    /// represents the set containing only layer 1.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

impl FromIterator<LayerRange> for LayerSet {
    fn from_iter<T: IntoIterator<Item = LayerRange>>(iter: T) -> Self {
        let elements = iter.into_iter().map(|r| r.into()).collect();
        LayerSet { elements }
    }
}

impl From<LayerMask> for LayerSet {
    fn from(value: LayerMask) -> Self {
        Self::from(&value)
    }
}

impl From<&LayerMask> for LayerSet {
    fn from(value: &LayerMask) -> Self {
        value.iter().map(LayerRange::from_layer).collect()
    }
}

impl FromIterator<Layer> for LayerSet {
    fn from_iter<T: IntoIterator<Item = Layer>>(iter: T) -> Self {
        iter.into_iter().collect()
    }
}

/// Element of a [`LayerSet`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum LayerSetElement {
    /// Signed layer in a layer set.
    Single(SignedLayer),
    /// Signed layer in a layer set.
    ///
    /// Example: `1..-2`
    Range([SignedLayer; 2]),
}

impl fmt::Display for LayerSetElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerSetElement::Single(i) => write!(f, "{i}"),
            LayerSetElement::Range([i, j]) => write!(f, "{i}..{j}"),
        }
    }
}

impl From<LayerRange> for LayerSetElement {
    fn from(range: LayerRange) -> Self {
        match range.to_single_layer() {
            Some(layer) => LayerSetElement::Single(layer.to_signed()),
            None => LayerSetElement::Range([range.start(), range.end()].map(Layer::to_signed)),
        }
    }
}

impl LayerSetElement {
    fn to_range(self, layer_count: u16) -> Option<LayerRange> {
        match self {
            LayerSetElement::Single(l) => l.resolve(layer_count).map(LayerRange::from_layer),
            LayerSetElement::Range(range) => SignedLayer::resolve_range(range, layer_count),
        }
    }
}

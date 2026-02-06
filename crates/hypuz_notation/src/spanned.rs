//! Notation structures with attached span information.
//!
//! Use these when you want to map notation elements to the original input
//! string, such as for highlighting moves as they are performed.

use chumsky::Parser;

pub use crate::common::*;
use crate::{Features, LayerFeatures, ParseError, Span, Spanned, Str, unspanned};

/// Resolves a span to a string.
fn src(source: &str, span: Span) -> Str {
    source[span.into_range()].into()
}

/// Parses a string containing puzzle notation into a list of [`Node`]s with
/// attached span information.
pub fn parse_notation(s: &str, features: Features) -> Result<NodeList, Vec<ParseError<'_>>> {
    let r = crate::parse::node_list_with_features(features).parse(s);
    r.into_result()
}

/// List of notation elements.
#[derive(Debug, Default, Clone)]
pub struct NodeList(pub Vec<Spanned<Node>>);

impl NodeList {
    /// Converts to [`unspanned::NodeList`].
    pub fn to_unspanned(&self, source: &str) -> unspanned::NodeList {
        unspanned::NodeList(
            self.0
                .iter()
                .map(|node| node.to_unspanned(source))
                .collect(),
        )
    }

    /// Parses a notation node from a string.
    pub fn from_str(s: &'_ str, features: Features) -> Result<Self, Vec<ParseError<'_>>> {
        crate::parse::node_list_with_features(features)
            .parse(s)
            .into_result()
    }
}

/// Notation element with attached span information.
#[derive(Debug, Clone)]
pub enum Node {
    /// Notation element that can be repeated.
    RepeatedNode {
        /// The notation element that is repeated.
        inner: Spanned<RepeatableNode>,
        /// Multiplier.
        ///
        /// If there is no multiplier, then this is `1` with an empty span
        /// immediately after the node.
        multiplier: Spanned<Multiplier>,
    },
    /// Pause, written using `.`.
    Pause,
    /// Square-1 move.
    Sq1Move(Sq1Move),
    /// WCA Megaminx scrambling move.
    MegaminxScrambleMove(MegaminxScrambleMove),
}

impl Node {
    /// Converts to [`unspanned::Node`].
    pub fn to_unspanned(&self, source: &str) -> unspanned::Node {
        match self {
            Node::RepeatedNode { inner, multiplier } => unspanned::Node::RepeatedNode {
                inner: inner.to_unspanned(source),
                multiplier: **multiplier,
            },
            Node::Pause => unspanned::Node::Pause,
            Node::Sq1Move(sq1_move) => unspanned::Node::Sq1Move(*sq1_move),
            Node::MegaminxScrambleMove(megaminx_scramble_move) => {
                unspanned::Node::MegaminxScrambleMove(*megaminx_scramble_move)
            }
        }
    }
}

/// Notation element that can be repeated.
#[derive(Debug, Clone)]
pub enum RepeatableNode {
    /// Move, such as `x` or `IR2` or `{1..-1}U[ R->F ]`
    Move {
        /// Layer prefix.
        ///
        /// For an empty layer prefix, this has an empty span.
        ///
        /// Example `{1..-1}` in the move `{1..-1}U[ R->F ]`
        layers: Spanned<LayerPrefix>,
        /// Span of the move family, which must not be empty.
        ///
        /// Example: `U` in the move `{1..-1}U[ R->F ]`
        family: Span,
        /// Span of the bracketed transform, if present.
        ///
        /// The outer span includes the brackets; the inner span is only the
        /// contents of the brackets, not including surrounding whitespace.
        ///
        /// Example: `R->F` in the move `{1..-1}U[ R->F ]`
        transform: Option<Spanned<Span>>,
    },
    /// Rotation using `@`, such as `@U[ R->F ]`.
    Rotation {
        /// Span of the `@` symbol.
        at_sign: Span,
        /// Span of the move family, which may be empty.
        ///
        /// Example: `U` in the rotation `@U[ R->F ]`
        family: Span,
        /// Span of the bracketed transform, if present.
        ///
        /// The outer span includes the brackets; the inner span is only the
        /// contents of the brackets, not including surrounding whitespace.
        ///
        /// Example: `R->F` in the rotation `@U[ R->F ]`
        transform: Option<Spanned<Span>>,
    },
    /// List of nodes surrounded by `()`.
    Group {
        /// Kind of group.
        ///
        /// The span corresponds to the character before the opening `(`. For a
        /// simple group, this has an empty span.
        kind: Spanned<GroupKind>,
        /// Nodes inside the group.
        contents: Spanned<NodeList>,
    },
    /// Two lists of nodes surrounded by `[]` with a symbol between them. This
    /// is used for conjugate & commutator notation.
    BinaryGroup {
        /// Kind of group.
        ///
        /// The span corresponds to the `,` or `:` separating the two lists.
        kind: Spanned<BinaryGroupKind>,
        /// Nodes inside each half of the group.
        contents: [NodeList; 2],
    },
}
impl RepeatableNode {
    /// Converts to [`unspanned::RepeatableNode`].
    pub fn to_unspanned(&self, source: &str) -> unspanned::RepeatableNode {
        match self {
            RepeatableNode::Move {
                layers,
                family,
                transform,
            } => unspanned::RepeatableNode::Move(unspanned::Move {
                layers: layers.to_unspanned(),
                rot: unspanned::Rotation {
                    family: src(source, *family),
                    transform: transform.map(|s| src(source, *s)),
                },
            }),
            RepeatableNode::Rotation {
                at_sign: _,
                family,
                transform,
            } => unspanned::RepeatableNode::Rotation(unspanned::Rotation {
                family: src(source, *family),
                transform: transform.map(|s| src(source, *s)),
            }),
            RepeatableNode::Group { kind, contents } => unspanned::RepeatableNode::Group {
                kind: **kind,
                contents: contents.to_unspanned(source),
            },
            RepeatableNode::BinaryGroup { kind, contents } => {
                let [a, b] = &contents;
                unspanned::RepeatableNode::BinaryGroup {
                    kind: **kind,
                    contents: [a, b].map(|node_list| node_list.to_unspanned(source)),
                }
            }
        }
    }
}

/// Layer prefix for a move.
#[derive(Debug, Clone)]
pub struct LayerPrefix {
    /// Span of the preceding `~` indicating to invert the layer set, if there
    /// is one.
    pub invert: Option<Span>,
    /// Contents of the layer set, if it is nonempty.
    pub contents: Option<Spanned<LayerPrefixContents>>,
}

impl LayerPrefix {
    /// Converts to [`unspanned::LayerPrefix`].
    pub fn to_unspanned(&self) -> unspanned::LayerPrefix {
        unspanned::LayerPrefix {
            invert: self.invert.is_some(),
            contents: self.contents.as_ref().map(|c| c.to_unspanned()),
        }
    }

    /// Parses a layer prefix from a string.
    pub fn from_str(s: &str, features: LayerFeatures) -> Result<Self, Vec<ParseError<'_>>> {
        crate::parse::layer_prefix_with_features(features)
            .parse(s)
            .into_result()
    }
}

/// Contents of a layer prefix for a move.
#[derive(Debug, Clone)]
pub enum LayerPrefixContents {
    /// Single positive layer.
    ///
    /// Example: `3`
    Single(Layer),
    /// Positive layer range.
    ///
    /// Example: `3-4`
    Range([Spanned<Layer>; 2]),
    /// Layer set, which supports negative numbers.
    ///
    /// Example: `{1..-2,6}`
    Set(Vec<Spanned<LayerSetElement>>),
}

impl LayerPrefixContents {
    /// Converts to [`unspanned::LayerPrefixContents`].
    pub fn to_unspanned(&self) -> unspanned::LayerPrefixContents {
        match self {
            LayerPrefixContents::Single(i) => unspanned::LayerPrefixContents::Single(*i),
            LayerPrefixContents::Range([i, j]) => {
                unspanned::LayerPrefixContents::Range(LayerRange::new(**i, **j))
            }
            LayerPrefixContents::Set(elements) => {
                unspanned::LayerPrefixContents::Set(unspanned::LayerSet {
                    elements: elements.iter().map(|e| e.to_unspanned()).collect(),
                })
            }
        }
    }
}

/// Element of a layer set.
#[derive(Debug, Copy, Clone)]
pub enum LayerSetElement {
    /// Signed layer in a layer set.
    Single(SignedLayer),
    /// Signed layer in a layer set.
    ///
    /// Example: `1..-2`
    Range([Spanned<SignedLayer>; 2]),
}

impl LayerSetElement {
    /// Converts to [`unspanned::LayerSetElement`].
    pub fn to_unspanned(&self) -> unspanned::LayerSetElement {
        match self {
            LayerSetElement::Single(i) => unspanned::LayerSetElement::Single(*i),
            LayerSetElement::Range([i, j]) => unspanned::LayerSetElement::Range([**i, **j]),
        }
    }
}

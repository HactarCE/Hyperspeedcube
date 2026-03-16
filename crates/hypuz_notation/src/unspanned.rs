//! Notation structures without attached span information.
//!
//! Use these when you do not care about mapping notation elements to the
//! original input string, or when you want to construct notation elements from
//! scratch.

use std::fmt;
use std::ops::{Deref, DerefMut, Not};

pub use crate::common::*;
use crate::{AxisLayersInfo, Features, LayerFeatures, ParseError, Str};

/// Parses a string containing puzzle notation into a list of [`Node`]s.
pub fn parse_notation(s: &str, features: Features) -> Result<NodeList, Vec<ParseError<'_>>> {
    Ok(crate::spanned::parse_notation(s, features)?.to_unspanned(s))
}

/// Parses a string containing a single [`Node`] of puzzle notation.
pub fn parse_notation_node(s: &str, features: Features) -> Result<Node, Vec<ParseError<'_>>> {
    Ok(crate::spanned::parse_notation_node(s, features)?.to_unspanned(s))
}

/// Parses a string containing a [`LayerPrefix`].
pub fn parse_layer_prefix(
    s: &str,
    features: LayerFeatures,
) -> Result<LayerPrefix, Vec<ParseError<'_>>> {
    Ok(crate::spanned::parse_layer_prefix(s, features)?.to_unspanned())
}

/// List of notation elements.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl FromIterator<Node> for NodeList {
    fn from_iter<T: IntoIterator<Item = Node>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Invert for NodeList {
    fn inv(self) -> Result<Self, InvertError> {
        self.0.into_iter().rev().map(|n| n.inv()).collect()
    }

    fn inv_deep(self) -> Result<Self, InvertError> {
        self.0.into_iter().rev().map(|n| n.inv_deep()).collect()
    }
}

impl NodeList {
    /// Constructs a new empty node list.
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

/// Notation element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node {
    /// List of nodes surrounded by `()` with an optional multiplier.
    Group(Group),
    /// Two lists of nodes surrounded by `[]` with a symbol between them and an
    /// optional multiplier. This is used for conjugate & commutator notation.
    BinaryGroup(BinaryGroup),
    /// Rotation using `@`, such as `@U{R->F}`.
    Rotation(Rotation),
    /// Move, such as `x` or `IR2` or `{1..-1}U{R->F}`
    Move(Move),
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
            Node::Group(g) => fmt::Display::fmt(g, f),
            Node::BinaryGroup(g) => fmt::Display::fmt(g, f),
            Node::Rotation(rot) => fmt::Display::fmt(rot, f),
            Node::Move(mv) => fmt::Display::fmt(mv, f),
            Node::Pause => write!(f, "."),
            Node::Sq1Move(sq1_move) => fmt::Display::fmt(sq1_move, f),
            Node::MegaminxScrambleMove(megaminx_scramble_move) => {
                fmt::Display::fmt(megaminx_scramble_move, f)
            }
        }
    }
}

impl From<Group> for Node {
    fn from(value: Group) -> Self {
        Self::Group(value)
    }
}

impl From<BinaryGroup> for Node {
    fn from(value: BinaryGroup) -> Self {
        Self::BinaryGroup(value)
    }
}

impl From<Rotation> for Node {
    fn from(value: Rotation) -> Self {
        Self::Rotation(value)
    }
}

impl From<Move> for Node {
    fn from(value: Move) -> Self {
        Self::Move(value)
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

impl Invert for Node {
    fn inv(self) -> Result<Self, InvertError> {
        match self {
            Node::Move(mv) => Ok(Node::Move(mv.inv()?)),
            Node::Rotation(rotation) => Ok(Node::Rotation(rotation.inv()?)),
            Node::Group(g) => Ok(Node::Group(g.inv()?)),
            Node::BinaryGroup(g) => Ok(Node::BinaryGroup(g.inv()?)),
            Node::Pause => Ok(Node::Pause),
            Node::Sq1Move(sq1_move) => Ok(Node::Sq1Move(sq1_move.inv()?)),
            Node::MegaminxScrambleMove(megaminx_scramble_move) => {
                Ok(Node::MegaminxScrambleMove(megaminx_scramble_move.inv()))
            }
        }
    }

    fn inv_deep(self) -> Result<Self, InvertError> {
        match self {
            Node::Group(g) => Ok(Node::Group(g.inv_deep()?)),
            Node::BinaryGroup(g) => Ok(Node::BinaryGroup(g.inv_deep()?)),
            _ => self.inv(),
        }
    }
}

impl Node {
    /// Returns the contained [`Move`] if this is a [`Node::Move`], or returns
    /// `None` otherwise.
    pub fn into_move(self) -> Option<Move> {
        match self {
            Node::Move(mv) => Some(mv),
            _ => None,
        }
    }

    /// Returns a reference to the contained [`Move`] if this is a
    /// [`Node::Move`], or returns `None` otherwise.
    pub fn as_move(&self) -> Option<&Move> {
        match self {
            Node::Move(mv) => Some(mv),
            _ => None,
        }
    }
}

/// List of nodes surrounded by `()` with an optional multiplier.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Group {
    /// Kind of group, which is notated by an optional prefix symbol.
    pub kind: GroupKind,
    /// Nodes inside the group.
    pub contents: NodeList,
    /// Multiplier applying to the whole group.
    pub multiplier: Multiplier,
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            contents,
            multiplier,
        } = self;
        if let Some(prefix) = kind.prefix() {
            write!(f, "{prefix}")?;
        }
        write!(f, "(")?;
        fmt::Display::fmt(contents, f)?;
        write!(f, ")")?;
        fmt::Display::fmt(multiplier, f)?;
        Ok(())
    }
}

impl Invert for Group {
    fn inv(mut self) -> Result<Self, InvertError> {
        self.multiplier = self.multiplier.inv()?;
        Ok(self)
    }

    fn inv_deep(mut self) -> Result<Self, InvertError> {
        if self.kind == GroupKind::Niss {
            Err(InvertError::NissNodeCannotBeInverted)
        } else {
            self.contents = self.contents.inv_deep()?;
            Ok(self)
        }
    }
}

impl Group {
    /// Constructs a new simple group with a multiplier of 1.
    pub fn new_simple(contents: impl Into<NodeList>) -> Self {
        Self {
            kind: GroupKind::Simple,
            contents: contents.into(),
            multiplier: Multiplier(1),
        }
    }
}

/// Two lists of nodes surrounded by `[]` with a symbol between them and an
/// optional multiplier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BinaryGroup {
    /// Kind of group, which is notated by the separator symbol.
    pub kind: BinaryGroupKind,
    /// Nodes to the left of the separator symbol.
    pub lhs: NodeList,
    /// Nodes to the right of the separator symbol.
    pub rhs: NodeList,
    /// Multiplier applying to the whole group.
    pub multiplier: Multiplier,
}

impl fmt::Display for BinaryGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            lhs,
            rhs,
            multiplier,
        } = self;
        write!(f, "[")?;
        fmt::Display::fmt(lhs, f)?;
        write!(f, "{} ", kind.separator())?;
        fmt::Display::fmt(rhs, f)?;
        write!(f, "]")?;
        fmt::Display::fmt(multiplier, f)?;
        Ok(())
    }
}

impl Invert for BinaryGroup {
    fn inv(mut self) -> Result<Self, InvertError> {
        self.multiplier = self.multiplier.inv()?;
        Ok(self)
    }

    fn inv_deep(self) -> Result<Self, InvertError> {
        let [lhs, rhs] = match self.kind {
            BinaryGroupKind::Commutator => [self.rhs, self.lhs],
            BinaryGroupKind::Conjugate => [self.lhs, self.rhs.inv_deep()?],
        };
        Ok(Self {
            kind: self.kind,
            lhs,
            rhs,
            multiplier: self.multiplier,
        })
    }
}

impl BinaryGroup {
    /// Constructs a new commutator group with a multiplier of 1.
    pub fn new_commutator(lhs: impl Into<NodeList>, rhs: impl Into<NodeList>) -> Self {
        Self {
            kind: BinaryGroupKind::Commutator,
            lhs: lhs.into(),
            rhs: rhs.into(),
            multiplier: Multiplier(1),
        }
    }

    /// Constructs a new conjugate group with a multiplier of 1.
    pub fn new_conjugate(lhs: impl Into<NodeList>, rhs: impl Into<NodeList>) -> Self {
        Self {
            kind: BinaryGroupKind::Conjugate,
            lhs: lhs.into(),
            rhs: rhs.into(),
            multiplier: Multiplier(1),
        }
    }
}

/// Rotation containing a transform and an optional multiplier.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rotation {
    /// Optional move family and optional bracketed transform.
    pub transform: Transform,
    /// Multiplier.
    pub multiplier: Multiplier,
}

impl fmt::Display for Rotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            transform,
            multiplier,
        } = self;
        write!(f, "@")?;
        fmt::Display::fmt(transform, f)?;
        fmt::Display::fmt(multiplier, f)?;
        Ok(())
    }
}

impl From<Transform> for Rotation {
    fn from(value: Transform) -> Self {
        value.into_rotation()
    }
}

impl Invert for Rotation {
    fn inv(mut self) -> Result<Self, InvertError> {
        self.multiplier = self.multiplier.inv()?;
        Ok(self)
    }
}

impl Rotation {
    /// Constructs a rotation.
    pub fn new(
        family: impl Into<Str>,
        constraints: Option<ConstraintSet>,
        multiplier: impl Into<Multiplier>,
    ) -> Self {
        Self {
            transform: Transform::new(family, constraints),
            multiplier: multiplier.into(),
        }
    }
}

/// Move containing a layer prefix, a transform, and an optional multiplier.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Move {
    /// Layer prefix, which may be empty.
    ///
    /// An empty layer prefix is typically equivalent to a layer prefix that
    /// includes only layer 1.
    pub layers: LayerPrefix,
    /// Move family and optional bracketed transform.
    ///
    /// If the family is empty, then it displays as a single underscore `_`.
    pub transform: Transform,
    /// Multiplier.
    pub multiplier: Multiplier,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            layers,
            transform,
            multiplier,
        } = self;
        fmt::Display::fmt(layers, f)?;
        if transform.family.is_empty() {
            // If family name is empty, add an underscore so that at
            // least it parses back correctly.
            write!(f, "_")?;
        } else {
            fmt::Display::fmt(transform, f)?;
        }
        fmt::Display::fmt(multiplier, f)?;
        Ok(())
    }
}

impl From<Transform> for Move {
    fn from(value: Transform) -> Self {
        value.into_move(LayerPrefix::default(), 1)
    }
}

impl Invert for Move {
    fn inv(mut self) -> Result<Self, InvertError> {
        self.multiplier = self.multiplier.inv()?;
        Ok(self)
    }
}

impl Move {
    /// Constructs a move.
    pub fn new(
        layers: impl Into<LayerPrefix>,
        family: impl Into<Str>,
        constraints: Option<ConstraintSet>,
        multiplier: impl Into<Multiplier>,
    ) -> Self {
        Self {
            layers: layers.into(),
            transform: Transform::new(family, constraints),
            multiplier: multiplier.into(),
        }
    }
}

/// Transform, which may be used in a rotation or as part of a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    /// Move family, which may be empty for a rotation but must be nonempty for
    /// a transform.
    ///
    /// Example: `U` in the move `{1..-1}U{R->F, I->P}` or the rotation
    /// `@U{R->F, I->P}`
    pub family: Str,
    /// Constraint set, if present.
    ///
    /// Example: `{R->F, I->P}` in the move `{1..-1}U{R->F, I->P}` or the
    /// rotation `@U{R->F, I->P}`.
    pub constraints: Option<ConstraintSet>,
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            family,
            constraints,
        } = self;
        write!(f, "{family}")?;
        if let Some(constraints) = constraints {
            fmt::Display::fmt(constraints, f)?;
        }
        Ok(())
    }
}

impl Transform {
    /// Constructs a transform.
    pub fn new(family: impl Into<Str>, constraints: Option<ConstraintSet>) -> Self {
        Self {
            family: family.into(),
            constraints,
        }
    }

    /// Constructs a move with this transform.
    pub fn into_move(
        self,
        layers: impl Into<LayerPrefix>,
        multiplier: impl Into<Multiplier>,
    ) -> Move {
        let transform = self;
        Move {
            layers: layers.into(),
            transform,
            multiplier: multiplier.into(),
        }
    }

    /// Constructs a rotation with this transform.
    pub fn into_rotation(self) -> Rotation {
        Rotation {
            transform: self,
            multiplier: Multiplier(1),
        }
    }
}

/// Constraint set, which may be used to specify a rotation or move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConstraintSet {
    /// Constraints in the set, in arbitrary order.
    pub constraints: Box<[Constraint]>,
}

impl fmt::Display for ConstraintSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        write_separated_list(f, &self.constraints, ",")?;
        write!(f, "}}")?;
        Ok(())
    }
}

impl IntoIterator for ConstraintSet {
    type Item = Constraint;

    type IntoIter = std::vec::IntoIter<Constraint>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConstraintSet {
    type Item = &'a Constraint;

    type IntoIter = std::slice::Iter<'a, Constraint>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.iter()
    }
}

impl<C: Into<Constraint>> FromIterator<C> for ConstraintSet {
    fn from_iter<T: IntoIterator<Item = C>>(iter: T) -> Self {
        Self {
            constraints: iter.into_iter().map(|c| c.into()).collect(),
        }
    }
}

impl From<Vec<Constraint>> for ConstraintSet {
    fn from(constraints: Vec<Constraint>) -> Self {
        Self {
            constraints: constraints.into_boxed_slice(),
        }
    }
}

impl From<Box<[Constraint]>> for ConstraintSet {
    fn from(constraints: Box<[Constraint]>) -> Self {
        Self { constraints }
    }
}

/// Constraint, which may be used as part of a constraint set to specify a
/// rotation or move.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Constraint {
    /// Constraint that the transform must take one reference point to another.
    FromTo([Str; 2]),
    /// Constraint that the transform must swap two referecne points.
    Swap([Str; 2]),
    /// Constraint that the transform must keep a reference point fixed.
    Fix(Str),
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constraint::FromTo([a, b]) => {
                let arrow = if f.alternate() { "→" } else { "->" };
                write!(f, "{a}{arrow}{b}")
            }
            Constraint::Swap([a, b]) => {
                let arrow = if f.alternate() { "↔" } else { "<>" };
                write!(f, "{a}{arrow}{b}")
            }
            Constraint::Fix(a) => write!(f, "{a}"),
        }
    }
}

impl<A: Into<Str>, B: Into<Str>> From<(A, B)> for Constraint {
    fn from((a, b): (A, B)) -> Self {
        Self::FromTo([a.into(), b.into()])
    }
}

/// Layer prefix for a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl From<()> for LayerPrefix {
    fn from((): ()) -> Self {
        Self::default()
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

    /// Converts the layer prefix to a bitmask of layers.
    ///
    /// Layers beyond `layers_info.max_layers` are truncated. The returned layer
    /// mask may be empty.
    pub fn to_layer_mask(&self, layers_info: AxisLayersInfo) -> LayerMask {
        let Self { invert, contents } = self;
        let mut ret = contents
            .as_ref()
            .unwrap_or(&LayerPrefixContents::Single(Layer::SHALLOWEST))
            .to_layer_mask(layers_info);
        if *invert && let Some(all_layers) = LayerRange::all(layers_info.max_layer) {
            ret.invert_range(all_layers);
        }
        ret
    }
}

impl Not for LayerPrefix {
    type Output = Self;

    fn not(mut self) -> Self::Output {
        self.invert = !self.invert;
        self
    }
}

/// Contents of a layer prefix for a move.
///
/// This type directly corresponds to notation. When computing layer masks,
/// prefer [`LayerMask`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub fn to_ranges(&self, layers_info: AxisLayersInfo) -> Vec<LayerRange> {
        match self {
            LayerPrefixContents::Single(l) => (*l <= layers_info.max_layer)
                .then_some(LayerRange::from_layer(*l))
                .into_iter()
                .collect(),
            LayerPrefixContents::Range(range) => range
                .clamp_to_layer_count(layers_info.max_layer)
                .into_iter()
                .collect(),
            LayerPrefixContents::Set(set) => set.to_ranges(layers_info),
        }
    }

    /// Converts the layer prefix to a bitmask of layers.
    pub fn to_layer_mask(&self, layers_info: AxisLayersInfo) -> LayerMask {
        match self {
            LayerPrefixContents::Single(l) => (l.to_u16() <= layers_info.max_layer)
                .then(|| LayerMask::from_layer(*l))
                .unwrap_or_default(),
            LayerPrefixContents::Range(range) => range
                .clamp_to_layer_count(layers_info.max_layer)
                .map(LayerMask::from_range)
                .unwrap_or_default(),
            LayerPrefixContents::Set(elements) => elements.to_layer_mask(layers_info),
        }
    }

    /// Canonicalizes the layer prefix.
    ///
    /// Returns `None` if the layer prefix does not include any layers.
    pub fn simplify(&self, layers_info: AxisLayersInfo) -> Option<Self> {
        Some(Self::from_ranges(self.to_ranges(layers_info))).filter(|contents| !contents.is_empty())
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

impl From<Layer> for LayerPrefixContents {
    fn from(layer: Layer) -> Self {
        Self::Single(layer)
    }
}

impl From<LayerRange> for LayerPrefixContents {
    fn from(range: LayerRange) -> Self {
        Self::Range(range)
    }
}

impl From<SignedLayer> for LayerPrefixContents {
    fn from(layer: SignedLayer) -> Self {
        match layer.to_unsigned() {
            Some(l) => Self::Single(l),
            None => Self::Set(LayerSet::from_iter([layer])),
        }
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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
    pub fn to_ranges(&self, layers_info: AxisLayersInfo) -> Vec<LayerRange> {
        self.elements
            .iter()
            .filter_map(|elem| elem.to_range(layers_info))
            .collect()
    }

    /// Converts the layer prefix to a bitmask of layers.
    pub fn to_layer_mask(&self, layers_info: AxisLayersInfo) -> LayerMask {
        let mut ret = LayerMask::new();
        for &elem in &self.elements {
            if let Some(range) = elem.to_range(layers_info) {
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

impl<E: Into<LayerSetElement>> FromIterator<E> for LayerSet {
    fn from_iter<T: IntoIterator<Item = E>>(iter: T) -> Self {
        let elements = iter.into_iter().map(|l| l.into()).collect();
        LayerSet { elements }
    }
}

/// Element of a [`LayerSet`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl From<Layer> for LayerSetElement {
    fn from(layer: Layer) -> Self {
        Self::Single(layer.to_signed())
    }
}

impl From<SignedLayer> for LayerSetElement {
    fn from(layer: SignedLayer) -> Self {
        Self::Single(layer)
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
    fn to_range(self, layers_info: AxisLayersInfo) -> Option<LayerRange> {
        match self {
            LayerSetElement::Single(l) => layers_info.resolve(l).map(LayerRange::from_layer),
            LayerSetElement::Range(range) => layers_info.resolve_range(range),
        }
    }
}

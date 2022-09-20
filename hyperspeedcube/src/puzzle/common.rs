use cgmath::{One, Quaternion, Rotation};
use enum_iterator::Sequence;
use itertools::Itertools;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;
use std::ops::*;
use std::str::FromStr;
use strum::{Display, EnumIter, EnumMessage};

use super::*;

#[delegatable_trait]
#[enum_dispatch]
pub trait PuzzleType {
    fn ty(&self) -> PuzzleTypeEnum;
    fn name(&self) -> &str;
    fn family_display_name(&self) -> &'static str;
    fn family_internal_name(&self) -> &'static str;
    fn projection_type(&self) -> ProjectionType;

    fn layer_count(&self) -> u8;
    fn family_max_layer_count(&self) -> u8;

    fn radius(&self) -> f32;
    fn scramble_moves_count(&self) -> usize;

    fn faces(&self) -> &[FaceInfo];
    fn pieces(&self) -> &[PieceInfo];
    fn stickers(&self) -> &[StickerInfo];
    fn twist_axes(&self) -> &[TwistAxisInfo];
    fn twist_directions(&self) -> &[TwistDirectionInfo];
    fn piece_types(&self) -> &[PieceTypeInfo];

    fn twist_axis_from_name(&self, name: &str) -> Option<TwistAxis> {
        (0..self.twist_axes().len() as u8)
            .map(TwistAxis)
            .find(|&twist_axis| self.info(twist_axis).name == name)
    }
    fn twist_direction_from_name(&self, name: &str) -> Option<TwistDirection> {
        (0..self.twist_directions().len() as u8)
            .map(TwistDirection)
            .find(|&twist_direction| self.info(twist_direction).name == name)
    }
    fn opposite_twist_axis(&self, twist_axis: TwistAxis) -> Option<TwistAxis>;
    fn count_quarter_turns(&self, twist: Twist) -> usize;

    fn check_layers(&self, layers: LayerMask) -> Result<(), &'static str> {
        let layer_count = self.layer_count() as u32;
        if layers.0 > 0 || layers.0 < 1 << layer_count {
            Ok(())
        } else {
            Err("invalid layer mask")
        }
    }
    fn all_layers(&self) -> LayerMask {
        LayerMask::all_layers(self.layer_count())
    }
    fn slice_layers(&self) -> Option<LayerMask> {
        LayerMask::slice_layers(self.layer_count())
    }
    fn reverse_layers(&self, layers: LayerMask) -> LayerMask {
        LayerMask(layers.0.reverse_bits() >> (32 - self.layer_count()))
    }

    fn make_recenter_twist(&self, axis: TwistAxis) -> Result<Twist, String>;

    fn reverse_twist(&self, twist: Twist) -> Twist {
        Twist {
            axis: twist.axis,
            direction: self.reverse_twist_direction(twist.direction),
            layers: twist.layers,
        }
    }
    fn canonicalize_twist(&self, twist: Twist) -> Twist;

    fn reverse_twist_direction(&self, direction: TwistDirection) -> TwistDirection;
    fn chain_twist_directions(&self, dirs: &[TwistDirection]) -> Option<TwistDirection>;

    fn notation_scheme(&self) -> &NotationScheme;
    fn split_twists_string<'s>(&self, string: &'s str) -> regex::Matches<'static, 's> {
        const TWIST_PATTERN: &str = r"(\{[\d\s,]*\}|[^\s()])+";
        // one or more of either      (                    )+
        //     a pair of `{}`          \{        \}
        //       containing digits,      [\d   ]*
        //                  whitespace,     \s
        //                  and commas        ,
        //   or                                    |
        //     any symbol other than                [^    ]
        //       whitespace                           \s
        //       and parens                             ()

        lazy_static! {
            static ref TWIST_REGEX: Regex = Regex::new(TWIST_PATTERN).unwrap();
        }

        TWIST_REGEX.find_iter(string)
    }

    fn twist_command_short_description(
        &self,
        axis_name: Option<TwistAxis>,
        direction: TwistDirection,
        layers: LayerMask,
    ) -> String {
        match axis_name {
            Some(axis) => self
                .notation_scheme()
                .twist_to_string(self.canonicalize_twist(Twist {
                    axis,
                    direction,
                    layers,
                })),
            None => {
                let dir = self.info(direction).symbol;
                format!("{layers}Ø{dir}")
            }
        }
    }
}

trait PuzzleTypeRefExt {
    fn deref_internal(&self) -> Self;
}
#[delegate_to_methods]
#[delegate(PuzzleType, target_ref = "deref_internal")]
impl<'a, P: PuzzleType> PuzzleTypeRefExt for &'a P {
    fn deref_internal(&self) -> &'a P {
        *self
    }
}

#[enum_dispatch]
pub trait PuzzleState: PuzzleType {
    fn twist(&mut self, twist: Twist) -> Result<(), &'static str>;
    fn is_piece_affected_by_twist(&self, twist: Twist, piece: Piece) -> bool {
        twist.layers[self.layer_from_twist_axis(twist.axis, piece)]
    }
    fn pieces_affected_by_twist(&self, twist: Twist) -> Vec<Piece> {
        (0..self.pieces().len() as _)
            .map(Piece)
            .filter(|&piece| self.is_piece_affected_by_twist(twist, piece))
            .collect()
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8;

    fn rotation_candidates(&self) -> Vec<(Vec<Twist>, Quaternion<f32>)>;
    fn nearest_rotation(&self, rot: Quaternion<f32>) -> (Vec<Twist>, Quaternion<f32>) {
        let inv_rot = rot.invert();

        let mut nearest = (vec![], Quaternion::one());
        // If I understand correctly, the scalar part of a quaternion is the
        // cosine of half the angle of rotation. So we can use the absolute
        // value of that quantity to compare whether one quaternion is a larger
        // rotation than another.
        let mut score_of_nearest = rot.s.abs();
        for (twists, twist_rot) in self.rotation_candidates() {
            let s = (inv_rot * twist_rot).s.abs();

            if s > score_of_nearest {
                nearest = (twists, twist_rot);
                score_of_nearest = s;
            }
        }
        for twist in &mut nearest.0 {
            *twist = self.canonicalize_twist(*twist);
        }
        nearest
    }

    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: StickerGeometryParams,
    ) -> Option<StickerGeometry>;

    fn is_solved(&self) -> bool;

    #[cfg(debug_assertions)]
    fn sticker_debug_info(&self, _s: &mut String, _sticker: Sticker) {}
}

/// Enumeration of all puzzle types.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PuzzleTypeEnum {
    /// 3D Rubik's cube.
    Rubiks3D {
        #[serde(deserialize_with = "rubiks_3d::deserialize_layer_count")]
        layer_count: u8,
    },
    /// 4D Rubik's cube.
    Rubiks4D {
        #[serde(deserialize_with = "rubiks_4d::deserialize_layer_count")]
        layer_count: u8,
    },
}
#[delegate_to_methods]
#[delegate(PuzzleType, target_ref = "as_dyn_type")]
impl PuzzleTypeEnum {
    fn as_dyn_type(&self) -> &dyn PuzzleType {
        match *self {
            PuzzleTypeEnum::Rubiks3D { layer_count } => rubiks_3d::puzzle_type(layer_count),
            PuzzleTypeEnum::Rubiks4D { layer_count } => rubiks_4d::puzzle_type(layer_count),
        }
    }
    pub fn validate(self) -> Result<(), String> {
        match self {
            PuzzleTypeEnum::Rubiks3D { layer_count } => {
                if rubiks_3d::LAYER_COUNT_RANGE.contains(&layer_count) {
                    Ok(())
                } else {
                    Err(format!("invalid layer count {layer_count} for this puzzle"))
                }
            }
            PuzzleTypeEnum::Rubiks4D { layer_count } => {
                if rubiks_4d::LAYER_COUNT_RANGE.contains(&layer_count) {
                    Ok(())
                } else {
                    Err(format!("invalid layer count {layer_count} for this puzzle"))
                }
            }
        }
    }
}
impl Default for PuzzleTypeEnum {
    fn default() -> Self {
        Self::Rubiks4D { layer_count: 3 }
    }
}
impl fmt::Display for PuzzleTypeEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl AsRef<str> for PuzzleTypeEnum {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    pub axis: TwistAxis,
    pub direction: TwistDirection,
    pub layers: LayerMask,
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.axis.0, self.direction.0, self.layers.0)
    }
}
impl FromStr for Twist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // IIFE for to mimic `try_block`
        (|| {
            let mut segments = s.split(',');
            let axis = TwistAxis(segments.next()?.parse().ok()?);
            let direction = TwistDirection(segments.next()?.parse().ok()?);
            let layers = LayerMask(segments.next()?.parse().ok()?);
            if segments.next().is_some() {
                return None;
            }
            Some(Self {
                axis,
                direction,
                layers,
            })
        })()
        .ok_or(())
    }
}
impl Twist {
    pub fn from_rng(ty: PuzzleTypeEnum) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            axis: TwistAxis(rng.gen_range(0..ty.twist_axes().len()) as _),
            direction: TwistDirection(rng.gen_range(0..ty.twist_directions().len()) as _),
            layers: if ty.layer_count() > 1 {
                LayerMask(rng.gen_range(1..ty.all_layers().0))
            } else {
                ty.all_layers()
            },
        }
    }
}

/// Puzzle of any type.
#[enum_dispatch(PuzzleType, PuzzleState)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Puzzle {
    /// 3D Rubik's cube.
    Rubiks3D(Rubiks3D),
    /// 4D Rubik's cube.
    Rubiks4D(Rubiks4D),
}
impl Default for Puzzle {
    fn default() -> Self {
        Self::new(PuzzleTypeEnum::default())
    }
}
impl Puzzle {
    /// Creates a new puzzle of a particular type.
    pub fn new(ty: PuzzleTypeEnum) -> Puzzle {
        match ty {
            PuzzleTypeEnum::Rubiks3D { layer_count } => {
                Puzzle::Rubiks3D(Rubiks3D::new(layer_count))
            }
            PuzzleTypeEnum::Rubiks4D { layer_count } => {
                Puzzle::Rubiks4D(Rubiks4D::new(layer_count))
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Piece(pub u16);
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Sticker(pub u16);
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Face(pub u8);
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistAxis(pub u8);
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirection(pub u8);
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PieceType(pub u8);

pub trait PuzzleInfo<T> {
    type Output;

    fn info(&self, thing: T) -> &Self::Output;
}
macro_rules! impl_puzzle_info_trait {
    (fn $method:ident($thing:ty) -> &$thing_info:ty) => {
        impl<T: PuzzleType + ?Sized> PuzzleInfo<$thing> for T {
            type Output = $thing_info;

            fn info(&self, thing: $thing) -> &$thing_info {
                &self.$method()[thing.0 as usize]
            }
        }
    };
}
impl_puzzle_info_trait!(fn faces(Face) -> &FaceInfo);
impl_puzzle_info_trait!(fn pieces(Piece) -> &PieceInfo);
impl_puzzle_info_trait!(fn stickers(Sticker) -> &StickerInfo);
impl_puzzle_info_trait!(fn twist_axes(TwistAxis) -> &TwistAxisInfo);
impl_puzzle_info_trait!(fn twist_directions(TwistDirection) -> &TwistDirectionInfo);
impl_puzzle_info_trait!(fn piece_types(PieceType) -> &PieceTypeInfo);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceInfo {
    pub stickers: SmallVec<[Sticker; 8]>,
    pub piece_type: PieceType,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StickerInfo {
    pub piece: Piece,
    pub color: Face,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FaceInfo {
    pub symbol: &'static str, // e.g., "R"
    pub name: &'static str,   // e.g., "Right"
}
impl FaceInfo {
    pub const fn new(symbol: &'static str, name: &'static str) -> Self {
        Self { symbol, name }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistAxisInfo {
    pub name: &'static str, // e.g., "R"
}
impl AsRef<str> for TwistAxisInfo {
    fn as_ref(&self) -> &str {
        self.name
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TwistDirectionInfo {
    pub symbol: &'static str, // "'"
    pub name: &'static str,   // "CCW"
}
impl AsRef<str> for TwistDirectionInfo {
    fn as_ref(&self) -> &str {
        self.name
    }
}
impl TwistDirectionInfo {
    pub const fn new(symbol: &'static str, name: &'static str) -> Self {
        Self { symbol, name }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceTypeInfo {
    pub name: String,
}
impl AsRef<str> for PieceTypeInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl PieceTypeInfo {
    pub const fn new(name: String) -> Self {
        Self { name }
    }
}

/// Convention for counting moves.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Display,
    EnumIter,
    EnumMessage,
    Sequence,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum TwistMetric {
    #[strum(serialize = "ATM", message = "Axial Turn Metric")]
    Atm,
    #[strum(serialize = "ETM", message = "Execution Turn Metric")]
    Etm,

    #[default]
    #[strum(serialize = "STM", message = "Slice Turn Metric (default)")]
    Stm,
    #[strum(serialize = "BTM", message = "Block Turn Metric")]
    Btm,
    #[strum(serialize = "OBTM", message = "Outer Block Turn Metric")]
    Obtm,

    #[strum(serialize = "QSTM", message = "Quarter Slice Turn Metric")]
    Qstm,
    #[strum(serialize = "QBTM", message = "Quarter Block Turn Metric")]
    Qbtm,
    #[strum(serialize = "QOBTM", message = "Quarter Outer Block Turn Metric")]
    Qobtm,
}
impl TwistMetric {
    pub fn long_description(self) -> String {
        let mut bullets = vec![];

        if self == Self::Atm {
            bullets.push(
                "Consecutive twists of the same axis are combined, even with different layers.",
            );
        }
        if self == Self::Etm {
            bullets
                .push("Twists are counted as they are executed, including whole-puzzle rotations.");
        } else {
            bullets.push("Whole-puzzle rotations are not counted.");
        }
        match self {
            Self::Stm | Self::Qstm => bullets.push("Slice twists count as one move."),
            Self::Btm | Self::Qbtm => {
                bullets.push("Noncontiguous slice twists are split into contiguous slice twists.")
            }
            Self::Obtm | Self::Qobtm => {
                bullets.push("Slice twists are split into contiguous outer-block twists.")
            }
            _ => (),
        }
        match self.is_qtm() {
            Some(false) => {
                bullets.push("Consecutive twists of the same axis and layers are combined.")
            }
            Some(true) => bullets.push("Double twists are split into quarters."),
            None => (),
        }

        bullets.into_iter().map(|s| format!("• {s}")).join("\n")
    }

    pub fn is_qtm(self) -> Option<bool> {
        match self {
            Self::Atm | Self::Etm => None,
            Self::Stm | Self::Btm | Self::Obtm => Some(false),
            Self::Qstm | Self::Qbtm | Self::Qobtm => Some(true),
        }
    }
    pub fn set_qtm(&mut self, is_qtm: bool) {
        *self = match self {
            Self::Stm | Self::Qstm => {
                if is_qtm {
                    Self::Qstm
                } else {
                    Self::Stm
                }
            }
            Self::Btm | Self::Qbtm => {
                if is_qtm {
                    Self::Qbtm
                } else {
                    Self::Btm
                }
            }
            Self::Obtm | Self::Qobtm => {
                if is_qtm {
                    Self::Qobtm
                } else {
                    Self::Obtm
                }
            }
            _ => *self,
        };
    }

    /// Counts a sequence of twists using this metric.
    pub fn count_twists(
        self,
        puzzle: impl PuzzleType,
        twists: impl IntoIterator<Item = Twist>,
    ) -> usize {
        #[allow(clippy::needless_late_init)]
        let slice_multiplier: fn(LayerMask, u8) -> u32;

        match self {
            Self::Atm => {
                let mut count = 0;

                let mut prev_axis = None;
                for twist in twists {
                    let opp = puzzle.opposite_twist_axis(twist.axis);
                    let is_same_axis =
                        prev_axis == Some(twist.axis) || opp.is_some() && prev_axis == opp;
                    if !is_same_axis {
                        if twist.layers == puzzle.all_layers() {
                            prev_axis = None;
                        } else {
                            count += 1;
                            prev_axis = Some(twist.axis);
                        }
                    }
                }

                return count;
            }
            Self::Etm => return twists.into_iter().count(),

            Self::Stm | Self::Qstm => slice_multiplier = |_, _| 1,
            Self::Btm | Self::Qbtm => {
                slice_multiplier = |layers, _| layers.count_contiguous_slices()
            }
            Self::Obtm | Self::Qobtm => slice_multiplier = LayerMask::count_outer_slices,
        }

        let is_qtm = self.is_qtm().unwrap();

        let mut count = 0;

        let mut prev_axis = None;
        let mut prev_layers = None;
        for twist in twists {
            if twist.layers == puzzle.all_layers() {
                let opp = puzzle.opposite_twist_axis(twist.axis);
                let is_same_axis =
                    prev_axis == Some(twist.axis) || opp.is_some() && prev_axis == opp;
                if !is_same_axis {
                    // Axes may have shifted around, so clear them.
                    prev_axis = None;
                    prev_layers = None;
                }
                // Don't count full-puzzle rotations.
                continue;
            }

            let direction_multiplier = if is_qtm {
                puzzle.count_quarter_turns(twist)
            } else if prev_axis == Some(twist.axis) && prev_layers == Some(twist.layers) {
                // Same axis and layers as previous twist! This twist is
                // free.
                0
            } else {
                1
            };

            prev_axis = Some(twist.axis);
            prev_layers = Some(twist.layers);

            count += direction_multiplier
                * slice_multiplier(twist.layers, puzzle.layer_count()) as usize;
        }

        count
    }
}

/// Positive or negative.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sign {
    /// Positive.
    Pos = 1,
    /// Negative.
    Neg = -1,
}
impl Neg for Sign {
    type Output = Sign;
    fn neg(self) -> Sign {
        match self {
            Sign::Pos => Sign::Neg,
            Sign::Neg => Sign::Pos,
        }
    }
}
impl Mul<Sign> for Sign {
    type Output = Sign;
    fn mul(self, rhs: Sign) -> Sign {
        match self {
            Sign::Pos => rhs,
            Sign::Neg => -rhs,
        }
    }
}
impl Sign {
    /// Returns an integer representation of the sign (either -1 or 1).
    pub const fn int(self) -> i8 {
        match self {
            Sign::Pos => 1,
            Sign::Neg => -1,
        }
    }
    /// Returns a floating-point representation of the sign (either -1.0 or
    /// 1.0).
    pub const fn float(self) -> f32 {
        self.int() as f32
    }
    /// Returns an iterator over all signs.
    pub fn iter() -> impl Clone + Iterator<Item = Sign> {
        [Sign::Pos, Sign::Neg].into_iter()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProjectionType {
    _3D,
    _4D,
}

/// Bitmask selecting a subset of a puzzle's layers.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct LayerMask(pub u32);
impl Default for LayerMask {
    fn default() -> Self {
        Self(1)
    }
}
impl From<RangeInclusive<u8>> for LayerMask {
    fn from(range: RangeInclusive<u8>) -> Self {
        let mut lo = *range.start();
        let mut hi = std::cmp::min(*range.end(), 31);
        if lo > hi {
            std::mem::swap(&mut lo, &mut hi);
        }
        let count = hi - lo + 1;
        Self(((1 << count) - 1) << lo)
    }
}
impl Index<u8> for LayerMask {
    type Output = bool;

    fn index(&self, index: u8) -> &Self::Output {
        match self.0 & (1 << index) {
            0 => &false,
            _ => &true,
        }
    }
}
impl Not for LayerMask {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
macro_rules! impl_layer_mask_op {
    (
        impl $op_trait:ident, $op_assign_trait:ident;
        fn $op_fn:ident, $op_assign_fn:ident
    ) => {
        impl $op_trait for LayerMask {
            type Output = Self;

            fn $op_fn(self, rhs: Self) -> Self::Output {
                Self(self.0.$op_fn(rhs.0))
            }
        }
        impl $op_assign_trait for LayerMask {
            fn $op_assign_fn(&mut self, rhs: Self) {
                self.0.$op_assign_fn(rhs.0)
            }
        }
    };
}
impl_layer_mask_op!(impl BitOr, BitOrAssign; fn bitor, bitor_assign);
impl_layer_mask_op!(impl BitAnd, BitAndAssign; fn bitand, bitand_assign);
impl_layer_mask_op!(impl BitXor, BitXorAssign; fn bitxor, bitxor_assign);
impl fmt::Display for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_default() {
            Ok(())
        } else if let Some(l) = self.get_single_layer() {
            write!(f, "{}", l + 1)
        } else {
            write!(f, "{{")?;

            let mut first = true;

            let mut mask = self.0;
            let mut offset = 1;
            while mask != 0 {
                if first {
                    first = false;
                } else {
                    write!(f, ",")?;
                }

                let lo = offset + mask.trailing_zeros();
                offset += mask.trailing_zeros();
                mask >>= mask.trailing_zeros();

                let hi = lo + mask.trailing_ones() - 1;
                offset += mask.trailing_ones();
                mask >>= mask.trailing_ones();

                write!(f, "{lo}")?;
                if lo != hi {
                    write!(f, "-{hi}")?;
                }
            }

            write!(f, "}}")?;
            Ok(())
        }
    }
}
impl FromStr for LayerMask {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // IIFE
        (|| {
            if s.trim().starts_with('{') {
                s.trim()
                    .strip_prefix('{')?
                    .strip_suffix('}')?
                    .split(',')
                    .map(|s| match s.trim().split_once('-') {
                        // Range notation; e.g., "3-6"
                        Some((lo, hi)) => {
                            let lo = lo.trim().parse::<u8>().ok()? - 1;
                            let hi = hi.trim().parse::<u8>().ok()? - 1;
                            Some(Self::from(lo..=hi))
                        }
                        // Single layer notation; e.g., "3"
                        None => Some(Self(1 << (s.trim().parse::<u8>().ok()? - 1))),
                    })
                    .try_fold(Self(0), |a, b| Some(a | b?))
            } else {
                Some(Self(1 << (s.trim().parse::<u8>().ok()? - 1)))
            }
        })()
        .ok_or("invalid layer mask")
    }
}
impl LayerMask {
    pub(crate) fn slice_layers(total_layer_count: u8) -> Option<Self> {
        (total_layer_count >= 3).then(|| Self((Self::all_layers(total_layer_count).0 >> 1) & !1))
    }
    pub(crate) fn all_layers(total_layer_count: u8) -> Self {
        Self((1 << total_layer_count as u32) - 1)
    }

    pub(crate) fn is_default(self) -> bool {
        self == Self::default()
    }
    pub(crate) fn long_description(self) -> String {
        match self.count() {
            0 => "no layers".to_owned(),
            1 => format!("layer {}", self.0.trailing_zeros() + 1),
            _ => format!(
                "layers {}",
                (0..32).filter(|&i| self[i]).map(|i| i + 1).join(", ")
            ),
        }
    }
    pub(crate) fn count(self) -> u32 {
        self.0.count_ones()
    }
    pub(crate) fn count_contiguous_slices(self) -> u32 {
        let mut n = self.0;
        let mut ret = 0;
        while n != 0 {
            n >>= n.trailing_zeros();
            n >>= n.trailing_ones();
            ret += 1;
        }
        ret
    }
    pub(crate) fn count_outer_slices(self, layer_count: u8) -> u32 {
        let mut n = self.0;
        let mut ret = 0;
        while n != 0 {
            match n & 1 {
                0 => n >>= n.trailing_zeros(),
                1 => n >>= n.trailing_ones(),
                _ => unreachable!(),
            }
            ret += 1;
        }
        if self[layer_count - 1] {
            ret -= 1;
        }
        ret
    }
    pub(crate) fn is_contiguous_from_outermost(self) -> bool {
        self.0 != 0 && self.0.count_ones() == self.0.trailing_ones()
    }
    pub(crate) fn get_single_layer(self) -> Option<u32> {
        (self.count() == 1).then(|| self.0.trailing_zeros())
    }
}

/// Twists for the hovered sticker.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ClickTwists {
    /// Clockwise twist, typically bound to left click.
    pub cw: Option<Twist>,
    /// Counterclockwise twist, typically bound to right click.
    pub ccw: Option<Twist>,
    /// Recenter twist, typically bound to middle click.
    pub recenter: Option<Twist>,
}
impl ClickTwists {
    #[must_use]
    pub fn rev(self) -> Self {
        Self {
            cw: self.ccw,
            ccw: self.cw,
            ..self
        }
    }
}

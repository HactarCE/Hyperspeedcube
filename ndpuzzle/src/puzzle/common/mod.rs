use enum_iterator::Sequence;
use itertools::Itertools;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::ops::*;
use std::str::FromStr;
use std::sync::{Arc, Weak};
use strum::{Display, EnumIter, EnumMessage};

#[macro_use]
mod info;
pub mod geometry;
mod notation;
mod shape;
mod twists;

pub use geometry::{ProjectedStickerGeometry, StickerGeometry, StickerGeometryParams};
pub use info::*;
pub use notation::*;
pub use shape::*;
pub use twists::*;

use crate::math::Matrix;

/// Puzzle type metadata.
pub struct PuzzleType {
    pub this: Weak<PuzzleType>,
    pub name: String,
    pub shape: Arc<PuzzleShape>,
    pub twists: Arc<PuzzleTwists>,

    pub family_name: String,             // TODO: remove
    pub projection_type: ProjectionType, // TODO: remove
    pub layer_count: u8,                 // TODO: remove

    pub pieces: Vec<PieceInfo>,
    pub stickers: Vec<StickerInfo>,
    pub piece_types: Vec<PieceTypeInfo>,

    pub scramble_moves_count: usize,

    pub notation: NotationScheme,

    pub new: Box<dyn Send + Sync + Fn(Arc<PuzzleType>) -> Box<dyn PuzzleState>>,
}
impl fmt::Debug for PuzzleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PuzzleType")
            .field("this", &self.this)
            .field("name", &self.name)
            .field("shape", &self.shape)
            .field("twists", &self.twists)
            .field("family_name", &self.family_name)
            .field("projection_type", &self.projection_type)
            .field("pieces", &self.pieces)
            .field("stickers", &self.stickers)
            .field("piece_types", &self.piece_types)
            .field("scramble_moves_count", &self.scramble_moves_count)
            .field("notation", &self.notation)
            .finish()
    }
}
impl fmt::Display for PuzzleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
impl Hash for PuzzleType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl AsRef<str> for PuzzleType {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl_puzzle_info_trait!(for PuzzleType { fn info(Facet) -> &FacetInfo { .shape.facets } });
impl_puzzle_info_trait!(for PuzzleType { fn info(TwistAxis) -> &TwistAxisInfo { .twists.axes } });
impl_puzzle_info_trait!(for PuzzleType { fn info(TwistDirection) -> &TwistDirectionInfo { .twists.directions } });
impl_puzzle_info_trait!(for PuzzleType { fn info(Piece) -> &PieceInfo { .pieces } });
impl_puzzle_info_trait!(for PuzzleType { fn info(Sticker) -> &StickerInfo { .stickers } });
impl_puzzle_info_trait!(for PuzzleType { fn info(PieceType) -> &PieceTypeInfo { .piece_types } });
impl PuzzleType {
    pub fn new(&self) -> Box<dyn PuzzleState> {
        (self.new)(self.arc())
    }
    pub fn arc(&self) -> Arc<Self> {
        self.this
            .upgrade()
            .expect("unable to promote Weak<PuzzleType> to Arc<PuzzleType>")
    }

    pub fn check_layers(&self, layers: LayerMask) -> Result<(), &'static str> {
        let layer_count = self.layer_count as u32;
        if layers.0 > 0 || layers.0 < 1 << layer_count {
            Ok(())
        } else {
            Err("invalid layer mask")
        }
    }
    pub fn all_layers(&self) -> LayerMask {
        LayerMask::all_layers(self.layer_count)
    }
    pub fn slice_layers(&self) -> Option<LayerMask> {
        LayerMask::slice_layers(self.layer_count)
    }
    pub fn reverse_layers(&self, layers: LayerMask) -> LayerMask {
        LayerMask(layers.0.reverse_bits() >> (32 - self.layer_count))
    }

    pub fn reverse_twist(&self, twist: Twist) -> Twist {
        Twist {
            axis: twist.axis,
            direction: self.info(twist.direction).rev,
            layers: twist.layers,
        }
    }
    pub fn canonicalize_twist(&self, twist: Twist) -> Twist {
        if let Some((opp_axis, rev_dir)) = self.info(twist.axis).opposite_twist(twist.direction) {
            let opposite_twist = Twist {
                axis: opp_axis,
                direction: rev_dir,
                layers: self.reverse_layers(twist.layers),
            };
            std::cmp::min(twist, opposite_twist)
        } else {
            twist
        }
    }

    pub fn split_twists_string<'s>(&self, string: &'s str) -> regex::Matches<'static, 's> {
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

    pub fn twist_command_short_description(
        &self,
        axis_name: Option<TwistAxis>,
        direction: TwistDirection,
        layers: LayerMask,
    ) -> String {
        match axis_name {
            Some(axis) => self
                .notation
                .twist_to_string(self.canonicalize_twist(Twist {
                    axis,
                    direction,
                    layers,
                })),
            None => {
                let dir = &self.info(direction).symbol;
                format!("{layers}Ø{dir}")
            }
        }
    }
}

pub trait PuzzleState: fmt::Debug + Send + Sync {
    fn ty(&self) -> &Arc<PuzzleType>;

    fn clone_boxed(&self) -> Box<dyn PuzzleState>;

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str>;
    fn is_piece_affected_by_twist(&self, twist: Twist, piece: Piece) -> bool {
        twist.layers[self.layer_from_twist_axis(twist.axis, piece)]
    }
    fn pieces_affected_by_twist(&self, twist: Twist) -> Vec<Piece> {
        (0..self.ty().pieces.len() as _)
            .map(Piece)
            .filter(|&piece| self.is_piece_affected_by_twist(twist, piece))
            .collect()
    }
    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        // TODO: handle bandaging
        match self.ty().info(piece).points.first() {
            Some(point) => {
                let axis_info = self.ty().info(twist_axis);
                let t = axis_info.reference_frame.reverse().matrix() * self.piece_transform(piece);
                let relative_point = t * point;
                // TODO: this is O(n)
                axis_info
                    .cuts
                    .iter()
                    .take_while(|cut| !cut.is_point_above(&relative_point))
                    .count() as u8
            }
            None => 0, // TODO: wrong
        }
    }

    fn piece_transform(&self, p: Piece) -> Matrix;
    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: &StickerGeometryParams,
    ) -> Option<StickerGeometry> {
        let sticker_info = self.ty().info(sticker);
        let facet_info = self.ty().info(sticker_info.color);
        let piece_transform = self.piece_transform(sticker_info.piece);
        let pole = &facet_info.pole;
        Some(StickerGeometry {
            verts: sticker_info
                .points
                .iter()
                // Apply facet shrink.
                .map(|point| (point - &pole) * p.facet_scale + &pole)
                // Apply piece transform.
                .map(|point| &piece_transform * point)
                // Project from 4D into 3D.
                .map(|point| p.project_4d(point))
                .collect::<Option<_>>()?,
            polygon_indices: sticker_info
                .polygons
                .iter()
                .map(|indices| indices.clone().into_boxed_slice())
                .collect(),
            polygon_twists: vec![ClickTwists::default(); sticker_info.polygons.len()],
        })
    }

    fn is_solved(&self) -> bool;

    #[cfg(debug_assertions)]
    fn sticker_debug_info(&self, _s: &mut String, _sticker: Sticker) {}
}
impl Clone for Box<dyn PuzzleState> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}
impl<T: PuzzleState> PuzzleState for Box<T> {
    fn ty(&self) -> &Arc<PuzzleType> {
        (**self).ty()
    }

    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
        (**self).clone_boxed()
    }

    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
        (**self).twist(twist)
    }

    fn layer_from_twist_axis(&self, twist_axis: TwistAxis, piece: Piece) -> u8 {
        (**self).layer_from_twist_axis(twist_axis, piece)
    }

    fn piece_transform(&self, p: Piece) -> Matrix {
        (**self).piece_transform(p)
    }
    fn sticker_geometry(
        &self,
        sticker: Sticker,
        p: &StickerGeometryParams,
    ) -> Option<StickerGeometry> {
        (**self).sticker_geometry(sticker, p)
    }

    fn is_solved(&self) -> bool {
        (**self).is_solved()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Twist {
    pub axis: TwistAxis,
    pub direction: TwistDirection,
    pub layers: LayerMask,
}
impl PartialOrd for Twist {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Twist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Prefer smaller layer mask; otherwise prefer smaller axis; otherwise
        // prefer smaller direction.
        (self.layers, self.axis, self.direction).cmp(&(other.layers, other.axis, other.direction))
    }
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.axis.0, self.direction.0, self.layers.0)
    }
}
impl FromStr for Twist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // IIFE to mimic `try_block`
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
    pub fn from_rng(ty: &PuzzleType) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            axis: TwistAxis(rng.gen_range(0..ty.twists.axes.len()) as _),
            direction: TwistDirection(rng.gen_range(0..ty.twists.directions.len()) as _),
            layers: if ty.layer_count > 1 {
                LayerMask(rng.gen_range(1..ty.all_layers().0))
            } else {
                ty.all_layers()
            },
        }
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
        puzzle: &PuzzleType,
        twists: impl IntoIterator<Item = Twist>,
    ) -> usize {
        #[allow(clippy::needless_late_init)]
        let slice_multiplier: fn(LayerMask, u8) -> u32;

        match self {
            Self::Atm => {
                let mut count = 0;

                let mut prev_axis = None;
                for twist in twists {
                    let opp = puzzle.info(twist.axis).opposite_axis();
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
                let opp = puzzle.info(twist.axis).opposite_axis();
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
                puzzle.info(twist.direction).qtm
            } else if prev_axis == Some(twist.axis) && prev_layers == Some(twist.layers) {
                // Same axis and layers as previous twist! This twist is
                // free.
                0
            } else {
                1
            };

            prev_axis = Some(twist.axis);
            prev_layers = Some(twist.layers);

            count +=
                direction_multiplier * slice_multiplier(twist.layers, puzzle.layer_count) as usize;
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
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    pub fn slice_layers(total_layer_count: u8) -> Option<Self> {
        (total_layer_count >= 3).then(|| Self((Self::all_layers(total_layer_count).0 >> 1) & !1))
    }
    pub fn all_layers(total_layer_count: u8) -> Self {
        Self((1 << total_layer_count as u32) - 1)
    }

    pub fn is_default(self) -> bool {
        self == Self::default()
    }
    pub fn long_description(self) -> String {
        match self.count() {
            0 => "no layers".to_owned(),
            1 => format!("layer {}", self.0.trailing_zeros() + 1),
            _ => format!(
                "layers {}",
                (0..32).filter(|&i| self[i]).map(|i| i + 1).join(", ")
            ),
        }
    }
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
    pub fn count_contiguous_slices(self) -> u32 {
        let mut n = self.0;
        let mut ret = 0;
        while n != 0 {
            n >>= n.trailing_zeros();
            n >>= n.trailing_ones();
            ret += 1;
        }
        ret
    }
    pub fn count_outer_slices(self, layer_count: u8) -> u32 {
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
    pub fn is_contiguous_from_outermost(self) -> bool {
        self.0 != 0 && self.0.count_ones() == self.0.trailing_ones()
    }
    pub fn get_single_layer(self) -> Option<u32> {
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

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::ops::*;
use std::str::FromStr;

idx_struct! {
    /// Layer ID, not to be confused with a _layer mask_, which is a bitmask
    /// where each bit corresponds to a layer ID.
    pub struct LayerId(pub u8);
}

/// Bitmask selecting a subset of a puzzle's layers.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct LayerMask(pub crate::LayerMaskUint);
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
    /// Returns a mask containing all layers.
    pub fn all_layers(total_layer_count: u8) -> Self {
        Self((1 << total_layer_count as u32) - 1)
    }

    /// Returns whether the layer mask is the default, which contains only the outermost layer.
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
    /// Returns a human-friendly description of the layer mask.
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
    /// Returns the number of selected layers.
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
    /// Returns the number of contiguous blocks of 1s.
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
    /// Returns the number of contiguous blocks of 0s and 1s from the outermost
    /// layer.
    pub fn count_outer_blocks(self, layer_count: u8) -> u32 {
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
    /// Returns whether a layer mask consists of one contiguous block of 1s
    /// containing the outermost layer.
    pub fn is_contiguous_from_outermost(self) -> bool {
        self.0 != 0 && self.0.count_ones() == self.0.trailing_ones()
    }
    /// Returns the single layer in the layer mask, or `None` if there is not
    /// exactly one layer.
    pub fn get_single_layer(self) -> Option<u32> {
        (self.count() == 1).then(|| self.0.trailing_zeros())
    }
}

use std::collections::{btree_map, BTreeMap};

use serde::{Deserialize, Serialize};

pub mod v2;

pub use v2 as current;

pub const CURRENT_VERSION: &str = "v2";

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "version")]
pub enum AnyVersion {
    #[serde(rename = "v2")]
    V2(Box<v2::Preferences>),
}
impl AnyVersion {
    pub fn into_current(self) -> current::Preferences {
        match self {
            AnyVersion::V2(p) => *p,
            // for future versions, migrate past versions one step forward
        }
    }
}

/// Data stored in preferences that must be converted to a slightly different
/// format before being saved to disk.
pub trait PrefsConvert: Default {
    type DeserContext;
    type SerdeFormat;

    /// Converts data from the in-memory format to the serde-compatible file
    /// format.
    fn to_serde(&self) -> Self::SerdeFormat;
    /// Converts data from the serde-compatible file format to the in-memory
    /// format.
    fn from_serde(ctx: &Self::DeserContext, value: Self::SerdeFormat) -> Self {
        let mut ret = Self::default();
        ret.reload_from_serde(ctx, value);
        ret
    }
    /// Updates data in-place from the serde-compatible file format, preserving
    /// references.
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat);
}
impl<'de, T: Default + Serialize + Deserialize<'de> + Clone> PrefsConvert for T {
    type DeserContext = ();
    type SerdeFormat = Self;

    fn to_serde(&self) -> Self {
        self.clone()
    }
    fn reload_from_serde(&mut self, _ctx: &(), value: Self) {
        *self = value;
    }
}

pub(crate) fn reload_btreemap<K: Ord, V: PrefsConvert>(
    old: &mut BTreeMap<K, V>,
    ctx: &V::DeserContext,
    new: BTreeMap<K, V::SerdeFormat>,
) {
    old.retain(|k, _| new.contains_key(k));
    for (k, v) in new {
        match old.entry(k) {
            btree_map::Entry::Vacant(e) => {
                e.insert(V::from_serde(ctx, v));
            }
            btree_map::Entry::Occupied(e) => {
                e.into_mut().reload_from_serde(ctx, v);
            }
        }
    }
}

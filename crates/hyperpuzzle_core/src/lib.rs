//! Multidimensional twisty puzzle generator and simulator backend.

#[macro_use]
extern crate lazy_static; // TODO: replace with std::sync::Lazy

#[cfg(test)]
use criterion as _; // Suppress unused crate warning (it's used in a benchmark)

#[macro_use]
pub mod util;
pub mod catalog;
mod components;
mod lint;
mod logging;
mod names;
mod puzzle;
mod rgb;
pub mod tags;
mod timestamp;
mod traits;
mod version;

/// Re-export of `chrono`.
pub use chrono;
pub use components::{Component, ComponentList, MissingComponent};
/// Re-export of `hypergroup`
pub use hypergroup as group;
/// Re-export of `hypershape`.
pub use hypershape;
/// Re-export of `hyperspeedcube_cli_types`.
pub use hyperspeedcube_cli_types::*;
/// Re-export of `hypuz_notation`.
pub use hypuz_notation as notation;
/// Re-export of `hypuz_util`.
pub use hypuz_util::*;
pub use prelude::*;
pub use tags::{AllTags, TAGS};

pub use crate::logging::*;
pub use crate::names::{
    AutoNames, is_name_spec_valid, name_spec_matches_name, preferred_name_from_name_spec,
};
pub use crate::rgb::Rgb;
pub use crate::timestamp::Timestamp;

/// Prelude of common imports.
pub mod prelude {
    pub use crate::catalog::*;
    pub use crate::lint::PuzzleLintOutput;
    pub use crate::names::{
        AutoNames, BadName, NameSpec, NameSpecBiMap, NameSpecBiMapBuilder, NameSpecMap,
        StringBiMap, StringBiMapBuilder,
    };
    pub use crate::notation::{
        self, AxisLayersInfo, Layer, LayerMask, LayerPrefix, LayerRange, Move, Multiplier,
    };
    pub use crate::puzzle::*; // TODO: narrow this down (remove standalone functions)
    pub use crate::tags::{TagData, TagDisplay, TagMenuNode, TagSet, TagType, TagValue};
    pub use crate::ti::*;
    pub use crate::traits::*;
    pub use crate::version::Version;
}

/// Version string such as `hyperpuzzle v1.2.3`.
pub const PUZZLE_ENGINE_VERSION_STRING: &str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

/// Default length for a full scramble.
///
/// **Changing this will break scramble compatibility for most puzzles.**
pub const FULL_SCRAMBLE_LENGTH: u32 = 1000;

/// Name of the default vantage group, if no name is specified.
pub const DEFAULT_VANTAGE_GROUP_NAME: &str = "Main";
/// Name of the default color scheme, if no other is specified.
pub const DEFAULT_COLOR_SCHEME_NAME: &str = "Main";
/// Name of the default gradient, to which unknown or conflicting colors are
/// assigned.
pub const DEFAULT_COLOR_GRADIENT_NAME: &str = "Rainbow";

/// Returns the randomness chain used for generating scrambles.
#[cfg(feature = "timecheck")]
pub fn get_drand_chain() -> timecheck::drand::Chain {
    timecheck::drand::Chain::quicknet()
}

/// Maximum number of ID redirects.
const MAX_ID_REDIRECTS: usize = 15;

/// **This function is deprecated after 2.0.0-zeta.12.**
///
/// Parses the ID of a generated object into its constituent parts: the
/// generator ID, and the parameters. Returns `None` if the ID is not a valid ID
/// for a generated object.
#[deprecated = "use `CatalogId` instead"]
pub fn zeta12_parse_generated_id(id: &str) -> Option<(&str, Vec<&str>)> {
    let (generator_id, args) = id.split_once(':')?;
    Some((generator_id, args.split(',').collect()))
}

/// **This function is deprecated after 2.0.0-zeta.12.**
///
/// Returns the ID of a generated object.
#[deprecated = "use `CatalogId` instead"]
pub fn zeta12_generated_id(
    generator_id: &str,
    params: impl IntoIterator<Item = impl ToString>,
) -> String {
    let mut ret = generator_id.to_owned();
    let mut is_first = true;
    for param in params {
        ret += if is_first { ":" } else { "," };
        is_first = false;
        ret += &param.to_string();
    }
    ret
}

const AD_HOC_ID_STR: &str = "ad_hoc";
const ROT_ID_STR: &str = "rot";
const REFL_ID_STR: &str = "rot";

/// Returns the ID for an ad-hoc color system or twist system.
pub fn ad_hoc_id(puzzle_id: CatalogId) -> CatalogId {
    CatalogId {
        base: AD_HOC_ID_STR.parse().expect("bad ID"),
        args: Some(vec![puzzle_id.into()]),
        subset: None,
    }
}

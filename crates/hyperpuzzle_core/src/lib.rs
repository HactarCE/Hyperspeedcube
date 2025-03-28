//! Multidimensional twisty puzzle generator and simulator backend.

#[macro_use]
extern crate lazy_static;

pub mod catalog;
mod lint;
mod logging;
mod nameable;
mod puzzle;
mod rgb;
pub mod tags;
mod timestamp;
mod traits;
pub mod util;
mod version;

/// Re-export of `chrono`.
pub use chrono;
pub use prelude::*;
pub use tags::{AllTags, TAGS};

pub use crate::logging::*;
pub use crate::rgb::Rgb;
pub use crate::timestamp::Timestamp;

/// Prelude of common imports.
pub mod prelude {
    pub use crate::LayerMaskUint;
    pub use crate::catalog::{
        Catalog, ColorSystemCatalog, ColorSystemGenerator, GeneratorParam, GeneratorParamError,
        GeneratorParamType, GeneratorParamValue, PuzzleCatalog, PuzzleListMetadata, PuzzleSpec,
        PuzzleSpecGenerator, Redirectable,
    };
    pub use crate::lint::PuzzleLintOutput;
    pub use crate::nameable::Nameable;
    pub use crate::puzzle::*; // TODO: narrow this down (remove standalone functions)
    pub use crate::tags::{TagData, TagDisplay, TagMenuNode, TagSet, TagType, TagValue};
    pub use crate::traits::*;
    pub use crate::version::Version;
}

/// Unsigned integer type used for [`LayerMask`].
pub type LayerMaskUint = u32;

/// Version string such as `hyperpuzzle v1.2.3`.
pub const PUZZLE_ENGINE_VERSION_STRING: &str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

/// Default length for a full scramble
pub const FULL_SCRAMBLE_LENGTH: u32 = 1000;

/// Radius of the promordial cube, which determines the maximum extent of all
/// vertices along any axis.
pub const PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 1_048_576.0; // big power of 2 feels good

/// Name of the default color scheme, if no other is specified.
pub const DEFAULT_COLOR_SCHEME_NAME: &str = "Default";
/// Name of the default gradient, to which unknown or conflicting colors are
/// assigned.
pub const DEFAULT_COLOR_GRADIENT_NAME: &str = "Rainbow";

/// Maximum number of ID redirects.
const MAX_ID_REDIRECTS: usize = 5;

/// Parses the ID of a generated object into its components: the generator ID,
/// and the parameters. Returns `None` if the ID is not a valid ID for a
/// generated object.
pub fn parse_generated_id(id: &str) -> Option<(&str, Vec<&str>)> {
    let (generator_id, args) = id.split_once(':')?;
    Some((generator_id, args.split(',').collect()))
}

/// Returns the ID of a generated object.
pub fn generated_id(generator_id: &str, params: impl IntoIterator<Item = impl ToString>) -> String {
    let mut ret = generator_id.to_owned();
    let mut is_first = true;
    for param in params {
        ret += if is_first { ":" } else { "," };
        is_first = false;
        ret += &param.to_string();
    }
    ret
}

/// Compares IDs of objects in a [`Catalog`].
///
/// Currently this uses [`human_sort`], a string comparison algorithm that is
/// handles numbers in a human-friendly way.
pub fn compare_ids(a: &str, b: &str) -> std::cmp::Ordering {
    human_sort::compare(a, b)
}

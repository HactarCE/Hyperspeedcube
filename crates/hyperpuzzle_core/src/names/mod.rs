mod name_spec;
mod name_spec_bi_map;
mod string_bi_map;

pub use name_spec::{NameSpec, NameSpecMap, preferred_name_from_name_spec};
pub use name_spec_bi_map::{NameSpecBiMap, NameSpecBiMapBuilder};
pub use string_bi_map::{StringBiMap, StringBiMapBuilder};

/// Error indicating a bad name.
#[allow(missing_docs)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum BadName {
    #[error("name {name:?} is already taken by #{id}")]
    AlreadyTaken { name: String, id: usize },
    #[error("name {name:?} is invalid")]
    InvalidName { name: String },
    #[error("name is empty")]
    Empty,
    #[error("internal error (this is a bug)")]
    InternalError,
    #[error("exhausted list of auto-generated names")]
    ExhaustedAutonames,
}

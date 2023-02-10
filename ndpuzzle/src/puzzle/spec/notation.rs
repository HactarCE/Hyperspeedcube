use serde::{Deserialize, Serialize};

/// Specification for a twist notation system.
///
/// TODO: add `#[serde(deny_unknown_fields)]`
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct NotationSpec {}

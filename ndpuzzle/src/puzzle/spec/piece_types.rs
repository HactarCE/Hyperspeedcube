use serde::{Deserialize, Serialize};

/// Specification for piece types.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
// TODO: add `#[serde(deny_unknown_fields)]`
pub struct PieceTypesSpec {}

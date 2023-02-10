use serde::{Deserialize, Serialize};

/// Specification for a puzzle control system.
///
/// TODO: add `#[serde(deny_unknown_fields)]`
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ControlsSpec {}

use std::fmt;

use crate::{CatalogArgValue, CatalogId};

/// Parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorParam {
    /// Human-friendly name.
    pub name: String,
    /// Parameter type.
    pub ty: GeneratorParamType,
    /// Default value.
    pub default: GeneratorParamValue,
}
impl GeneratorParam {
    /// Converts a string to a value for this parameter and returns an error if
    /// it is invalid.
    pub fn value_from_arg(
        &self,
        arg: &CatalogArgValue,
    ) -> Result<GeneratorParamValue, GeneratorParamError> {
        let make_error = || GeneratorParamError {
            expected: self.clone(),
            got: arg.to_string(),
        };

        match self.ty {
            GeneratorParamType::Int { .. } => Ok(GeneratorParamValue::Int(
                arg.to_int().ok_or_else(make_error)?,
            )),
            GeneratorParamType::Puzzle => Ok(GeneratorParamValue::PuzzleId(arg.to_id())),
        }
    }
}
/// Type of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorParamType {
    /// Integer.
    Int {
        /// Minimum value (inclusive).
        min: i64,
        /// Maximum value (inclusive).
        max: i64,
    },
    /// Puzzle ID.
    Puzzle,
}
impl fmt::Display for GeneratorParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorParamType::Int { min, max } => write!(f, "int ({min} to {max})"),
            GeneratorParamType::Puzzle => write!(f, "puzzle"),
        }
    }
}

/// Value of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeneratorParamValue {
    /// Integer.
    Int(i64),
    /// Puzzle ID.
    PuzzleId(CatalogId),
}
impl fmt::Display for GeneratorParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorParamValue::Int(i) => write!(f, "{i}"),
            GeneratorParamValue::PuzzleId(id) => write!(f, "{id}"),
        }
    }
}
impl From<GeneratorParamValue> for CatalogArgValue {
    fn from(value: GeneratorParamValue) -> Self {
        match value {
            GeneratorParamValue::Int(i) => i.into(),
            GeneratorParamValue::PuzzleId(id) => id.into(),
        }
    }
}

/// Error encountered when parsing a generator parameter.
#[derive(Debug, Clone)]
pub struct GeneratorParamError {
    /// Parameter requirements.
    pub expected: GeneratorParam,
    /// Value supplied.
    pub got: String,
}
impl fmt::Display for GeneratorParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { expected, got } = self;
        let GeneratorParam { name, ty, .. } = expected;
        write!(f, "bad value {got:?} for param {name:?} (expected {ty})")
    }
}
impl std::error::Error for GeneratorParamError {}

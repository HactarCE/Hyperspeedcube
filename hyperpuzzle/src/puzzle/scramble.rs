use rand::Rng;

use crate::Timestamp;

/// Info about how to generate a scramble.
///
/// Given a puzzle definition, this exactly determines the scramble content.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ScrambleInfo {
    /// Type of scramble to generate.
    pub ty: ScrambleType,
    /// Timestamp when the scramble was requested.
    pub time: Timestamp,
    /// Random seed, probably sourced from a "true" RNG provided by the OS.
    pub seed: u32,
}
impl ScrambleInfo {
    /// Generates a new random scramble based on the current time.
    pub fn new(ty: ScrambleType) -> Self {
        Self {
            ty,
            time: Timestamp::now(),
            seed: rand::rng().random(),
        }
    }
}

/// Type of scramble to generate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrambleType {
    /// Full scramble.
    Full,
    /// Partial scramble of a specific number of moves.
    Partial(u32),
}

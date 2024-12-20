use rand::Rng;

use crate::Timestamp;

/// Parameters to deterministically generate a twist sequence to scramble a
/// puzzle.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ScrambleParams {
    /// Type of scramble to generate.
    pub ty: ScrambleType,
    /// Timestamp when the scramble was requested.
    pub time: Timestamp,
    /// Random seed, probably sourced from a "true" RNG provided by the OS.
    pub seed: u32,
}
impl ScrambleParams {
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

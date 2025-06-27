//! N-dimensional Euclidean geometry functionality.

use crate::{Builtins, Result};

mod plane;
mod point;
mod transform;

/// Adds the built-in operators and functions.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    plane::define_in(builtins)?;
    point::define_in(builtins)?;
    transform::define_in(builtins)?;
    Ok(())
}

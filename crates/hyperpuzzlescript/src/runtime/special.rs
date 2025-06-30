use crate::{Result, Value, ast};

/// Scoped special variables.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct SpecialVariables {
    /// Number of dimensions in the space.
    pub ndim: Option<u8>,
    /// Symmetry to apply for puzzle operations.
    pub sym: Value,

    /// Active puzzle.
    pub puz: Value,
    /// Active shape.
    pub shape: Value,
    /// Active twist system.
    pub twists: Value,
    /// Active axis system.
    pub axes: Value,
}
impl SpecialVariables {
    /// Sets a special variable in the `with` block.
    pub fn set(&mut self, ident: ast::SpecialVar, value: Value) -> Result<()> {
        match ident {
            ast::SpecialVar::Ndim => self.ndim = Some(value.to()?),
            ast::SpecialVar::Sym => self.sym = value,

            ast::SpecialVar::Puz => self.puz = value,
            ast::SpecialVar::Shape => self.shape = value,
            ast::SpecialVar::Twists => self.twists = value,
            ast::SpecialVar::Axes => self.axes = value,
        }

        Ok(())
    }
}

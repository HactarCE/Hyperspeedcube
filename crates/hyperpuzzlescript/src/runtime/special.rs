use crate::{Result, Value, ast};

/// Scoped special variables.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct SpecialVariables {
    /// Number of dimensions in the space.
    pub ndim: Option<u8>,
    /// Symmetry to apply for puzzle operations.
    pub sym: Value,
}
impl SpecialVariables {
    /// Sets a special variable in the `with` block.
    pub fn set(&mut self, ident: ast::SpecialVar, value: Value) -> Result<()> {
        match ident {
            ast::SpecialVar::Ndim => self.ndim = Some(value.to()?),
            ast::SpecialVar::Sym => self.sym = value,
        }
        Ok(())
    }
}

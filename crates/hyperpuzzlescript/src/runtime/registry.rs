use crate::{Error, Result, Span, Value, ast};

/// Scoped special variables.
#[derive(Debug, Default, Clone)]
pub struct SpecialVariables {
    /// Number of dimensions in the space.
    pub ndim: Option<u8>,
}
impl SpecialVariables {
    /// Returns `#ndim`, or errors if it is undefined.
    pub fn ndim(&self, span: Span) -> Result<u8> {
        self.ndim.ok_or(Error::NoNdim.at(span))
    }

    /// Sets a special variable in the `with` block.
    pub fn set(&mut self, ident: ast::SpecialVar, value: Value) -> Result<()> {
        match ident {
            ast::SpecialVar::Ndim => self.ndim = Some(value.as_u8()?),
            ast::SpecialVar::Sym => todo!("set sym"),
        }
        Ok(())
    }
}

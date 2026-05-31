use std::sync::Arc;

use ecow::EcoString;
use parking_lot::Mutex;

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
    pub puz: Arc<Mutex<Value>>,
    /// Active shape.
    pub shape: Arc<Mutex<Value>>,
    /// Active twist system.
    pub twists: Arc<Mutex<Value>>,
    /// Active axis system.
    pub axes: Arc<Mutex<Value>>,

    /// Generator or puzzle ID.
    pub id: Option<EcoString>,
}
impl SpecialVariables {
    /// Sets a special variable in the `with` block.
    pub fn set(&mut self, ident: ast::SpecialVar, value: Value) -> Result<()> {
        match ident {
            ast::SpecialVar::Ndim => self.ndim = value.to()?,
            ast::SpecialVar::Sym => self.sym = value,

            ast::SpecialVar::Puz => self.puz = Arc::new(Mutex::new(value)),
            ast::SpecialVar::Shape => self.shape = Arc::new(Mutex::new(value)),
            ast::SpecialVar::Twists => self.twists = Arc::new(Mutex::new(value)),
            ast::SpecialVar::Axes => self.axes = Arc::new(Mutex::new(value)),

            ast::SpecialVar::Id => self.id = value.to()?,
        }

        Ok(())
    }
}

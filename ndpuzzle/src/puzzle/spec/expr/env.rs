use ahash::AHashMap;

use super::{constants, Function, Value};

/// Evaluation environment for a math expression.
#[derive(Debug, Clone)]
pub struct Env<'a> {
    pub ndim: u8,

    pub functions: AHashMap<&'a str, Function<'a>>,
    pub constants: AHashMap<&'a str, Value>,
    pub parent: Option<&'a Env<'a>>,
}
impl<'a> Env<'a> {
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    pub fn with_ndim(ndim: u8) -> Self {
        Self {
            ndim,

            functions: Function::builtins(),
            constants: constants::builtin_constants(),
            parent: None,
        }
    }

    pub fn lookup_constant(&self, name: &str) -> Option<&Value> {
        self.constants
            .get(name)
            .or_else(|| self.parent?.lookup_constant(name))
    }
    pub fn lookup_function(&self, name: &str) -> Option<&Function<'a>> {
        self.functions
            .get(name)
            .or_else(|| self.parent?.lookup_function(name))
    }
    pub fn base(&self) -> &Self {
        match self.parent {
            Some(parent) => parent.base(),
            None => self,
        }
    }
}

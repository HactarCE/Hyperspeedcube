use ahash::AHashMap;

use super::{constants, Function, Value};

/// Evaluation environment for a math expression.
#[derive(Debug, Clone)]
pub struct Env<'a> {
    pub(super) ndim: u8,

    pub(super) functions: AHashMap<&'a str, Function<'a>>,
    pub(super) constants: AHashMap<&'a str, Value>,
    pub(super) parent: Option<&'a Env<'a>>,
}
impl<'a> Env<'a> {
    /// Returns the number of dimensions in the environment.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Constructs a new environment with a specific number of dimensions.
    pub fn with_ndim(ndim: u8) -> Self {
        Self {
            ndim,

            functions: Function::builtins(),
            constants: constants::builtin_constants(),
            parent: None,
        }
    }

    pub(super) fn lookup_constant(&self, name: &str) -> Option<&Value> {
        self.constants
            .get(name)
            .or_else(|| self.parent?.lookup_constant(name))
    }
    pub(super) fn lookup_function(&self, name: &str) -> Option<&Function<'a>> {
        self.functions
            .get(name)
            .or_else(|| self.parent?.lookup_function(name))
    }
    pub(super) fn base_env(&self) -> &Self {
        match self.parent {
            Some(parent) => parent.base_env(),
            None => self,
        }
    }
}

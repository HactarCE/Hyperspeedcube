use ahash::AHashMap;
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use super::{Function, SpannedValue, Value};
use crate::math::*;

#[derive(Debug, Clone)]
pub struct Ctx<'a> {
    ndim: u8,

    functions: AHashMap<&'a str, Arc<dyn Function>>,
    constants: AHashMap<&'a str, Value>,
}
impl Ctx<'_> {
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    fn with_ndim(ndim: u8) -> Self {
        Self {
            ndim,

            functions: super::BUILTIN_FUNCTIONS.iter().cloned().collect(),
            constants: super::BUILTIN_CONSTANTS.iter().cloned().collect(),
        }
    }
}

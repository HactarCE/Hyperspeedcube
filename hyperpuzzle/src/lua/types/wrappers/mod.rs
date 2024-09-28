//! Small conversion wrappers for various Lua types (mostly numbers or strings).
//!
//! This is a wrapper type that just describes how a Lua value is converted to a
//! Rust value, along with the error messages that should be generated.

use mlua::prelude::*;

mod multivector_index;
mod ndim;
mod numbers;
mod sequence;
mod vec_string;
mod vector_index;

pub use multivector_index::LuaMultivectorIndex;
pub use ndim::LuaNdim;
pub use numbers::{LuaIndex, LuaIntegerNoConvert, LuaMirrorIndex, LuaNumberNoConvert};
pub use sequence::LuaSequence;
pub use vec_string::LuaVecString;
pub use vector_index::LuaVectorIndex;

use super::*;

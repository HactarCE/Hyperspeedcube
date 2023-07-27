use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::geometry::ShapeArena;
use crate::math::{cga::*, *};

#[derive(Debug, Clone)]
pub struct LuaSpace(pub Arc<Mutex<ShapeArena>>);

impl LuaSpace {
    pub fn new(ndim: u8) -> Self {
        LuaSpace(Arc::new(Mutex::new(ShapeArena::new_euclidean_cga(ndim))))
    }
}

impl LuaUserData for LuaSpace {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {}
}

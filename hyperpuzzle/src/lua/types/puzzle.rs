use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::PuzzleBuilder;

lua_userdata_value_conversion_wrapper! {
    #[name = "puzzlebuilder"]
    pub struct LuaPuzzleBuilder(Arc<Mutex<PuzzleBuilder>>);
}

impl LuaUserData for LuaNamedUserData<Arc<Mutex<PuzzleBuilder>>> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        // TODO
    }
}

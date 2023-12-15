use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::PuzzleBuilder;

lua_userdata_value_conversion_wrapper! {
    #[name = "puzzlebuilder"]
    pub struct LuaPuzzleBuilder(Arc<Mutex<Option<PuzzleBuilder>>>);
}

impl LuaUserData for LuaNamedUserData<Arc<Mutex<Option<PuzzleBuilder>>>> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(_methods: &mut T) {
        // TODO
    }
}

impl LuaPuzzleBuilder {
    pub fn lock(&self) -> MutexGuard<'_, Option<PuzzleBuilder>> {
        self.0.lock()
    }

    pub fn get(lua: LuaContext<'_>) -> LuaResult<Self> {
        lua.globals()
            .get("PUZZLE")
            .map_err(|_| LuaError::external("no puzzle being built"))
    }
    pub fn with<T, E: LuaExternalError>(
        lua: LuaContext<'_>,
        f: impl FnOnce(&mut PuzzleBuilder) -> Result<T, E>,
    ) -> LuaResult<T> {
        let mutex = Self::get(lua)?;
        let mut mutex_guard = mutex.lock();
        let puzzle_builder = mutex_guard
            .as_mut()
            .ok_or_else(|| LuaError::external("no puzzle being built"))?;
        f(puzzle_builder).to_lua_err()
    }
    pub fn take(lua: LuaContext<'_>) -> LuaResult<PuzzleBuilder> {
        Self::get(lua)?
            .lock()
            .take()
            .ok_or_else(|| LuaError::external("no puzzle being bulit"))
    }

    pub fn carve(lua: LuaContext<'_>, LuaManifold(m): LuaManifold) -> LuaResult<()> {
        Self::with(lua, |this| this.carve(&this.active_pieces(), m))?;
        Ok(())
    }
    pub fn slice(lua: LuaContext<'_>, LuaManifold(m): LuaManifold) -> LuaResult<()> {
        Self::with(lua, |this| {
            this.slice(&this.active_pieces(), m)
                .map_err(|e| LuaError::external(format!("{e:?}")))
        })?;
        Ok(())
    }
}

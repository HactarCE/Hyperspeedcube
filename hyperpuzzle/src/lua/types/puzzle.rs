use std::{collections::HashMap, sync::Arc};

use eyre::{bail, Result};
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::{Color, PuzzleBuilder};

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
        Self::with(lua, |this| this.slice(&this.active_pieces(), m))?;
        Ok(())
    }

    pub fn add_color(lua: LuaContext<'_>, LuaManifold(m): LuaManifold) -> LuaResult<()> {
        Self::with(lua, |this| this.add_color(m))?;
        Ok(())
    }
    pub fn name_colors(lua: LuaContext<'_>, names: LuaTable<'_>) -> LuaResult<()> {
        Self::with(lua, |this| {
            for (i, name) in names.sequence_values().enumerate() {
                this.set_color_name(Color(i as u16), name?)?;
            }
            eyre::Ok(())
        })
    }
    pub fn reorder_colors<'lua>(lua: LuaContext<'lua>, new_order: LuaTable<'lua>) -> LuaResult<()> {
        Self::with(lua, |this| {
            let expected_len = this.colors.len();
            let actual_len = new_order.len()? as usize;
            if expected_len != actual_len {
                bail!("{expected_len} colors are defined but {actual_len} colors are given")
            }

            let colors_by_name: HashMap<String, Color> = this
                .colors
                .iter()
                .filter_map(|(id, color)| Some((color.name.clone()?, id)))
                .collect();
            let mut colors_seen = this.colors.map_ref(|_, _| false);

            let new_order = new_order
                .sequence_values()
                .map(|value| {
                    let name_or_id: LuaValue<'_> = value?;
                    if let LuaValue::String(s) = name_or_id {
                        let s = s.to_str()?;
                        match colors_by_name.get(s) {
                            Some(&color) => Ok(color),
                            None => bail!("no color named {s:?}"),
                        }
                    } else if let Ok(LuaIntegerNoConvert(color_index)) =
                        <_>::from_lua(name_or_id.clone(), lua)
                    {
                        if (1..=this.colors.len() as LuaInteger).contains(&color_index) {
                            Ok(Color(color_index as u16 - 1)) // -1 because Lua is 1-indexed
                        } else {
                            bail!("color index {color_index} out of range");
                        }
                    } else {
                        bail!("expected color name or index; got {name_or_id:?}");
                    }
                })
                .map(|color| {
                    let color = color?;
                    if std::mem::replace(&mut colors_seen[color], true) {
                        // +1 because Lua is 1-indexed
                        bail!("duplicate color order assignment for #{}", color.0 + 1);
                    }
                    Ok(color)
                })
                .collect::<Result<Vec<Color>>>()?;

            this.set_color_order(new_order.into())
        })
    }
    pub fn set_default_colors(lua: LuaContext<'_>, colors: LuaTable<'_>) -> LuaResult<()> {
        Self::with(lua, |this| {
            let expected_len = this.colors.len();
            let actual_len = colors.len()? as usize;
            if expected_len != actual_len {
                bail!("{expected_len} colors are defined but {actual_len} colors are given")
            }

            for (i, default_color) in colors.sequence_values().enumerate() {
                this.set_color_default_color(Color(i as u16), default_color?)?;
            }
            eyre::Ok(())
        })
    }
}

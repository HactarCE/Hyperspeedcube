use std::sync::Arc;

use super::*;
use crate::builder::{ColorSystemBuilder, PuzzleBuilder};
use crate::lua::lua_warn_fn;
use crate::{LibraryDb, Puzzle};

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleParams {
    /// String ID of the puzzle.
    pub id: String,
    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,

    /// Color system.
    pub colors: Option<ColorSystemParams>,

    /// User-friendly name for the puzzle.
    pub name: Option<String>,
    /// Alternative user-friendly names for the puzzle.
    pub aliases: Vec<String>,
    /// Lua table containing metadata about the puzzle.
    pub meta: Option<LuaRegistryKey>,
    /// Lua table containing additional properties of the puzzle.
    pub properties: Option<LuaRegistryKey>,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: Option<bool>,

    /// Lua function to build the puzzle.
    user_build_fn: LuaRegistryKey,
}

impl<'lua> FromLua<'lua> for PuzzleParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let name: Option<String>;
        let ndim: LuaNdim;
        let build: LuaFunction<'lua>;
        let colors: Option<ColorSystemParams>;
        let aliases: Option<Vec<String>>;
        let meta: Option<LuaTable<'lua>>;
        let properties: Option<LuaTable<'lua>>;
        let remove_internals: Option<bool>;

        unpack_table!(lua.unpack(table {
            name,
            ndim,
            build,

            colors,

            aliases,
            meta,
            properties,

            remove_internals,
        }));

        let create_opt_registry_value = |v| -> LuaResult<Option<LuaRegistryKey>> {
            match v {
                Some(v) => Ok(Some(lua.create_registry_value(v)?)),
                None => Ok(None),
            }
        };

        Ok(PuzzleParams {
            id: String::new(), // This is overwritten in `puzzledb:add()`.
            ndim,

            colors,

            name,
            aliases: aliases.unwrap_or(vec![]),
            meta: create_opt_registry_value(meta)?,
            properties: create_opt_registry_value(properties)?,

            remove_internals,

            user_build_fn: lua.create_registry_value(build)?,
        })
    }
}

impl PuzzleParams {
    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua) -> LuaResult<Arc<Puzzle>> {
        let LuaNdim(ndim) = self.ndim;
        let id = self.id.clone();
        let name = self.name.clone().unwrap_or(self.id.clone());
        let puzzle_builder = PuzzleBuilder::new(id, name, ndim).into_lua_err()?;
        if let Some(colors) = &self.colors {
            puzzle_builder.lock().shape.colors = colors.build(lua)?;
        }
        if let Some(remove_internals) = self.remove_internals {
            puzzle_builder.lock().shape.remove_internals = remove_internals;
        }
        let space = puzzle_builder.lock().space();

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            lua.registry_value::<LuaFunction<'_>>(&self.user_build_fn)?
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        let mut puzzle_builder = puzzle_builder.lock();

        // Assign default piece type to remaining pieces.
        puzzle_builder.shape.mark_untyped_pieces().into_lua_err()?;

        puzzle_builder.build(lua_warn_fn(lua)).into_lua_err()
    }

    /// Returns the name or the ID of the puzzle.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone)]
pub enum ColorSystemParams {
    ById(String),
    Bespoke(ColorSystemBuilder),
}
impl<'lua> FromLua<'lua> for ColorSystemParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Table(t) => Ok(Self::Bespoke(
                crate::lua::types::color_system::from_lua_table(lua, None, t)?,
            )),
            LuaValue::String(id) => Ok(Self::ById(id.to_string_lossy().into_owned())),
            _ => Err(LuaError::external(
                "expected string, table, or nil for `colors`",
            )),
        }
    }
}
impl ColorSystemParams {
    pub fn build(&self, lua: &Lua) -> LuaResult<ColorSystemBuilder> {
        match self {
            ColorSystemParams::ById(id) => LibraryDb::build_color_system(lua, id),
            ColorSystemParams::Bespoke(colors) => Ok(colors.clone()),
        }
    }
}

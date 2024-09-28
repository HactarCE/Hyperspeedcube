use std::sync::Arc;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::{LibraryDb, Puzzle, PuzzleMetadata, PuzzleMetadataExternal};

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleParams {
    /// String ID of the puzzle.
    pub id: String,
    /// Version of the puzzle.
    pub version: Version,

    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,
    /// Lua function to build the puzzle.
    user_build_fn: LuaRegistryKey,

    /// Color system ID.
    pub colors: Option<String>,

    /// User-friendly name for the puzzle. (default = same as ID)
    pub name: Option<String>,
    /// Lua table containing metadata about the puzzle.
    pub meta: PuzzleMetadata,
    /// Lua table containing additional properties of the puzzle.
    pub properties: Option<LuaRegistryKey>,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: Option<bool>,
}

impl<'lua> FromLua<'lua> for PuzzleParams {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let id: String;
        let version: Version;
        let ndim: LuaNdim;
        let build: LuaFunction<'lua>;
        let name: Option<String>;
        let colors: Option<String>;
        let meta: PuzzleMetadata;
        let properties: Option<LuaTable<'lua>>;
        let remove_internals: Option<bool>;
        let __generated__: Option<bool>;
        unpack_table!(lua.unpack(table {
            id,
            name,
            version,

            ndim,
            build,

            colors,

            meta,
            properties,

            remove_internals,

            __generated__, // TODO: this can be hacked
        }));

        let id = if __generated__ == Some(true) {
            id // ID already validated
        } else {
            crate::validate_id(id).into_lua_err()?
        };

        Ok(PuzzleParams {
            id,
            version,

            ndim,
            user_build_fn: lua.create_registry_value(build)?,

            colors,

            name,
            meta,
            properties: crate::lua::create_opt_registry_value(lua, properties)?,

            remove_internals,
        })
    }
}

impl PuzzleParams {
    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua) -> LuaResult<Arc<Puzzle>> {
        let LuaNdim(ndim) = self.ndim;
        let id = self.id.clone();
        let name = self.name.clone().unwrap_or_else(|| {
            lua.warning(format!("missing `name` for puzzle `{id}`"), false);
            self.id.clone()
        });
        let version = self.version.clone();
        let puzzle_builder = PuzzleBuilder::new(id, name, version, ndim).into_lua_err()?;
        if let Some(colors_id) = &self.colors {
            puzzle_builder.lock().shape.colors = LibraryDb::build_color_system(lua, colors_id)?;
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

impl<'lua> FromLua<'lua> for PuzzleMetadata {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_nil() {
            return Ok(PuzzleMetadata::default());
        }

        let table: LuaTable<'lua> = lua.unpack(value)?;

        let author: Option<String>;
        let authors: Option<Vec<String>>;
        let inventor: Option<String>;
        let inventors: Option<Vec<String>>;
        let aliases: Option<Vec<String>>;
        let external: PuzzleMetadataExternal;
        unpack_table!(lua.unpack(table {
            author,
            authors,
            inventor,
            inventors,
            aliases,
            external,
        }));

        let mut authors = authors.unwrap_or_default();
        authors.extend(author);
        let mut inventors = inventors.unwrap_or_default();
        inventors.extend(inventor);
        let aliases = aliases.unwrap_or_default();

        Ok(PuzzleMetadata {
            authors,
            inventors,
            aliases,
            external,
        })
    }
}

impl<'lua> FromLua<'lua> for PuzzleMetadataExternal {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if value.is_nil() {
            return Ok(PuzzleMetadataExternal::default());
        }

        let table: LuaTable<'lua> = lua.unpack(value)?;

        let wca: Option<String>;
        unpack_table!(lua.unpack(table { wca }));

        Ok(PuzzleMetadataExternal { wca })
    }
}

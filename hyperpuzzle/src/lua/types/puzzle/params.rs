use std::sync::Arc;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::{LibraryDb, Puzzle};

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleParams {
    /// String ID of the puzzle.
    pub id: String,
    /// Version of the puzzle. (default = `[0, 0, 0]`)
    pub version: [usize; 3],
    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,

    /// Color system ID.
    pub colors: Option<String>,

    /// User-friendly name for the puzzle. (default = same as ID)
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

        let id: String;
        let version: Option<String>;
        let name: Option<String>;
        let ndim: LuaNdim;
        let build: LuaFunction<'lua>;
        let colors: Option<String>;
        let aliases: Option<Vec<String>>;
        let meta: Option<LuaTable<'lua>>;
        let properties: Option<LuaTable<'lua>>;
        let remove_internals: Option<bool>;

        unpack_table!(lua.unpack(table {
            id,
            name,
            version,
            ndim,
            build,

            colors,

            aliases,
            meta,
            properties,

            remove_internals,
        }));

        let id = crate::validate_id(id.clone()).into_lua_err()?;

        let version = match version {
            Some(s) => parse_puzzle_version_str(&s).unwrap_or_else(|e| {
                lua.warning(format!("error in `version` for puzzle {id}: {e}"), false);
                [0; 3]
            }),
            None => {
                lua.warning(format!("missing `version` for puzzle {id}"), false);
                [0; 3]
            }
        };

        let create_opt_registry_value = |v| -> LuaResult<Option<LuaRegistryKey>> {
            match v {
                Some(v) => Ok(Some(lua.create_registry_value(v)?)),
                None => Ok(None),
            }
        };

        Ok(PuzzleParams {
            id,
            version,
            ndim,

            colors,

            name,
            aliases: aliases.unwrap_or_default(),
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

fn parse_puzzle_version_str(version_string: &str) -> Result<[usize; 3], String> {
    fn parse_component(s: &str) -> Result<usize, String> {
        s.parse()
            .map_err(|e| format!("invalid major version because {e}"))
    }

    let mut segments = version_string.split('.');
    let major = parse_component(segments.next().ok_or("missing major version")?)?;
    let minor = parse_component(segments.next().unwrap_or("0"))?;
    let patch = parse_component(segments.next().unwrap_or("0"))?;
    if segments.next().is_some() {
        return Err("too many segments; only the form `major.minor.patch` is accepted".to_owned());
    }
    Ok([major, minor, patch])
}

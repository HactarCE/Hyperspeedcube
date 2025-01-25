use std::sync::Arc;

use hyperpuzzle_core::{
    BuildTask, Progress, Puzzle, PuzzleListMetadata, PuzzleSpec, Redirectable, TagValue,
};
use parking_lot::Mutex;

use super::*;
use crate::builder::{ColorSystemBuilder, PuzzleBuilder};
use crate::lua::lua_warn_fn;

/// Specification for a puzzle.
#[derive(Debug)]
pub struct LuaPuzzleSpec {
    /// Metadata for the puzzle.
    pub meta: PuzzleListMetadata,

    /// Color system ID.
    pub colors: Option<String>,

    /// Number of dimensions of the space in which the puzzle is constructed.
    pub ndim: LuaNdim,
    /// Lua function to build the puzzle.
    build: LuaFunction,

    /// Whether to automatically remove internal pieces as they are constructed.
    pub remove_internals: Option<bool>,
    /// Number of moves for a full scramble.
    pub full_scramble_length: Option<u32>,
}

impl FromLua for LuaPuzzleSpec {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let ret = LuaPuzzleSpec::from_lua_table(lua, lua.unpack(value)?)?;
        crate::validate_id_str(&ret.meta.id).into_lua_err()?;
        Ok(ret)
    }
}

impl LuaPuzzleSpec {
    pub(crate) fn from_generated_lua_table(lua: &Lua, table: LuaTable) -> LuaResult<Self> {
        // don't validate ID
        Self::from_lua_table(lua, table)
    }

    fn from_lua_table(lua: &Lua, table: LuaTable) -> LuaResult<Self> {
        let id: String;
        let version: LuaVersion;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let tags: Option<LuaTable>;
        let colors: Option<String>;
        let ndim: LuaNdim;
        let build: LuaFunction;
        let remove_internals: Option<bool>;
        let scramble: Option<u32>;
        unpack_table!(lua.unpack(table {
            id,
            version,
            name,
            aliases,
            tags,
            colors,
            ndim,
            build,
            remove_internals,
            scramble,
        }));

        let mut tags = crate::lua::tags::unpack_tags_table(lua, tags)?;

        if let Some(color_system_id) = colors.clone() {
            tags.insert_named("colors/system", TagValue::Str(color_system_id))
                .map_err(LuaError::external)?;
        }

        if let Some(filename) = crate::lua::lua_current_filename(lua) {
            tags.insert_named("file", TagValue::Str(filename))
                .map_err(LuaError::external)?;
        }

        crate::lua::tags::inherit_parent_tags(&mut tags);

        crate::lua::protect_with_local_env(lua, &build)?;

        let name = name.unwrap_or_else(|| {
            lua.warning(format!("missing `name` for puzzle `{id}`"), false);
            id.clone()
        });

        Ok(LuaPuzzleSpec {
            meta: PuzzleListMetadata {
                id,
                version: version.0,
                name,
                aliases: aliases.unwrap_or_default(),
                tags,
            },

            colors,

            ndim,
            build,

            remove_internals,
            full_scramble_length: scramble,
        })
    }

    /// Converts to a [`PuzzleSpec`].
    pub fn into_puzzle_spec(self, lua: &Lua) -> PuzzleSpec {
        let lua = lua.clone();
        PuzzleSpec {
            meta: self.meta.clone(),
            build: Box::new(move |ctx| {
                crate::lua::env::set_logger(&lua, &ctx.logger);
                let puzzle = self.build(&lua, &ctx.progress)?;
                Ok(Redirectable::Direct(puzzle))
            }),
        }
    }

    /// Runs initial setup, user Lua code, and final construction for a puzzle.
    pub fn build(&self, lua: &Lua, progress: &Mutex<Progress>) -> LuaResult<Arc<Puzzle>> {
        progress.lock().task = BuildTask::Initializing;

        let LuaNdim(ndim) = self.ndim;
        let puzzle_builder = PuzzleBuilder::new(self.meta.clone(), ndim)
            .map_err(|e| LuaError::external(format!("{e:#}")))?;
        if let Some(colors_id) = &self.colors {
            progress.lock().task = BuildTask::BuildingColors;
            puzzle_builder.lock().shape.colors = ColorSystemBuilder::from(
                &*crate::lua::LuaLoader::get_catalog(lua)
                    .build_color_system_blocking(colors_id)
                    .map_err(LuaError::external)
                    .context("error building color system")?,
            );
            progress.lock().task = BuildTask::Initializing;
        }
        if let Some(remove_internals) = self.remove_internals {
            puzzle_builder.lock().shape.remove_internals = remove_internals;
        }
        if let Some(full_scramble_length) = self.full_scramble_length {
            puzzle_builder.lock().full_scramble_length = full_scramble_length;
        }
        let space = puzzle_builder.lock().space();

        progress.lock().task = BuildTask::BuildingPuzzle;

        let () = LuaSpace(space).with_this_as_global_space(lua, || {
            self.build
                .call(LuaPuzzleBuilder(Arc::clone(&puzzle_builder)))
                .context("error executing puzzle definition")
        })?;

        progress.lock().task = BuildTask::Finalizing;

        let mut puzzle_builder = puzzle_builder.lock();

        // Assign default piece type to remaining pieces.
        puzzle_builder
            .shape
            .mark_untyped_pieces()
            .map_err(|e| LuaError::external(format!("{e:#}")))?;

        puzzle_builder
            .build(lua_warn_fn(lua))
            .map_err(|e| LuaError::external(format!("{e:#}")))
    }
}

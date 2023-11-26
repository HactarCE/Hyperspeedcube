use std::sync::Arc;

use eyre::{Result, WrapErr};
use itertools::Itertools;
use parking_lot::Mutex;
use rlua::prelude::*;

mod constants;
mod types;

pub use types::LuaLogLine;
use types::*;

use crate::{PieceSet, Puzzle, PuzzleBuilder, PuzzleDefinition};

macro_rules! lua_module {
    ($filename:literal) => {
        ($filename, include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[lua_module!("logging.lua"), lua_module!("prelude.lua")];

pub fn new_lua() -> Lua {
    let lua = Lua::new_with(
        rlua::StdLib::BASE
            | rlua::StdLib::TABLE
            | rlua::StdLib::STRING
            | rlua::StdLib::UTF8
            | rlua::StdLib::MATH,
    );

    lua.context(|lua| {
        // Monkeypatch builtin `type` function.
        let globals = lua.globals();
        globals.set("type", lua.create_function(|_, v| Ok(lua_type_name(&v)))?)?;

        for (module_name, module_source) in LUA_MODULES {
            log::info!("Loading Lua module {module_name:?}");
            if let Err(e) = lua.load(module_source).set_name(module_name)?.exec() {
                panic!("error loading Lua module {module_name:?}:\n\n{e}\n\n");
            }
        }

        // Grab the sandbox environment so we can insert our custom globals.
        let sandbox: LuaTable<'_> = lua.globals().get("SANDBOX_ENV")?;

        // Constants
        let puzzle_engine_version_string =
            format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        sandbox.raw_set("_PUZZLE_ENGINE", puzzle_engine_version_string)?;
        sandbox.raw_set("AXES", constants::lua_construct_axes_table(lua)?)?;

        // Constants
        let puzzle_engine_version_string =
            format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        sandbox.raw_set("_PUZZLE_ENGINE", puzzle_engine_version_string)?;
        sandbox.raw_set("AXES", constants::lua_construct_axes_table(lua)?)?;

        // Constructors
        let vec_fn = lua.create_function(LuaVector::construct_from_multivalue)?;
        sandbox.raw_set("vec", vec_fn)?;
        let mvec_fn = lua.create_function(LuaMultivector::construct_from_multivalue)?;
        sandbox.raw_set("mvec", mvec_fn)?;
        let plane_fn = lua.create_function(LuaManifold::construct_plane)?;
        sandbox.raw_set("plane", plane_fn)?;
        let sphere_fn = lua.create_function(LuaManifold::construct_sphere)?;
        sandbox.raw_set("sphere", sphere_fn)?;
        let schlafli_fn = lua.create_function(LuaSymmetry::construct_from_schlafli_table)?;
        sandbox.raw_set("schlafli", schlafli_fn)?;

        LuaResult::Ok(())
    })
    .expect("error initializing lua");

    lua
}

pub fn load_sandboxed<'lua>(
    lua: LuaContext<'lua>,
    filename: &str,
    contents: &str,
) -> Result<LuaTable<'lua>, LuaObjectLoadError> {
    // IIFE to mimic try_block
    (|| {
        lua.globals()
            .get::<_, LuaFunction<'_>>("start_file")?
            .call(filename)?;
        let file: LuaTable<'_> = lua.globals().get("FILE")?;
        let sandbox_env: LuaTable<'_> = file.get("env")?;

        match lua
            .load(&contents)
            .set_name(filename)?
            .set_environment(sandbox_env)?
            .exec()
        {
            Ok(()) => Ok(Ok(file)),
            Err(e) => Ok(Err(LuaObjectLoadError::UserError(e))),
        }
    })()
    .unwrap_or_else(|e| Err(LuaObjectLoadError::InternalError(e)))
}

pub fn drain_logs(lua: LuaContext<'_>) -> Vec<LuaLogLine> {
    // IIFE to mimic try_block
    (|| {
        lua.globals()
            .get::<_, LuaTable>("LOG_LINES")?
            .sequence_values::<LuaTable>()
            .map(|v| LuaLogLine::try_from(v?))
            .try_collect()
    })()
    .unwrap_or(vec![])
}

pub fn build_puzzle(lua: LuaContext<'_>, id: &str) -> Result<Arc<Puzzle>> {
    let puzzle_table = lua
        .globals()
        .get::<_, LuaTable>("library")?
        .get::<_, LuaTable>("objects")?
        .get::<_, LuaTable>(format!("puzzle[{id:?}]"))?; // TODO: this relies on Lua and Rust string escaping being the same

    lua.globals()
        .set("LOG_FILENAME", puzzle_table.get::<_, String>("filename")?)?;

    let LuaNdim(ndim) = puzzle_table
        .get("ndim")
        .wrap_err("expected `ndim` to be a number of dimensions")?;
    let name: String = puzzle_table
        .get("name")
        .wrap_err("expected `name` to be a string")?;

    let (puzzle_builder, root) = PuzzleBuilder::new_solid(name.to_string(), id.to_string(), ndim)?;
    let space = Arc::clone(&puzzle_builder.space);
    lua.globals().set("SPACE", LuaSpace(space))?;
    let puzzle_builder = Arc::new(Mutex::new(Some(puzzle_builder)));
    lua.globals()
        .set("PUZZLE", LuaPuzzleBuilder(puzzle_builder))?;

    puzzle_table
        .get::<_, LuaFunction>("build")?
        .call(LuaPieceSet(PieceSet::from_iter([root])))?;

    LuaPuzzleBuilder::take(lua)?.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_lua_tests() {
        let lua = new_lua();

        lua.context(|lua| {
            let (filename, contents) = lua_module!("tests.lua");

            let file = load_sandboxed(lua, filename, contents).expect("failed to load tests");
            let env: LuaTable<'_> = file.get("env").expect("no env");

            for pair in env.pairs::<String, LuaFunction<'_>>() {
                if let Ok((name, function)) = pair {
                    if name.starts_with("test_") {
                        println!("Running {name:?} ...");
                        if let Err(e) = function.call::<(), ()>(()) {
                            eprintln!("{e:#?}");
                            panic!("{e}");
                        }
                    }
                }
            }
        });
    }
}

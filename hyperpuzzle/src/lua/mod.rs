use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use parking_lot::Mutex;
use rlua::prelude::*;

mod constants;
mod types;
mod util;

use types::*;

use crate::{Object, PieceSet, Puzzle, PuzzleBuilder};

macro_rules! lua_module {
    ($filename:literal) => {
        ($filename, include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[
    lua_module!("util.lua"),
    lua_module!("logging.lua"),
    lua_module!("library.lua"),
    lua_module!("sandbox.lua"),
    #[cfg(test)]
    lua_module!("tests.lua"),
];

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
        let sandbox: LuaTable = lua.globals().get("SANDBOX_ENV")?;

        // Constants
        let puzzle_engine_version_string =
            format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        sandbox.set("_PUZZLE_ENGINE", puzzle_engine_version_string)?;
        sandbox.set("AXES", constants::lua_construct_axes_table(lua)?)?;

        // Constructors
        let vec_fn = lua.create_function(LuaVector::construct_from_multivalue)?;
        sandbox.set("vec", vec_fn)?;
        let mvec_fn = lua.create_function(LuaMultivector::construct_from_multivalue)?;
        sandbox.set("mvec", mvec_fn)?;
        let plane_fn = lua.create_function(LuaManifold::construct_plane)?;
        sandbox.set("plane", plane_fn)?;
        let sphere_fn = lua.create_function(LuaManifold::construct_sphere)?;
        sandbox.set("sphere", sphere_fn)?;

        LuaResult::Ok(())
    })
    .expect("error initializing lua");

    lua
}

pub fn load_sandboxed(
    lua: &Lua,
    filename: &str,
    contents: &str,
) -> Result<Vec<Object>, LuaObjectLoadError> {
    lua.context(|lua| {
        // Construct a sandbox environment.
        let sandbox_env: LuaTable = lua
            .globals()
            .get::<_, LuaFunction>("make_sandbox")?
            .call(filename)?;

        let library = lua.globals().get::<_, LuaTable>("library")?;
        library
            .get::<_, LuaFunction>("start_loading_file")?
            .call(filename)?;

        // Try to load the file.
        if let Err(e) = lua.load(&contents).set_environment(sandbox_env)?.exec() {
            let error_fn = lua.globals().get::<_, LuaFunction>("error")?;
            error_fn.call(format!("error loading {filename:?}:"))?;
            for line in e.to_string().lines() {
                error_fn.call(format!("{line}"))?;
            }
            error_fn.call("")?;
            return Ok(Err(LuaObjectLoadError::UserError(e)));
        }

        // Generate metadata for each object.
        let objects_defined_in_file = lua
            .globals()
            .get::<_, LuaTable>("library")?
            .get::<_, LuaTable>("files")?
            .get::<_, LuaTable>(filename)?;
        let result: Vec<Object> = objects_defined_in_file
            .pairs()
            .map(|pair| {
                let (_, obj): (LuaValue<'_>, LuaValue<'_>) = pair?;
                Object::from_lua(obj, lua)
            })
            .try_collect()?;

        library
            .get::<_, LuaFunction>("finish_loading_file")?
            .call(filename)?;

        Ok(Ok(result))
    })
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

pub fn build_puzzle(lua: LuaContext<'_>, name: &str) -> Result<Arc<Puzzle>> {
    let puzzle_table = lua
        .globals()
        .get::<_, LuaTable>("library")?
        .get::<_, LuaTable>("objects")?
        .get::<_, LuaTable>(format!("puzzle/{name}"))?;

    let LuaNdim(ndim) = puzzle_table.get("ndim")?;

    let (puzzle_builder, root) = PuzzleBuilder::new_solid(name.to_string(), ndim);
    let puzzle_builder = Arc::new(Mutex::new(puzzle_builder));
    lua.globals()
        .set("PUZZLE", LuaPuzzleBuilder(Arc::clone(&puzzle_builder)));

    puzzle_table
        .get::<_, LuaFunction>("build")?
        .call(LuaPieceSet(PieceSet([root].into_iter().collect())))?;

    let mut puzzle_builder = puzzle_builder.lock();
    puzzle_builder.take().build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_lua_tests() {
        let lua = new_lua();

        lua.context(|ctx| {
            for pair in ctx.globals().pairs::<String, rlua::Function>() {
                if let Ok((name, function)) = pair {
                    if name.starts_with("test_") {
                        println!("Running {name:?} ...");
                        if let Err(e) = function.call::<(), ()>(()) {
                            panic!("{e}");
                        }
                    }
                }
            }
        })
    }
}

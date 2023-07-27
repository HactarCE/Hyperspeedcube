use itertools::Itertools;
use rlua::prelude::*;

#[macro_use]
mod macros;
mod constants;
mod functions;
pub mod types;
mod util;

pub use types::{LuaFileLoadError, LuaLogLine};

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
        globals.set("type", lua_fn!(|_lua, arg| Ok(types::lua_type_name(&arg))))?;

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
        sandbox.set("vec", lua_fn!(types::lua_construct_vector))?;
        sandbox.set("mvec", lua_fn!(types::lua_construct_multivector))?;
        sandbox.set("plane", lua_fn!(functions::lua_construct_plane_manifold))?;

        LuaResult::Ok(())
    })
    .expect("error initializing lua");

    lua
}

pub fn load_sandboxed(lua: &Lua, filename: &str, contents: &str) -> Result<(), LuaFileLoadError> {
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
        match lua.load(&contents).set_environment(sandbox_env)?.exec() {
            Err(e) => {
                let error_fn = lua.globals().get::<_, LuaFunction>("error")?;
                error_fn.call(format!("error loading {filename:?}:"))?;
                for line in e.to_string().lines() {
                    error_fn.call(format!("{line}"))?;
                }
                error_fn.call("")?;
                Ok(Err(LuaFileLoadError::UserError(e)))
            }

            Ok(()) => {
                library
                    .get::<_, LuaFunction>("finish_loading_file")?
                    .call(filename)?;
                Ok(Ok(()))
            }
        }
    })
    .unwrap_or_else(|e| Err(LuaFileLoadError::InternalError(e)))
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

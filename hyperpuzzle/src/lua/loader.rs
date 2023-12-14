#![allow(clippy::semicolon_if_nothing_returned)] // useful for type inference of `()`

use std::sync::Arc;

use eyre::{bail, Result, WrapErr};
use parking_lot::Mutex;
use rlua::prelude::*;

use super::*;
use crate::{PieceSet, Puzzle, PuzzleBuilder, PuzzleData};

macro_rules! lua_module {
    ($filename:literal) => {
        ($filename, include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[
    lua_module!("prelude/01_pprint.lua"),
    lua_module!("prelude/02_logging.lua"),
    lua_module!("prelude/03_monkeypatch.lua"),
    lua_module!("prelude/04_files.lua"),
    lua_module!("prelude/05_sandbox.lua"),
];

#[derive(Debug, Clone)]
pub struct CachedPuzzle(Arc<Puzzle>);
impl rlua::UserData for CachedPuzzle {}

#[derive(Debug)]
pub struct LuaLoader {
    lua: Lua,
}
impl LuaLoader {
    pub fn new() -> Self {
        // SAFETY: We need the debug library to get traceback info for better error
        // reporting. We use Lua sandboxing functionality so the user should never
        // be able to access the debug module.
        let lua = unsafe {
            Lua::unsafe_new_with(
                rlua::StdLib::BASE
                    | rlua::StdLib::TABLE
                    | rlua::StdLib::STRING
                    | rlua::StdLib::UTF8
                    | rlua::StdLib::MATH
                    | rlua::StdLib::DEBUG,
            )
        };

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
            sandbox.raw_set("AXES", lua_axes_table(lua)?)?;

            // Constructors
            let vec_fn = lua.create_function(LuaVector::construct_from_multivalue)?;
            sandbox.raw_set("vec", vec_fn)?;
            let mvec_fn = lua.create_function(LuaMultivector::construct_from_multivalue)?;
            sandbox.raw_set("mvec", mvec_fn)?;
            let plane_fn = lua.create_function(LuaManifold::construct_plane)?;
            sandbox.raw_set("plane", plane_fn)?;
            let sphere_fn = lua.create_function(LuaManifold::construct_sphere)?;
            sandbox.raw_set("sphere", sphere_fn)?;
            let cd_fn = lua.create_function(LuaCoxeterGroup::construct_from_cd_table)?;
            sandbox.raw_set("cd", cd_fn)?;

            // Puzzle construction functions
            let carve_fn = lua.create_function(LuaPuzzleBuilder::carve)?;
            sandbox.raw_set("carve", carve_fn)?;
            let slice_fn = lua.create_function(LuaPuzzleBuilder::slice)?;
            sandbox.raw_set("slice", slice_fn)?;

            LuaResult::Ok(())
        })
        .expect("error initializing lua");

        LuaLoader { lua }
    }

    fn call_global<A: for<'lua> ToLuaMulti<'lua>, R: for<'lua> FromLuaMulti<'lua>>(
        &self,
        func_name: &str,
        args: A,
    ) -> Result<R> {
        self.lua.context(|lua| call_global(lua, func_name, args))
    }

    pub fn set_log_line_handler(
        &self,
        log_line_handler: impl 'static + Send + Fn(LuaLogLine),
    ) -> Result<()> {
        self.lua
            .context(move |lua| {
                lua.globals().set(
                    "log_line",
                    lua.create_function(move |_lua, args: LuaTable<'_>| {
                        log_line_handler(LuaLogLine::from(args));
                        Ok(())
                    })?,
                )
            })
            .wrap_err("error setting Lua log line handler")
    }

    pub fn set_file_contents(&self, filename: &str, contents: Option<&str>) -> Result<()> {
        self.call_global("set_file_contents", (filename, contents))
    }

    pub fn remove_all_files(&self) -> Result<()> {
        self.call_global("remove_all_files", ())
    }

    pub fn load_all_files(&self) {
        self.call_global("load_all_files", ())
            .expect("infallible Lua function failed!")
    }

    pub fn get_puzzle_data(&self) -> Vec<PuzzleData> {
        self.lua.context(|lua| {
            lua.globals()
                .get::<_, LuaTable<'_>>("PUZZLES")
                .expect("error reading Lua puzzle table")
                .pairs()
                .map(|pair| {
                    let (id, data): (String, LuaTable<'_>) = pair?;
                    let name = data.get("name")?;
                    let filename = data.get::<_, LuaTable<'_>>("file")?.get("name")?;
                    Ok(PuzzleData { id, name, filename })
                })
                .filter_map(|data: LuaResult<_>| {
                    if data.is_err() {
                        log::error!("ignoring broken puzzle");
                    }
                    data.ok()
                })
                .collect()
        })
    }

    pub fn build_puzzle(&self, puzzle_name: &str) -> Result<Arc<Puzzle>> {
        self.lua.context(|lua| {
            let puzzle_data: LuaTable<'_> = call_global(lua, "get_puzzle", puzzle_name)?;

            if let Some(CachedPuzzle(cached_puzzle)) = puzzle_data.get("cached")? {
                return Ok(cached_puzzle);
            }

            let id: String = puzzle_data
                .get("id")
                .wrap_err("expected `id` to be a string")?;
            let LuaNdim(ndim) = puzzle_data
                .get("ndim")
                .wrap_err("expected `ndim` to be a number of dimensions")?;
            let name: String = puzzle_data
                .get("name")
                .wrap_err("expected `name` to be a string")?;

            let (puzzle_builder, root) =
                PuzzleBuilder::new_solid(name.to_string(), id.to_string(), ndim)?;
            let space = Arc::clone(&puzzle_builder.space);

            let puzzle_builder = Arc::new(Mutex::new(Some(puzzle_builder)));

            lua.globals().set("SPACE", LuaSpace(space).to_lua(lua)?)?;
            lua.globals()
                .set("PUZZLE", LuaPuzzleBuilder(puzzle_builder).to_lua(lua)?)?;

            let build_puzzle_result: Result<Option<String>> = call_global(
                lua,
                "build_puzzle",
                (
                    puzzle_data.clone(),
                    LuaPieceSet(PieceSet::from_iter([root])),
                ),
            );

            let error = build_puzzle_result?;
            if let Some(error_message) = error {
                bail!(error_message)
            }

            let result = LuaPuzzleBuilder::take(lua)?.build()?;
            puzzle_data.set("cached", CachedPuzzle(Arc::clone(&result)))?;

            lua.globals().set("SPACE", LuaNil)?;
            lua.globals().set("PUZZLE", LuaNil)?;

            Ok(result)
        })
    }

    #[cfg(test)]
    pub fn run_test_suite(&self, filename: &str, contents: &str) {
        self.set_file_contents(filename, Some(contents))
            .expect("failed to load tests");
        self.lua.context(|lua| {
            let file = load_file(lua, filename).expect("error loading test file");
            let env: rlua::Table<'_> = file.get("environment").expect("no env");

            for pair in env.pairs::<String, rlua::Function<'_>>() {
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

fn call_global<'lua, A: ToLuaMulti<'lua>, R: FromLuaMulti<'lua>>(
    lua: LuaContext<'lua>,
    func_name: &str,
    args: A,
) -> Result<R> {
    lua.globals()
        .get::<_, LuaFunction<'_>>(func_name)
        .wrap_err_with(|| format!("missing global Lua function {func_name}"))?
        .call::<A, R>(args)
        .wrap_err_with(|| format!("error calling global Lua function {func_name}"))
}

fn lua_axes_table(lua: LuaContext<'_>) -> LuaResult<LuaTable<'_>> {
    let ret = lua.create_table()?;
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate() {
        ret.set(i, c.to_string())?;
        ret.set(c.to_string(), i)?;
        ret.set(c.to_ascii_lowercase().to_string(), i)?;
    }
    let read_only_metatable: LuaTable<'_> = lua.globals().get("READ_ONLY_METATABLE")?;
    ret.set_metatable(Some(read_only_metatable));
    Ok(ret)
}

#[cfg(test)]
fn load_file<'lua>(lua: LuaContext<'lua>, filename: &str) -> Result<LuaTable<'lua>> {
    call_global(lua, "load_file", filename)
}

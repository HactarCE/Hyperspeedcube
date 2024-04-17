use std::sync::Arc;

use eyre::Result;
use itertools::Itertools;
use mlua::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::library::{LibraryDb, LibraryFile, LibraryFileLoadResult, LibraryFileLoadState};

macro_rules! lua_module {
    ($filename:literal) => {
        // Start with an `=` so that the logger skips this file
        (concat!("=", $filename), include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[
    lua_module!("prelude/pprint.lua"),
    lua_module!("prelude/utils.lua"),
    lua_module!("prelude/sandbox.lua"),
];

/// Lua runtime for loading Lua files.
#[derive(Debug)]
pub(crate) struct LuaLoader {
    /// Lua instance.
    lua: Lua,
    /// Database of shapes, puzzles, etc.
    db: Arc<Mutex<LibraryDb>>,
    /// Handle to the Lua logger.
    logger: LuaLogger,
}
impl LuaLoader {
    /// Initializes a Lua runtime and loads all the functionality of the
    /// `hyperpuzzle` Lua API.
    pub fn new(db: Arc<Mutex<LibraryDb>>, logger: LuaLogger) -> Self {
        // SAFETY: We need the debug library to get traceback info for better error
        // reporting. We use Lua sandboxing functionality so the user should never
        // be able to access the debug module.
        let lua = Lua::new_with(
            mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH,
            LuaOptions::new(),
        )
        .expect("error initializing Lua runtime");

        // Registry library.
        lua.set_app_data(Arc::clone(&db));

        let logger_ref2 = logger.clone();

        // IIFE to mimic try_block
        (|| {
            // Monkeypatch builtin `type` function.
            let globals = lua.globals();
            let type_fn = |_lua, v| Ok(lua_type_name(&v));
            globals.raw_set("type", lua.create_function(type_fn)?)?;

            if crate::CAPTURE_LUA_OUTPUT {
                globals.raw_set("print", logger.lua_info_fn(&lua)?)?;
                lua.set_warning_function(move |lua, msg, _to_continue| {
                    Ok(logger.warn(crate::lua::current_filename(lua), msg))
                });
            }

            for (module_name, module_source) in LUA_MODULES {
                log::info!("Loading Lua module {module_name:?}");
                if let Err(e) = lua.load(*module_source).set_name(*module_name).exec() {
                    panic!("error loading Lua module {module_name:?}:\n\n{e}\n\n");
                }
            }

            // Grab the sandbox environment so we can insert our custom globals.
            let sandbox: LuaTable<'_> = lua.globals().get("SANDBOX_ENV")?;

            // Constants
            sandbox.raw_set("_PUZZLE_ENGINE", crate::PUZZLE_ENGINE_VERSION_STRING)?;
            sandbox.raw_set("AXES", lua_axes_table(&lua)?)?;

            // Database
            sandbox.raw_set("shapes", LuaShapeDb)?;
            sandbox.raw_set("axis_systems", LuaAxisSystemDb)?;
            sandbox.raw_set("twist_systems", LuaTwistSystemDb)?;
            sandbox.raw_set("puzzles", LuaPuzzleDb)?;

            // Constructors
            let vec_fn = |_lua, LuaVectorFromMultiValue(v)| Ok(LuaVector(v));
            sandbox.raw_set("vec", lua.create_function(vec_fn)?)?;
            let mvec_fn = |_lua, m: LuaMultivector| Ok(m);
            sandbox.raw_set("mvec", lua.create_function(mvec_fn)?)?;
            let plane_fn = LuaManifold::construct_plane;
            sandbox.raw_set("plane", lua.create_function(plane_fn)?)?;
            let sphere_fn = LuaManifold::construct_sphere;
            sandbox.raw_set("sphere", lua.create_function(sphere_fn)?)?;
            let cd_fn = |_lua, t| LuaSymmetry::construct_from_table(t);
            sandbox.raw_set("cd", lua.create_function(cd_fn)?)?;
            let refl_fn = LuaTransform::construct_reflection;
            sandbox.raw_set("refl", lua.create_function(refl_fn)?)?;
            let rot_fn = LuaTransform::construct_rotation;
            sandbox.raw_set("rot", lua.create_function(rot_fn)?)?;

            LuaResult::Ok(())
        })()
        .expect("error initializing Lua environment");

        let logger = logger_ref2;
        LuaLoader { lua, db, logger }
    }

    /// Loads all files that have not yet been loaded.
    pub fn load_all_files(&self) {
        let files = self.db.lock().files.values().cloned().collect_vec();
        for file in files {
            if !file.is_loaded() {
                if let Err(e) = self.load_file(&file.name) {
                    let e = e.context(format!("error loading file {:?}", file.name));
                    self.logger.error(Some(file.name.clone()), e.to_string());
                }
            }
        }
    }

    /// Loads a file if it has not yet been loaded, and then returns the file's
    /// export table.
    fn load_file<'lua>(&'lua self, filename: &str) -> LuaResult<LuaTable<'lua>> {
        let db = self.db.lock();

        // If the file doesn't exist, return an error.
        let Some(file) = db.files.get(filename) else {
            return Err(LuaError::external(format!("no such file {filename:?}")));
        };
        let file = Arc::clone(file);

        // If the file was already loaded, then return that.
        let mut load_state = file.load_state.lock();
        match &*load_state {
            LibraryFileLoadState::Unloaded => (), // Good! We're about to load it.
            LibraryFileLoadState::Loading(_) => {
                return Err(LuaError::external(format!(
                    "recursive dependency on file {filename:?}",
                )));
            }
            LibraryFileLoadState::Done(load_result) => {
                return match load_result {
                    Ok(f) => self.lua.registry_value(&f.exports),
                    Err(e) => Err(e.clone()),
                };
            }
        }

        let sandbox_env: LuaTable<'_> = self.call_global("make_sandbox", filename)?;
        let exports_table = self.lua.create_registry_value(sandbox_env.clone())?;

        // There must be no way to exit the function during this block.
        {
            *load_state =
                LibraryFileLoadState::Loading(LibraryFileLoadResult::with_exports(exports_table));
            // Set the currently-loading file.
            let old_file = self.lua.set_app_data::<Arc<LibraryFile>>(Arc::clone(&file));

            // Unlock the mutexes before we execute user code.
            drop(load_state);
            drop(db);

            let chunk = self
                .lua
                .load(&file.contents)
                .set_name(filename)
                .set_environment(sandbox_env.clone());
            let exec_result = chunk.exec();

            match old_file {
                Some(f) => self.lua.set_app_data::<Arc<LibraryFile>>(f),
                None => self.lua.remove_app_data::<Arc<LibraryFile>>(),
            };
            match exec_result {
                Ok(()) => {
                    {
                        let LibraryFileLoadResult {
                            dependencies: _,

                            exports: _,

                            shapes,
                            axis_systems,
                            twist_systems,
                            puzzles,
                        } = &*file.as_loading()?;

                        let mut db = self.db.lock();
                        let kv = |k: &String| (k.clone(), Arc::clone(&file));
                        db.shapes.extend(shapes.keys().map(kv));
                        db.axis_systems.extend(axis_systems.keys().map(kv));
                        db.twist_systems.extend(twist_systems.keys().map(kv));
                        db.puzzles.extend(puzzles.keys().map(kv));
                    }

                    file.load_state.lock().complete_ok(&self.lua)
                }
                Err(e) => Err(file.load_state.lock().complete_err(e)),
            }
        }
    }

    /// Builds a puzzle, or returns a cached puzzle if it has already been
    /// built.
    pub fn build_puzzle(&self, id: &str) -> Result<Arc<crate::Puzzle>> {
        let result = LibraryDb::build_puzzle(&self.lua, &id);
        if let Err(e) = &result {
            let file = LibraryDb::get(&self.lua)
                .ok()
                .and_then(|lib| Some(lib.lock().puzzles.get(id)?.name.clone()));
            self.logger.error(file, e);
        }
        result
    }

    fn call_global<'lua, A: IntoLuaMulti<'lua>, R: FromLuaMulti<'lua>>(
        &'lua self,
        func_name: &str,
        args: A,
    ) -> LuaResult<R> {
        self.lua
            .globals()
            .get::<_, LuaFunction<'_>>(func_name)?
            .call::<A, R>(args)
    }

    #[cfg(test)]
    pub fn run_test_suite(&self, filename: &str, contents: &str) {
        self.db
            .lock()
            .add_file(filename.to_string(), None, contents.to_string());
        let env = self.load_file(filename).expect("error loading test file");

        let mut ran_any_tests = false;
        for pair in env.pairs::<LuaValue<'_>, LuaValue<'_>>() {
            let (k, v) = pair.unwrap();
            let Ok(name) = self.lua.unpack::<String>(k) else {
                continue;
            };
            let Ok(function) = self.lua.unpack::<LuaFunction<'_>>(v) else {
                continue;
            };
            if name.starts_with("test_") {
                ran_any_tests = true;
                println!("Running {name:?} ...");
                if let Err(e) = function.call::<(), ()>(()) {
                    panic!("{e}");
                }
            }
        }
        assert!(ran_any_tests, "no tests ran!");
    }
}

fn lua_axes_table(lua: &Lua) -> LuaResult<LuaTable<'_>> {
    let axes_table = lua.create_table()?;
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate().take(6) {
        axes_table.set(i, c.to_string())?;
        axes_table.set(c.to_string(), i)?;
        axes_table.set(c.to_ascii_lowercase().to_string(), i)?;
    }
    seal_table(lua, &axes_table)?;
    Ok(axes_table)
}

use std::sync::Arc;

use eyre::Result;
use itertools::Itertools;
use mlua::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::library::{LibraryDb, LibraryFile, LibraryFileLoadResult, LibraryFileLoadState};

macro_rules! lua_module {
    ($filename:literal) => {
        // Start with an `=` instead of an `@` so the logger skips this file
        (concat!("=[HSC] ", $filename), include_str!($filename))
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
        let lua = Lua::new_with(
            mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH,
            LuaOptions::new(),
        )
        .expect("error initializing Lua runtime");

        // Register library.
        lua.set_app_data(Arc::clone(&db));

        let logger2 = logger.clone();

        // IIFE to mimic try_block
        (|| {
            let logger = logger2;

            // Monkeypatch builtin `type` function.
            let globals = lua.globals();
            let type_fn = |_lua, v| Ok(lua_type_name(&v));
            globals.raw_set("type", lua.create_function(type_fn)?)?;

            // Monkeypatch math lib.
            let math: LuaTable<'_> = globals.get("math")?;
            let round_fn = lua.create_function(|_lua, x: LuaNumber| Ok(x.round()))?;
            math.raw_set("round", round_fn)?;
            // TODO: support other numbers
            let eq_fn = |_lua, (a, b): (LuaNumber, LuaNumber)| Ok(hypermath::approx_eq(&a, &b));
            math.raw_set("eq", lua.create_function(eq_fn)?)?;
            let neq_fn = |_lua, (a, b): (LuaNumber, LuaNumber)| Ok(!hypermath::approx_eq(&a, &b));
            math.raw_set("neq", lua.create_function(neq_fn)?)?;

            if crate::CAPTURE_LUA_OUTPUT {
                let logger2 = logger.clone();
                globals.raw_set("print", logger2.lua_info_fn(&lua)?)?;
                lua.set_warning_function(move |lua, msg, _to_continue| {
                    logger2.warn(lua, msg);
                    Ok(())
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

            // Imports
            let db2 = Arc::clone(&db);
            let require_fn =
                move |lua, filename| LuaLoaderRef { lua, db: &db2 }.load_file_dependency(filename);
            sandbox.raw_set("require", lua.create_function(require_fn)?)?;

            // Database
            sandbox.raw_set("puzzles", LuaPuzzleDb)?;
            sandbox.raw_set("color_systems", LuaColorSystemDb)?;

            // `blade` constructors
            let vec_fn = |lua, LuaVectorFromMultiValue(v)| LuaBlade::from_vector(lua, v);
            sandbox.raw_set("vec", lua.create_function(vec_fn)?)?;
            let point_fn = |lua, LuaPointFromMultiValue(v)| LuaBlade::from_point(lua, v);
            sandbox.raw_set("point", lua.create_function(point_fn)?)?;
            let blade_fn = |_lua, b: LuaBlade| Ok(b);
            sandbox.raw_set("blade", lua.create_function(blade_fn)?)?;
            let plane_fn = |lua, LuaHyperplaneFromMultivalue(h)| LuaBlade::from_hyperplane(lua, &h);
            sandbox.raw_set("plane", lua.create_function(plane_fn)?)?;

            // `symmetry` constructors
            let cd_fn = |_lua, v| LuaSymmetry::construct_from_cd(v);
            sandbox.raw_set("cd", lua.create_function(cd_fn)?)?;
            let symmetry_fn = LuaSymmetry::construct_from_generators;
            sandbox.raw_set("symmetry", lua.create_function(symmetry_fn)?)?;

            // `transform` constructors
            let ident_fn = LuaTransform::construct_identity;
            sandbox.raw_set("ident", lua.create_function(ident_fn)?)?;
            let refl_fn = LuaTransform::construct_reflection;
            sandbox.raw_set("refl", lua.create_function(refl_fn)?)?;
            let rot_fn = LuaTransform::construct_rotation;
            sandbox.raw_set("rot", lua.create_function(rot_fn)?)?;

            // `region` constants
            sandbox.raw_set("REGION_ALL", LuaRegion::Everything)?;
            sandbox.raw_set("REGION_NONE", LuaRegion::Nothing)?;

            LuaResult::Ok(())
        })()
        .expect("error initializing Lua environment");

        LuaLoader { lua, db, logger }
    }

    /// Loads all files that have not yet been loaded.
    pub fn load_all_files(&self) {
        let files = self.db.lock().files.values().cloned().collect_vec();
        for file in files {
            if !file.is_loaded() {
                self.try_load_file(&file.name);
            }
        }
    }

    fn as_ref(&self) -> LuaLoaderRef<'_, '_> {
        LuaLoaderRef {
            lua: &self.lua,
            db: &self.db,
        }
    }

    /// Builds a puzzle, or returns a cached puzzle if it has already been
    /// built.
    pub fn build_puzzle(&self, id: &str) -> Result<Arc<crate::Puzzle>> {
        let result = LibraryDb::build_puzzle(&self.lua, id);
        if let Err(e) = &result {
            let file = LibraryDb::get(&self.lua)
                .ok()
                .and_then(|lib| Some(lib.lock().puzzles.get(id)?.name.clone()));
            self.logger.error(file, e);
        }
        result
    }
    /// Loads a file if it has not yet been loaded. If loading fails, an error
    /// is reported to the Lua console.
    fn try_load_file(&self, filename: &str) {
        if let Err(e) = self.as_ref().load_file(filename) {
            self.logger.error(
                Some(filename.to_string()),
                e.clone()
                    .context(format!("error loading file {filename:?}")),
            );
        }
    }

    #[cfg(test)]
    pub fn run_test_suite(&self, filename: &str, contents: &str) {
        self.db
            .lock()
            .add_file(filename.to_string(), None, contents.to_string());
        let env = self
            .as_ref()
            .load_file(filename)
            .expect("error loading test file");

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

                if let Some(digit) = name
                    .strip_suffix(&['d', 'D'])
                    .and_then(|s| s.chars().last())
                    .filter(|c| c.is_ascii_digit())
                {
                    let ndim: u8 = digit.to_string().parse().expect("bad ndim for test");
                    println!("Running {name:?} in {ndim}D space ...");
                    LuaSpace(hypershape::flat::Space::new(ndim))
                        .with_this_as_global_space(&self.lua, || function.call::<(), ()>(()))
                } else {
                    println!("Running {name:?} ...");
                    function.call::<(), ()>(())
                }
                .expect("test failed")
            }
        }
        assert!(ran_any_tests, "no tests ran!");
    }
}

struct LuaLoaderRef<'lua, 'db> {
    lua: &'lua Lua,
    db: &'db Mutex<LibraryDb>,
}
impl<'lua> LuaLoaderRef<'lua, '_> {
    /// Loads a file if it has not yet been loaded, and then returns the file's
    /// export table.
    fn load_file(&self, filename: &str) -> LuaResult<LuaTable<'lua>> {
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

        let make_sandbox_fn: LuaFunction<'_> = self.lua.globals().get("make_sandbox")?;
        let sandbox_env: LuaTable<'_> = make_sandbox_fn.call(filename)?;
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
                .set_name(format!("@{filename}"))
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
                            exports: _,

                            puzzles,
                            color_systems,
                        } = &*file.as_loading()?;

                        let mut db = self.db.lock();
                        let kv = |k: &String| (k.clone(), Arc::clone(&file));
                        db.puzzles.extend(puzzles.keys().map(kv));
                        db.color_systems.extend(color_systems.keys().map(kv));
                    }

                    file.load_state.lock().complete_ok(self.lua)
                }
                Err(e) => Err(file.load_state.lock().complete_err(e)),
            }
        }
    }

    /// Records a file dependency, then loads it using `load_file()`.
    fn load_file_dependency(&self, mut filename: String) -> LuaResult<LuaTable<'lua>> {
        if !filename.ends_with(".lua") {
            filename.push_str(".lua");
        }

        let current_file = LibraryFile::get_current(self.lua)?;

        // Handle basic globs.
        if let Some(prefix) = filename.strip_suffix("/*.lua") {
            // Assemble a list of files to load.
            let files_to_load = self
                .db
                .lock()
                .files
                .iter()
                // Don't depend on the current file.
                .filter(|(_file_key, lib_file)| !Arc::ptr_eq(lib_file, &current_file))
                // Depend on files that match the glob pattern.
                .filter_map(|(k, v)| Some((k, k.strip_prefix(prefix)?.strip_prefix('/')?, v)))
                .map(|(file_key, table_key, lib_file)| {
                    // Record the dependency.
                    lib_file.dependents.lock().push(Arc::clone(&current_file));
                    // Remove the `.lua` suffix for the table key.
                    let table_key = table_key.strip_suffix(".lua").unwrap_or(table_key);
                    (table_key.to_owned(), file_key.to_owned())
                })
                .collect_vec();

            // Unlock the database, then load all those files.
            let return_table = self.lua.create_table()?;
            let mut ret = Ok(return_table.clone());
            for (table_key, file_key) in files_to_load {
                match self.load_file(&file_key) {
                    Ok(exports) => return_table.raw_set(table_key, exports)?,
                    Err(e) => ret = Err(e),
                }
            }
            return ret;
        }

        if let Some(dependency) = self.db.lock().files.get(&filename) {
            // Record the dependency.
            dependency.dependents.lock().push(current_file);
        }
        self.load_file(&filename)
    }
}

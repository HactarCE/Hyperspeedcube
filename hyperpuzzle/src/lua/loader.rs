use std::sync::Arc;

use eyre::Result;
use itertools::Itertools;
use mlua::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::{library::LibraryDb, LibraryFileLoadState};

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
///
/// The contents of this struct are refcounted, so it's cheap to clone.
#[derive(Debug, Clone)]
pub(crate) struct LuaLoader {
    /// Lua instance.
    lua: Lua,
    /// Database of shapes, puzzles, etc.
    pub(crate) db: Arc<Mutex<LibraryDb>>,
    /// Handle to the Lua logger.
    logger: LuaLogger,
}
impl LuaLoader {
    /// Initializes a Lua runtime and loads all the functionality of the
    /// `hyperpuzzle` Lua API.
    pub fn new(db: Arc<Mutex<LibraryDb>>, logger: LuaLogger) -> LuaResult<Self> {
        let lua = Lua::new_with(
            mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH,
            LuaOptions::new(),
        )
        .expect("error initializing Lua runtime");

        let this = Self { lua, db, logger };
        let Self { lua, db, logger } = &this; // still use local variables

        // Register library.
        lua.set_app_data(Arc::clone(&db));

        // Monkeypatch builtin `type` function.
        let globals = lua.globals();
        globals.raw_set(
            "type",
            lua.create_function(|_lua, v| Ok(lua_type_name(&v)))?,
        )?;

        // Monkeypatch math lib.
        let math: LuaTable = globals.get("math")?;
        let round_fn = lua.create_function(|_lua, x: LuaNumber| Ok(x.round()))?;
        math.raw_set("round", round_fn)?;
        // TODO: support other numbers
        let eq_fn = lua.create_function(|_lua, (a, b): (LuaNumber, LuaNumber)| {
            Ok(hypermath::approx_eq(&a, &b))
        });
        math.raw_set("eq", eq_fn?)?;
        let neq_fn = lua.create_function(|_lua, (a, b): (LuaNumber, LuaNumber)| {
            Ok(!hypermath::approx_eq(&a, &b))
        });
        math.raw_set("neq", neq_fn?)?;

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
        let sandbox: LuaTable = lua.globals().get("SANDBOX_ENV")?;

        // Constants
        sandbox.raw_set("_PUZZLE_ENGINE", crate::PUZZLE_ENGINE_VERSION_STRING)?;
        sandbox.raw_set("AXES", lua_axes_table(&lua)?)?;

        // Imports
        let this2 = this.clone();
        let index_metamethod = lua.create_function(
            move |_lua, (_lib_table, index_string): (LuaTable, String)| {
                this2.load_file(index_string)
            },
        )?;
        let lib_table =
            crate::lua::create_sealed_table_with_index_metamethod(lua, index_metamethod)?;
        sandbox.raw_set("lib", lib_table)?;

        // Database
        sandbox.raw_set("puzzles", LuaPuzzleDb)?;
        sandbox.raw_set("puzzle_generators", LuaPuzzleGeneratorDb)?;
        sandbox.raw_set("color_systems", LuaColorSystemDb)?;

        // `blade` constructors
        let vec_fn =
            lua.create_function(|lua, LuaVectorFromMultiValue(v)| LuaBlade::from_vector(lua, v));
        sandbox.raw_set("vec", vec_fn?)?;
        let point_fn =
            lua.create_function(|lua, LuaPointFromMultiValue(v)| LuaBlade::from_point(lua, v))?;
        sandbox.raw_set("point", point_fn)?;
        let blade_fn = lua.create_function(|_lua, b: LuaBlade| Ok(b))?;
        sandbox.raw_set("blade", blade_fn)?;
        let plane_fn = lua.create_function(|lua, LuaHyperplaneFromMultivalue(h)| {
            LuaBlade::from_hyperplane(lua, &h)
        })?;
        sandbox.raw_set("plane", plane_fn)?;

        // `symmetry` constructors
        let cd_fn = LuaSymmetry::construct_from_cd;
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

        LuaResult::Ok(this)
    }

    /// Loads all files that have not yet been loaded.
    pub fn load_all_files(&self) {
        let mut db = self.db.lock();

        for dirname in db.directories.iter().cloned().collect_vec() {
            db.files
                .entry(format!("{dirname}.lua"))
                .or_insert_with(|| crate::LibraryFile {
                    name: dirname.to_string(),
                    path: None,
                    contents: None,
                    load_state: LibraryFileLoadState::Unloaded,
                });
        }

        let filenames = db.files.keys().cloned().collect_vec();
        drop(db);
        for filename in filenames {
            self.try_load_file(&filename);
        }
    }

    /// Builds a puzzle, or returns a cached puzzle if it has already been
    /// built.
    pub fn build_puzzle(&self, id: &str) -> Result<Arc<crate::Puzzle>> {
        let result = LibraryDb::build_puzzle(&self.lua, id);
        if let Err(e) = &result {
            let filename = LibraryDb::get(&self.lua).ok().and_then(|lib| {
                Some(
                    crate::TAGS
                        .filename(&lib.lock().puzzles.get(id)?.tags)
                        .to_owned(),
                )
            });
            self.logger.error(filename, e);
        }
        result
    }
    /// Loads a file if it has not yet been loaded. If loading fails, an error
    /// is reported to the Lua console.
    fn try_load_file(&self, filename: &str) {
        let name = filename.strip_suffix(".lua").unwrap_or(filename);
        if let Err(e) = self.load_file(name.to_string()) {
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
            .load_file(filename.to_string())
            .expect("error loading test file");

        let mut ran_any_tests = false;
        for pair in env.pairs::<LuaValue, LuaValue>() {
            let (k, v) = pair.unwrap();
            let Ok(name) = self.lua.unpack::<String>(k) else {
                continue;
            };
            let Ok(function) = self.lua.unpack::<LuaFunction>(v) else {
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
                        .with_this_as_global_space(&self.lua, || function.call::<()>(()))
                } else {
                    println!("Running {name:?} ...");
                    function.call::<()>(())
                }
                .expect("test failed")
            }
        }
        assert!(ran_any_tests, "no tests ran!");
    }

    /// Loads a file if it has not yet been loaded, and then returns the file's
    /// export table.
    fn load_file(&self, name: String) -> LuaResult<LuaTable> {
        let filename = format!("{name}.lua");
        let dirname = name;

        let mut db = self.db.lock();

        // If the file doesn't exist, return an error.
        let Some(file) = db.files.get_mut(&filename) else {
            return Err(LuaError::external(format!("no such file {filename:?}")));
        };

        // Check if the file was already loaded or is currently being loaded..
        match &file.load_state {
            LibraryFileLoadState::Unloaded => (), // happy path
            LibraryFileLoadState::Loading => {
                return Err(LuaError::external(format!(
                    "recursive dependency on file {filename:?}",
                )));
            }
            LibraryFileLoadState::Loaded(result) => return result.clone(),
        }

        let make_sandbox_fn: LuaFunction = self.lua.globals().get("make_sandbox")?;
        let sandbox_env: LuaTable = make_sandbox_fn.call(filename.clone())?;
        let sandbox_env2: LuaTable = sandbox_env.clone(); // pointer to same table
        let this = self.clone();
        let index_metamethod =
            self.lua
                .create_function(move |lua, (_table, index): (LuaTable, LuaValue)| {
                    if let Some(exported_value) =
                        sandbox_env2.get::<Option<LuaValue>>(index.clone())?
                    {
                        Ok(exported_value)
                    } else if let Ok(index_string) = String::from_lua(index, lua) {
                        let subfilename = format!("{dirname}/{index_string}");
                        Ok(LuaValue::Table(this.load_file(subfilename)?))
                    } else {
                        Ok(LuaNil)
                    }
                })?;
        let exports_table =
            crate::lua::create_sealed_table_with_index_metamethod(&self.lua, index_metamethod)?;

        let Some(file_contents) = file.contents.clone() else {
            return Ok(exports_table); // fake file! (probably a directory)
        };

        // There must be no way to exit the function during this block, or else
        // the file load result will never be stored and the file will be
        // eternally loading.
        {
            // Mark the file as loading.
            file.start_loading();

            // Unlock the mutexes before we execute user code.
            drop(db);

            let chunk = self
                .lua
                .load(&file_contents)
                .set_name(format!("@{filename}"))
                .set_environment(sandbox_env.clone());

            let exec_result = chunk.exec().map(|()| exports_table);

            match self.db.lock().files.get_mut(&filename) {
                Some(file) => {
                    file.finish_loading(exec_result.clone());
                    exec_result
                }
                None => {
                    // this shouldn't ever happen
                    let e = "file disappeared during loading";
                    log::error!("{e}");
                    Err(LuaError::external(e))
                }
            }
        }
    }
}

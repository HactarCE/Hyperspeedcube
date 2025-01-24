use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hyperpuzzle_core::{Catalog, LogLine, Logger};
use itertools::Itertools;
use log::Level;
use mlua::prelude::*;
use parking_lot::Mutex;

mod files;

use files::{LuaFile, LuaFileLoadState};

macro_rules! lua_module {
    ($filename:literal) => {
        // Start with an `=` instead of an `@` so the logger skips this file
        (concat!("=[HSC] ", $filename), include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[
    lua_module!("../prelude/pprint.lua"),
    lua_module!("../prelude/utils.lua"),
    lua_module!("../prelude/sandbox.lua"),
];

/// Lua runtime for loading Lua files.
///
/// The contents of this struct are refcounted, so it's cheap to clone.
#[derive(Clone)]
pub(crate) struct LuaLoader {
    /// Lua instance.
    pub(crate) lua: Lua,
    /// Database of shapes, puzzles, etc.
    pub(crate) catalog: Catalog,

    /// File contents by file path.
    files: Arc<Mutex<HashMap<String, LuaFile>>>,
}
impl LuaLoader {
    /// Initializes a Lua runtime and loads all the functionality of the
    /// `hyperpuzzle` Lua API.
    pub fn new(catalog: &Catalog, logger: &Logger) -> LuaResult<Self> {
        let lua = Lua::new_with(
            mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::UTF8 | mlua::StdLib::MATH,
            LuaOptions::new(),
        )
        .expect("error initializing Lua runtime");
        let files = Arc::new(Mutex::new(HashMap::new()));
        let this = Self {
            lua: lua.clone(),
            catalog: catalog.clone(),
            files,
        };

        // Register catalog.
        lua.set_app_data::<Catalog>(this.catalog.clone());

        super::env::monkeypatch_lua_environment(&lua)?;
        super::env::set_logger(&lua, logger)?;

        for (module_name, module_source) in LUA_MODULES {
            log::info!("Loading Lua module {module_name:?}");
            if let Err(e) = lua.load(*module_source).set_name(*module_name).exec() {
                panic!("error loading Lua module {module_name:?}:\n\n{e}\n\n");
            }
        }

        // Grab the sandbox environment so we can insert our custom globals.
        let sandbox: LuaTable = lua.globals().get("SANDBOX_ENV")?;

        super::env::init_lua_environment(&this, &sandbox)?;

        LuaResult::Ok(this)
    }

    /// Returns the catalog.
    pub fn get_catalog(lua: &Lua) -> Catalog {
        lua.app_data_ref::<Catalog>()
            .expect("Lua runtime does not have catalog")
            .clone()
    }

    /// Adds a Lua file. It will not immediately be loaded.
    ///
    /// If the filename conflicts with an existing one, then the existing file
    /// will be unloaded and overwritten.
    pub fn add_file(&self, filename: String, path: Option<PathBuf>, contents: String) {
        let mut files = self.files.lock();

        let mut dirname = filename.as_str();

        // Add the equivalent of a `mod.rs` if there is no such file already. If
        // we later add such a file, then this will get overwritten.
        while let Some((prefix, _)) = dirname.rsplit_once('/') {
            dirname = prefix;
            files
                .entry(format!("{dirname}.lua"))
                .or_insert_with(|| LuaFile {
                    name: dirname.to_string(),
                    path: None,
                    contents: None,
                    load_state: LuaFileLoadState::Unloaded,
                });
        }

        files.insert(
            filename.clone(),
            LuaFile {
                name: filename,
                path,
                contents: Some(contents),
                load_state: LuaFileLoadState::Unloaded,
            },
        );
    }

    /// Reads a file from the disk and adds it to the Lua loader using
    /// [`Self::add_file()`].
    pub fn read_file(&self, filename: String, path: PathBuf) {
        let file_path = path.strip_prefix(".").unwrap_or(&path);
        match std::fs::read_to_string(file_path) {
            Ok(contents) => self.add_file(filename, Some(file_path.to_path_buf()), contents),
            Err(e) => log::error!("error loading {file_path:?}: {e}"),
        }
    }

    pub fn read_builtin_directory(&self) {
        let mut stack = vec![crate::LUA_BUILTIN_DIR.clone()];
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => {
                        stack.push(subdir.clone());
                    }
                    include_dir::DirEntry::File(file) => {
                        if file.path().extension().is_some_and(|ext| ext == "lua") {
                            let name = relative_path_to_filename(file.path());
                            match file.contents_utf8() {
                                Some(contents) => self.add_file(name, None, contents.to_string()),
                                None => log::error!("Error loading built-in file {name}"),
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn read_directory(&self, directory: &Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "lua") {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        let name = relative_path_to_filename(relative_path);
                        self.read_file(name, path.to_owned());
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }

    /// Loads all files that haven't yet been loaded.
    pub fn load_all_files(&self, logger: &Logger) {
        super::env::set_logger(&self.lua, logger);
        let filenames = self.files.lock().keys().cloned().collect_vec();
        for filename in filenames {
            let name = filename.strip_suffix(".lua").unwrap_or(&filename);
            if let Err(e) = self.load_file(name.to_string()) {
                logger.log(LogLine {
                    level: Level::Error,
                    file: Some(filename.to_string()),
                    msg: format!("error loading file {filename:?}: {e:#}"),
                    traceback: None,
                });
            }
        }
    }

    #[cfg(test)]
    pub fn run_test_suite(&self, filename: &str, contents: &str) {
        use crate::lua::LuaSpace;

        self.add_file(filename.to_string(), None, contents.to_string());
        let env = self
            .load_file(filename.to_string())
            .expect("error loading test file");
        let raw_exports = env
            .metatable()
            .expect("no metatable on sandbox env")
            .raw_get::<LuaTable>("env")
            .expect("no __index metamethod on sandbox env");

        let mut ran_any_tests = false;
        for pair in raw_exports.pairs::<LuaValue, LuaValue>() {
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
    pub(super) fn load_file(&self, name: String) -> LuaResult<LuaTable> {
        let filename = if name.ends_with(".lua") {
            name.clone()
        } else {
            format!("{name}.lua")
        };
        let dirname = name;

        let mut files = self.files.lock();

        // If the file doesn't exist, return an error.
        let Some(file) = files.get_mut(&filename) else {
            return Err(LuaError::external(format!("no such file {filename:?}")));
        };

        // Check if the file was already loaded or is currently being loaded..
        match &file.load_state {
            LuaFileLoadState::Unloaded => (), // happy path
            LuaFileLoadState::Loading => {
                return Err(LuaError::external(format!(
                    "recursive dependency on file {filename:?}",
                )));
            }
            LuaFileLoadState::Loaded(result) => return result.clone(),
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

        // Save `sandbox_env` for usage in tests.
        #[cfg(test)]
        exports_table
            .metatable()
            .expect("missing metatable")
            .raw_set("env", &sandbox_env)?;

        let Some(file_contents) = file.contents.clone() else {
            return Ok(exports_table); // fake file! (probably a directory)
        };

        // There must be no way to exit the function during this block, or else
        // the file load result will never be stored and the file will be
        // eternally loading.
        {
            // Mark the file as loading.
            file.start_loading();

            // Unlock the mutex before we execute user code.
            drop(files);

            let chunk = self
                .lua
                .load(&file_contents)
                .set_name(format!("@{filename}"))
                .set_environment(sandbox_env.clone());

            let exec_result = chunk.exec().map(|()| exports_table);

            files = self.files.lock();
            match files.get_mut(&filename) {
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

/// Canonicalizes a relative file path to make a suitable filename.
fn relative_path_to_filename(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .join("/")
}

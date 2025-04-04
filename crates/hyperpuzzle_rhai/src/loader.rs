use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use itertools::Itertools;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rhai::packages::Package;
use rhai::{AST, Engine, EvalAltResult, Module, ParseError, Scope, Shared};
use thread_local::ThreadLocal;

use crate::package::HyperpuzzlePackage;

/// List of built-in and user-defined Rhai files.
#[derive(Debug, Default)]
struct FileList(HashMap<String, String>);
impl FileList {
    fn load_builtin_files(&mut self) {
        let mut stack = vec![crate::RHAI_BUILTIN_DIR.clone()];
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => stack.push(subdir.clone()),
                    include_dir::DirEntry::File(file) => {
                        let path = file.path();
                        if path.extension().is_some_and(|ext| ext == "rhai") {
                            match file.contents_utf8().map(str::to_owned) {
                                Some(contents) => self.add_file(path.to_owned(), contents),
                                None => log::error!("error loading built-in file {path:?}"),
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "hyperpaths")]
    fn load_from_directory(&mut self, directory: &Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "rhai") {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        match std::fs::read_to_string(path) {
                            Ok(contents) => self.add_file(relative_path.to_owned(), contents),
                            Err(e) => log::error!("error loading file {relative_path:?}: {e}"),
                        }
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }

    fn add_file(&mut self, path: PathBuf, contents: String) {
        let path_string = path
            .with_extension("")
            .components()
            .map(|path_component| path_component.as_os_str().to_string_lossy())
            .join("/")
            .chars()
            .filter(|&c| c != '"' && c != '\\') // dubious chars
            .collect();
        self.0.insert(path_string, contents);
    }
}

/// Module resolver for built-in and user-defined Rhai files.
///
/// Modules are cached so that they are only loaded once.
#[derive(Debug, Default)]
struct ModuleResolver {
    asts: HashMap<String, Result<AST, ParseError>>,
    cache: RwLock<HashMap<String, ModuleResult>>,
}
impl ModuleResolver {
    fn from_files<P: Package + Sync>(package: &P, files: &FileList) -> Self {
        let tl = ThreadLocal::new();
        let asts = files
            .0
            .par_iter()
            .map(|(file_path, file_contents)| {
                let engine = tl.get_or(|| {
                    let mut e = Engine::new_raw();
                    package.register_into_engine(&mut e);
                    e
                });
                match engine.compile(file_contents) {
                    Ok(mut ast) => {
                        ast.set_source(file_path);
                        (file_path.clone(), Ok(ast))
                    }
                    Err(e) => (file_path.clone(), Err(e)),
                }
            })
            .collect();
        let cache = RwLock::new(HashMap::new());
        Self { asts, cache }
    }
}
impl rhai::ModuleResolver for ModuleResolver {
    fn resolve(
        &self,
        engine: &rhai::Engine,
        source: Option<&str>,
        path: &str,
        pos: rhai::Position,
    ) -> Result<Shared<Module>, Box<EvalAltResult>> {
        // Load relative paths from source if `path` does not start with `/`.
        let module_name: Cow<'_, str> = match path.strip_prefix('/') {
            Some(abs_path) => abs_path.into(),
            None => match source.and_then(|s| s.rsplit_once('/')) {
                Some((parent_dir, _)) => format!("{parent_dir}/{path}").into(),
                None => path.into(),
            },
        };

        // Return cached module if it has already been loaded.
        if let Some(cached_module_result) = self.cache.read().get(&*module_name) {
            return match cached_module_result {
                ModuleResult::Ok(module) => Ok(module.clone()),
                ModuleResult::Loading => Err(Box::new(EvalAltResult::from(format!(
                    "circular dependency on module '{module_name}'"
                )))),
                ModuleResult::Err => Err(Box::new(EvalAltResult::from(format!(
                    "dependency on module '{module_name}' which encountered an error"
                )))),
            };
        }

        // Mark the module as "loading" to catch circular dependencies.
        self.cache
            .write()
            .insert((*module_name).to_owned(), ModuleResult::Loading);

        // Evaluate the module.
        let result = match self.asts.get(&*module_name).cloned() {
            None => Err(Box::new(EvalAltResult::ErrorModuleNotFound(
                (*module_name).to_owned(),
                pos,
            ))),
            Some(ast_result) => match ast_result {
                Ok(ast) => Module::eval_ast_as_new(Scope::new(), &ast, engine).map(Shared::new),
                Err(ParseError(parse_error_type, parse_error_pos)) => Err(Box::new(
                    EvalAltResult::ErrorParsing(*parse_error_type, parse_error_pos),
                )),
            }
            .map_err(|inner_error| {
                Box::new(EvalAltResult::ErrorInModule(
                    (*module_name).to_owned(),
                    inner_error,
                    pos,
                ))
            }),
        };

        // Cache the result.
        self.cache.write().insert(
            module_name.into_owned(),
            match &result {
                Ok(module) => ModuleResult::Ok(module.clone()),
                Err(_) => ModuleResult::Err,
            },
        );

        result
    }
}

#[derive(Debug, Clone)]
enum ModuleResult {
    Ok(Shared<Module>),
    Loading,
    Err,
}

pub(crate) fn load_files_with_new_engine(
    catalog: &hyperpuzzle_core::Catalog,
    logger: &hyperpuzzle_core::Logger,
) -> rhai::Engine {
    let mut files = FileList::default();

    files.load_builtin_files();
    #[cfg(feature = "hyperpaths")]
    match hyperpaths::rhai_dir() {
        Ok(rhai_dir) => {
            log::info!(
                "reading Rhai files from path {}",
                rhai_dir.to_string_lossy(),
            );
            files.load_from_directory(rhai_dir);
        }
        Err(e) => log::error!("error locating Rhai directory: {e}"),
    }

    let mut engine = rhai::Engine::new();

    let package = HyperpuzzlePackage::new(catalog);
    package.register_into_engine(&mut engine);

    let resolver = ModuleResolver::from_files(&package, &files);
    engine.set_module_resolver(resolver);

    let l = logger.clone();
    engine.on_print(move |s| l.info(s));
    engine.on_debug(|s, src, pos| match src {
        Some(src) => log::debug!("[{src}:{pos}] {s}"),
        None => log::debug!("[{pos}] {s}"),
    });

    // Load files in lexicographic order. The order shouldn't matter, but it's
    // nice to be deterministic.
    for file in files.0.keys().sorted() {
        // SAFETY: quotes and backslashes are disallowed so there's no injection
        // possible here.
        if let Err(e) = engine.eval::<()>(&format!("import \"{file}\"; ()")) {
            println!("error: {e}");
            logger.error(e.to_string());
        }
    }

    engine
}

#[cfg(test)]
pub(crate) fn new_engine() -> rhai::Engine {
    let mut engine = rhai::Engine::new();

    let package = HyperpuzzlePackage::new(&hyperpuzzle_core::Catalog::new());
    package.register_into_engine(&mut engine);

    engine
}

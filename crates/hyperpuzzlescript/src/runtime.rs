use std::{collections::HashMap, path::PathBuf};

use arcstr::ArcStr;
use indexmap::IndexMap;
use itertools::Itertools;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{ast, Error, Warning};

#[derive(Debug, Clone)]
pub enum Output {
    Info(String),
    Warn(Warning),
    Err(Error),
}

pub struct EvalCtx {

}

pub struct File {
    /// File path.
    pub path: String,
    /// File contents.
    pub contents: String,
    /// AST root node.
    pub ast: Result<ast::Node,Vec<Error>>,
    /// Exports.
    pub exports: Option<HashMap<String,()>>,
}

fn builtin_files() -> impl ParallelIterator<File> {

    let mut paths_and_contents = vec![];

        let mut stack = vec![crate::HPS_BUILTIN_DIR.clone()];
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => stack.push(subdir.clone()),
                    include_dir::DirEntry::File(file) => {
                        let path = file.path();
                        if path.extension().is_some_and(|ext| ext == "hps") {
                            match file.contents_utf8().map(str::to_owned) {
                                Some(contents) => paths_and_contents.push((path, contents)),
                                None => log::error!("error loading built-in file {path:?}"),
                            }
                        }
                    }
                }
            }
        }

        let parser = crate::parser::parser(make_input)

        paths_and_contents.par_iter().map(|(name, contents)| File::new(path,contents))
}

/// Script runtime.
#[derive(Debug, Default)]
struct Runtime {
    /// Source file names and contents.
    files: IndexMap<String, String>,
    asts: Vec<ast::Node>,
    output: Vec<Output>,
    // globals: HashMap<String, Value>,
}
impl Runtime {
    fn add_builtin_files(&mut self, output: &mut Vec<Error>) {
        let mut stack = vec![crate::HPS_BUILTIN_DIR.clone()];
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
    fn add_from_directory(&mut self, directory: &std::path::Path) {
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
        self.files.insert(path_string, contents);
    }

    pub fn load_all_files()
}

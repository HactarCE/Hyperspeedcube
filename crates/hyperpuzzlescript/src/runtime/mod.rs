use std::{collections::HashMap, fmt, ops::Index, path::Path};

mod file_store;

use crate::{Error, FileId, Span, Warning, ast};
pub use file_store::FileStore;

#[derive(Debug, Clone)]
pub enum Output {
    Info(String),
    Warn(Warning),
    Err(Error),
}

pub struct EvalCtx {}

/// Script runtime.
#[derive(Debug, Default)]
pub struct Runtime {
    /// Source file names and contents.
    files: FileStore,
    /// AST for each file, indexed by [`FileId`].
    asts: Vec<ast::Node>,
    /// Return value for each file.
    exports: Vec<()>,

    /// Log output.
    logs: Vec<Output>,

    /// Prelude to be imported into every file.
    prelude: HashMap<String, ()>,
}

impl Runtime {
    /// Constructs a new runtime with no files.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the runtime and parses it.
    pub fn add_file(&mut self, path: &Path, contents: String) {
        self.files.add_file(path, contents);
        self.parse_all();
    }

    /// Constructs a runtime with built-in files and user files (if feature
    /// `hyperpaths` is enabled).
    pub fn with_default_files() -> Self {
        let mut ret = Self {
            files: FileStore::with_default_files(),
            ..Self::default()
        };
        ret.parse_all();
        ret
    }

    /// Parses any files that have not yet been parsed.
    fn parse_all(&mut self) {
        for i in self.asts.len()..self.files.len() {
            let file_id = i as FileId;
            let file_contents = self.files.file_contents(file_id).unwrap_or("");
            let ast_node = crate::parse::parse(file_id, file_contents).unwrap_or_else(|errors| {
                self.logs.extend(errors.into_iter().map(Output::Err));
                let span = Span {
                    start: 0,
                    end: 0,
                    context: file_id,
                };
                (ast::NodeContents::Error, span)
            });
            self.asts.push(ast_node);
        }
    }

    /// Returns the top-level AST for a file, or `None` if it doesn't exist.
    pub fn ast(&self, file: FileId) -> Option<&ast::Node> {
        self.asts.get(file as usize)
    }
}

impl Index<Span> for Runtime {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self.files[index]
    }
}

impl ariadne::Cache<FileId> for &Runtime {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source, impl fmt::Debug> {
        self.files.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.files.ariadne_display(*id)
    }
}

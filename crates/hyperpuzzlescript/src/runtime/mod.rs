use std::{collections::HashMap, fmt, ops::Index, path::Path, sync::Arc};

mod ctx;
mod file_store;
mod scope;

use crate::{Error, FileId, Result, Span, Value, Warning, ast};
use arcstr::{ArcStr, Substr};
pub use ctx::EvalCtx;
use file_store::File;
pub use file_store::FileStore;
pub use scope::{BUILTIN_SCOPE, EMPTY_SCOPE, Scope};

/// Script runtime.
#[derive(Default)]
pub struct Runtime {
    /// Source file names and contents.
    files: FileStore,

    /// Prelude to be imported into every file.
    prelude: HashMap<String, ()>,
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

impl Runtime {
    /// Constructs a new runtime with no files.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the runtime.
    pub fn add_file(&mut self, path: &Path, contents: impl Into<ArcStr>) {
        self.files.add_file(path, contents.into())
    }

    /// Constructs a runtime with built-in files and user files (if feature
    /// `hyperpaths` is enabled).
    pub fn with_default_files() -> Self {
        Self {
            files: FileStore::with_default_files(),
            ..Self::default()
        }
    }

    /// Parses any files that have not yet been parsed.
    fn parse_all(&mut self) {
        for i in 0..self.files.len() {
            self.file_ast(i as FileId);
        }
    }

    /// Parses any files that have not yet been parsed and loads any files that
    /// have not yet been loaded.
    pub fn load_all_files(&mut self) {
        for i in 0..self.files.len() {
            self.file_ret(i as FileId);
        }
    }

    fn file_mut(&mut self, file_id: FileId) -> Option<&mut File> {
        Some(self.files.0.get_index_mut(file_id as usize)?.1)
    }

    /// Returns the top-level AST for a file, or `None` if it doesn't exist.
    pub fn file_ast(&mut self, file_id: FileId) -> Option<Arc<ast::Node>> {
        let file = self.file_mut(file_id)?;
        match file.ast.clone() {
            Some(ast) => Some(ast),
            None => {
                let contents = file.contents.clone();
                let ast =
                    crate::parse::parse(file_id as FileId, &contents).unwrap_or_else(|errors| {
                        self.errors(errors);
                        let span = Span {
                            start: 0,
                            end: contents.len() as u32,
                            context: file_id,
                        };
                        (ast::NodeContents::Error, span)
                    });
                let file = self.file_mut(file_id)?;
                file.ast = Some(Arc::new(ast));
                file.ast.clone()
            }
        }
    }

    pub fn file_ret(&mut self, file_id: FileId) -> Option<&Result<Value, ()>> {
        let file = self.file_mut(file_id)?;
        match file.result {
            // extra lookup is necessary to appease borrowchecker
            Some(_) => self.file_mut(file_id)?.result.as_ref(),
            None => {
                let ast = self.file_ast(file_id)?;
                let scope = Scope::new_top_level();
                let result = EvalCtx {
                    scope: &scope,
                    runtime: self,
                }
                .eval(&ast)
                .or_else(Error::try_resolve_return_value)
                .map_err(|e| self.error(e));
                let file = self.file_mut(file_id)?;
                file.result = Some(result);
                file.result.as_ref()
            }
        }
    }

    pub fn substr(&self, span: Span) -> Substr {
        self.files.substr(span)
    }

    pub fn file_name_to_id(&self, name: &str) -> Option<FileId> {
        Some(self.files.0.get_index_of(name)? as FileId)
    }

    pub fn print(&self, s: &str) {
        println!("[INFO] {s}");
    }

    pub fn warn(&self, w: Warning) {
        eprintln!("[WARN] {}", w.to_string(self));
    }

    pub fn error(&self, e: Error) {
        eprintln!("[ERROR] {}", e.to_string(self));
    }
    pub fn errors(&self, errors: impl IntoIterator<Item = Error>) {
        for e in errors {
            self.error(e);
        }
    }
}

impl Index<Span> for Runtime {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self.files[index]
    }
}

impl ariadne::Cache<FileId> for &Runtime {
    type Storage = ArcStr;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source<ArcStr>, impl fmt::Debug> {
        self.files.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.files.ariadne_display(*id)
    }
}

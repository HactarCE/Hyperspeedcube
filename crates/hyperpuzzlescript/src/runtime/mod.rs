use std::{collections::HashMap, fmt, ops::Index, sync::Arc};

mod ctx;
mod file_store;
mod scope;

use crate::{Error, FileId, Result, Span, Value, Warning, ast};
use arcstr::{ArcStr, Substr};
pub use ctx::EvalCtx;
pub use file_store::FileStore;
pub use scope::Scope;

/// Script runtime.
pub struct Runtime {
    /// Source file names and contents.
    pub files: FileStore,

    /// Prelude to be imported into every file.
    prelude: HashMap<String, ()>,

    builtins: Arc<Scope>,

    pub any_errors: bool,
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            files: Default::default(),
            prelude: Default::default(),
            builtins: crate::builtins::new_builtins_scope(),
            any_errors: Default::default(),
        }
    }
}

impl Runtime {
    /// Constructs a new runtime with no files.
    pub fn new() -> Self {
        Self::default()
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

    /// Parses any files that have not yet been parsed and executes any files that
    /// have not yet been executed.
    pub fn exec_all_files(&mut self) {
        for i in 0..self.files.len() {
            self.file_ret(i as FileId);
        }
    }

    /// Returns the top-level AST for a file, or `None` if it doesn't exist.
    pub fn file_ast(&mut self, file_id: FileId) -> Option<Arc<ast::Node>> {
        let file = self.files.get_mut(file_id)?;
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
                let file = self.files.get_mut(file_id)?;
                file.ast = Some(Arc::new(ast));
                file.ast.clone()
            }
        }
    }

    pub fn file_ret(&mut self, file_id: FileId) -> Option<&Result<Value, ()>> {
        let file = self.files.get_mut(file_id)?;
        match file.result {
            // extra lookup is necessary to appease borrowchecker
            Some(_) => self.files.get_mut(file_id)?.result.as_ref(),
            None => {
                let ast = self.file_ast(file_id)?;
                let scope = Scope::new_top_level(&self.builtins);
                let result = EvalCtx {
                    scope: &scope,
                    runtime: self,
                    caller_span: crate::BUILTIN_SPAN,
                }
                .eval(&ast)
                .or_else(Error::try_resolve_return_value)
                .map_err(|e| self.error(e));
                let file = self.files.get_mut(file_id)?;
                file.result = Some(result);
                file.result.as_ref()
            }
        }
    }

    pub fn substr(&self, span: Span) -> Substr {
        self.files.substr(span)
    }

    pub fn info_str(&mut self, s: impl fmt::Display) {
        println!("[INFO] {s}");
    }

    pub fn warn_str(&mut self, s: impl fmt::Display) {
        eprintln!("[WARN] {s}");
    }
    pub fn warn(&mut self, w: Warning) {
        self.warn_str(w.to_string(&*self));
    }

    pub fn error_str(&mut self, s: impl fmt::Display) {
        eprintln!("[ERROR] {s}");
    }
    pub fn error(&mut self, e: Error) {
        self.any_errors = true;
        self.error_str(e.to_string(&*self));
    }
    pub fn errors(&mut self, errors: impl IntoIterator<Item = Error>) {
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

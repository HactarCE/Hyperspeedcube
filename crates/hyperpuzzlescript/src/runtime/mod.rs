use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use arcstr::ArcStr;

mod ctx;
mod file_store;
mod scope;
mod special;

pub use ctx::EvalCtx;
pub use file_store::Modules;
pub use scope::{Builtins, ParentScope, Scope};
pub use special::SpecialVariables;

use crate::{FileId, FullDiagnostic, Map, Result, Span, Value, ValueData, ast, engines};

/// Script runtime.
pub struct Runtime {
    /// Source file names and contents.
    pub modules: Modules,
    /// Built-ins to be imported into every file.
    pub builtins: Arc<Scope>,
    /// Registered puzzle engines.
    pub puzzle_engines: HashMap<String, engines::PuzzleEngineCallback>,
    /// Registered twist system engines.
    pub twist_system_engines: HashMap<String, engines::TwistSystemEngineCallback>,

    /// Function to call on print.
    pub on_print: Box<dyn Send + Sync + FnMut(String)>,
    /// Function to call on warning or error.
    pub on_diagnostic: Box<dyn Send + Sync + FnMut(&Modules, FullDiagnostic)>,
    /// Number of warnings and errors reported since the last time this counter
    /// was reset.
    pub diagnostic_count: usize,
}

impl fmt::Debug for Runtime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runtime").finish_non_exhaustive()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            modules: Default::default(),
            builtins: Scope::new(),
            puzzle_engines: HashMap::new(),
            twist_system_engines: HashMap::new(),

            on_print: Box::new(|s| println!("[INFO] {s}")),
            on_diagnostic: Box::new(|files, e| eprintln!("{}", e.to_string(files))),
            diagnostic_count: 0,
        }
    }
}

impl Runtime {
    /// Constructs a new runtime with no files.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses any files that have not yet been parsed.
    ///
    /// Files are parsed automatically as they are needed, but it may be more
    /// efficient to call this method first.
    pub fn parse_all(&mut self) {
        for i in 0..self.modules.len() {
            self.file_ast(i as FileId);
        }
    }

    /// Parses any files that have not yet been parsed and executes any files
    /// that have not yet been executed.
    ///
    /// Files are executed automatically as they are imported, so it is not
    /// necessary to call this method if there is a specific entry point (like a
    /// main file where execution starts).
    ///
    /// Files are executed in an unspecified order.
    pub fn exec_all_files(&mut self) {
        for i in 0..self.modules.len() {
            self.load_module(i as FileId);
        }
    }

    /// Returns the top-level AST for a file, or `None` if it doesn't exist.
    pub fn file_ast(&mut self, file_id: FileId) -> Option<Arc<ast::Node>> {
        let file = self.modules.get_mut(file_id)?;
        match file.ast.clone() {
            Some(ast) => Some(ast),
            None => {
                let contents = file.contents.clone();
                let ast =
                    crate::parse::parse(file_id as FileId, &contents).unwrap_or_else(|errors| {
                        self.report_diagnostics(errors);
                        let span = Span {
                            start: 0,
                            end: contents.len() as u32,
                            context: file_id,
                        };
                        (ast::NodeContents::Error, span)
                    });
                let file = self.modules.get_mut(file_id)?;
                file.ast = Some(Arc::new(ast));
                file.ast.clone()
            }
        }
    }

    /// Executes a file if it has not yet been executed, and then returns its
    /// return value / exports.
    ///
    /// Returns `None` if the file doesn't exist. Returns `Err(())` if the file
    /// failed to load (in which case an error has already been reported).
    pub fn load_module(&mut self, file_id: FileId) -> Option<&Result<Value, ()>> {
        let file = self.modules.get(file_id)?;
        match file.result {
            // extra lookup is necessary to appease borrowchecker
            Some(_) => self.modules.get(file_id)?.result.as_ref(),
            None => {
                let result = self.load_module_uncached(file_id)?;
                let file = self.modules.get_mut(file_id)?;
                file.result = Some(result);
                file.result.as_ref()
            }
        }
    }
    fn load_module_uncached(&mut self, file_id: FileId) -> Option<Result<Value, ()>> {
        let file = self.modules.get(file_id)?;
        let submodules = file.submodules.clone();
        let mut exports: Option<Map> = None;
        for submodule_id in submodules {
            match self.load_module(submodule_id).cloned() {
                Some(Ok(submodule_return_value)) => {
                    exports.get_or_insert_default().insert(
                        self.modules.module_name(submodule_id)?,
                        submodule_return_value,
                    );
                }
                _ => return Some(Err(())), // submodule failed to load or doesn't exist
            }
        }
        let ast = self.file_ast(file_id)?;
        let scope = Scope::new_top_level(&self.builtins);
        let mut ctx = EvalCtx {
            scope: &scope,
            runtime: self,
            caller_span: crate::BUILTIN_SPAN,
            exports: &mut exports,
        };
        let result = ctx
            .eval(&ast)
            .or_else(FullDiagnostic::try_resolve_return_value)
            .map(|return_value| match exports.take() {
                Some(exports) => ValueData::Map(Arc::new(exports)).at(ast.1),
                None => return_value,
            })
            .map_err(|e| {
                if !e.is_silent() {
                    self.report_diagnostic(e);
                }
            });
        Some(result)
    }

    /// Calls [`Self::on_print`], which by default prints a message to stdout.
    pub fn print(&mut self, s: impl ToString) {
        (self.on_print)(s.to_string());
    }
    /// Calls [`Self::on_diagnostic`], which by default prints a message to
    /// stderr.
    pub fn report_diagnostic(&mut self, e: FullDiagnostic) {
        if e.is_silent() {
            return;
        }
        self.diagnostic_count += 1;
        (self.on_diagnostic)(&mut self.modules, e);
    }

    /// Calls [`Self::report_diagnostic`] on each error.
    pub fn report_diagnostics(&mut self, errors: impl IntoIterator<Item = FullDiagnostic>) {
        for e in errors {
            self.report_diagnostic(e);
        }
    }

    /// Locks the map of built-ins and executes a closure with it.
    pub fn with_builtins<R>(&mut self, f: impl FnOnce(&mut Builtins<'_>) -> R) -> R {
        f(&mut Builtins(&mut self.builtins.names.lock()))
    }

    /// Registers a puzzle engine for the runtime.
    pub fn register_puzzle_engine(&mut self, callback: engines::PuzzleEngineCallback) {
        self.puzzle_engines.insert(callback.name(), callback);
    }
    /// Registers a twist system engine for the runtime.
    pub fn register_twist_system_engine(&mut self, callback: engines::TwistSystemEngineCallback) {
        self.twist_system_engines.insert(callback.name(), callback);
    }
}

impl ariadne::Cache<FileId> for &Runtime {
    type Storage = ArcStr;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source<ArcStr>, impl fmt::Debug> {
        self.modules.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.modules.ariadne_display(*id)
    }
}

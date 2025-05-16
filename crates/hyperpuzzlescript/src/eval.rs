use std::{
    collections::{HashMap, hash_map},
    sync::Arc,
};

use arcstr::{ArcStr, Substr};
use itertools::Itertools;
use parking_lot::Mutex;

use crate::{
    ErrorMsg, Result, Span, ast,
    error::ImmutReason,
    ty::FnType,
    value::{FnOverload, FnValue, Value},
};

#[derive(Debug)]
pub struct ScopeRef {
    pub scope: Arc<Scope>,
    pub immut_reason: Option<ImmutReason>,
}
impl ScopeRef {
    pub fn new_parent(self) -> Arc<Scope> {
        Arc::new(Scope {
            names: Mutex::new(HashMap::new()),
            parent: Some(self),
        })
    }
    pub fn is_mutable(&self) -> bool {
        self.immut_reason.is_none()
    }
}

#[derive(Debug, Default)]
pub struct Scope {
    pub parent: Option<ScopeRef>,
    pub names: Mutex<HashMap<Substr, Value>>,
}
impl Scope {
    /// Constructs a new top-level scope.
    pub fn new() -> Arc<Scope> {
        Arc::new(Scope::default())
    }
    /// Constructs a new block scope.
    ///
    /// The parent scope is **mutable**.
    pub fn new_block(parent_scope: Arc<Scope>) -> Arc<Scope> {
        Self::new_with_parent(parent_scope, None)
    }
    /// Constructs a new function scope.
    ///
    /// The parent scope is **immutable**.
    pub fn new_closure(parent_scope: Arc<Scope>, fn_name: Option<Substr>) -> Arc<Scope> {
        let immut_reason = match fn_name {
            Some(name) => ImmutReason::NamedFn(name),
            None => ImmutReason::AnonymousFn,
        };
        Self::new_with_parent(parent_scope, Some(immut_reason))
    }
    fn new_with_parent(parent_scope: Arc<Scope>, immut_reason: Option<ImmutReason>) -> Arc<Scope> {
        Arc::new(Scope {
            names: Mutex::new(HashMap::new()),
            parent: Some(ScopeRef {
                scope: parent_scope,
                immut_reason,
            }),
        })
    }

    /// Returns the value of a variable.
    pub fn get(&self, name: &str) -> Value {
        self.names
            .lock()
            .get(name)
            .cloned()
            .unwrap_or_else(|| match &self.parent {
                Some(parent) => parent.scope.get(name),
                None => Value::Null,
            })
    }

    /// Sets a variable, creating a new one if it does not already exist.
    pub fn set(&self, name: Substr, value: Value) {
        match self.names.lock().entry(name.clone()) {
            hash_map::Entry::Occupied(mut e) => {
                e.insert(value);
            }
            hash_map::Entry::Vacant(e) => {
                if let Some(parent) = &self.parent {
                    if parent.is_mutable() {
                        parent.scope.set(name, value);
                        return;
                    }
                }
                e.insert(value);
            }
        }
    }

    /// Applies a function to modify the value of a variable. Returns an error
    /// if the variable is immutable.
    ///
    /// If the variable is `null`, defines a new variable in the current scope
    /// using `default` and then calls `modify`.
    pub fn modify(
        &self,
        span: Span,
        name: Substr,
        modify: impl FnOnce(&mut Value) -> Result<()>,
    ) -> Result<()> {
        match self.names.lock().entry(name.clone()) {
            hash_map::Entry::Occupied(mut e) => modify(e.get_mut()),
            hash_map::Entry::Vacant(e) => match &self.parent {
                Some(parent) => match parent.immut_reason.clone() {
                    Some(reason) => Err(ErrorMsg::Immut { name, reason }.at(span)),
                    None => parent.scope.modify(span, name, modify),
                },
                _ => modify(e.insert(Value::Null)),
            },
        }
    }

    /// Registers a function in the scope.
    pub fn register_func(&self, span: Span, name: Substr, overload: FnOverload) -> Result<()> {
        self.modify(span, name, |val| val.as_func_mut().push_overload(overload))
    }
}

pub struct Ctx {
    pub src: ArcStr,
    pub scope: Scope,
}

impl Ctx {
    pub fn eval(&mut self, node: &ast::Node) -> Result<Value> {
        let (contents, span) = node;
        match contents {
            ast::NodeContents::Assign {
                var,
                ty,
                assign_symbol,
                value,
            } => todo!(),
            ast::NodeContents::Export(_) => todo!(),
            ast::NodeContents::FnDef { name, contents } => todo!(),
            ast::NodeContents::ImportAllFrom(import_path) => todo!(),
            ast::NodeContents::ImportFrom(simple_spans, import_path) => todo!(),
            ast::NodeContents::ImportAs(import_path, simple_span) => todo!(),
            ast::NodeContents::Import(simple_span) => todo!(),
            ast::NodeContents::UseAllFrom(_) => todo!(),
            ast::NodeContents::UseFrom(simple_spans, _) => todo!(),
            ast::NodeContents::Block(items) => {
                if items.len() == 1 {
                    self.eval(&items[0])
                } else {
                    for item in items {
                        self.eval(item)?;
                    }
                    Ok(Value::Null)
                }
            }
            ast::NodeContents::IfElse {
                if_cases,
                else_case,
            } => todo!(),
            ast::NodeContents::ForLoop {
                loop_vars,
                iterator,
                body,
            } => todo!(),
            ast::NodeContents::WhileLoop { condition, body } => todo!(),
            ast::NodeContents::Continue => todo!(),
            ast::NodeContents::Break => todo!(),
            ast::NodeContents::Return(_) => todo!(),
            ast::NodeContents::Ident(simple_span) => Ok(self
                .get(&self.src[simple_span.into_range()])
                .ok_or("no such variable")?),
            ast::NodeContents::Op { op, args } => {
                let f = self.get(&self.src[op.into_range()]).expect("no operator");
                let args: Vec<Value> = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                f.unwrap_func().call(self, &args)
                //     match &self.src[op.into_range()] {
                //     "-" => Ok(Value::Num(
                //         self.eval(&args[0]).unwrap().unwrap_num()
                //             - self.eval(&args[1]).unwrap().unwrap_num(),
                //     )),
                //     "<" => Ok(Value::Bool(
                //         self.eval(&args[0]).unwrap().unwrap_num()
                //             < self.eval(&args[1]).unwrap().unwrap_num(),
                //     )),
                //     s => panic!("{s:?}"),
                // }
            }
            ast::NodeContents::FnCall { func, args } => {
                let f = self.get(func).expect("no operator");
                let args: Vec<Value> = args.iter().map(|arg| self.eval(arg)).try_collect()?;
                f.unwrap_func().call(self, &args)
            }
            ast::NodeContents::Paren(expr) => self.eval(expr),
            ast::NodeContents::Access { obj, field } => todo!(),
            ast::NodeContents::Index { obj, args } => todo!(),
            ast::NodeContents::Fn(fn_contents) => todo!(),
            ast::NodeContents::NullLiteral => Ok(Value::Null),
            ast::NodeContents::BoolLiteral(b) => Ok(Value::Bool(*b)),
            ast::NodeContents::NumberLiteral(n) => Ok(Value::Num(*n)),
            ast::NodeContents::StringLiteral(string_segments) => todo!(),
            ast::NodeContents::ListLiteral(items) => todo!(),
            ast::NodeContents::MapLiteral(items) => todo!(),
        }
    }
}

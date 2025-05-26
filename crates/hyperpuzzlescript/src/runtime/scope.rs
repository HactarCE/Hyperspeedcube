use std::collections::{HashMap, hash_map};
use std::sync::Arc;

use arcstr::Substr;
use parking_lot::Mutex;

use crate::{FnOverload, ImmutReason, Result, Span, Value};

#[derive(Debug, Clone)]
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

/// Scope containing variables, and optionally referencing a parent scope.
#[derive(Debug, Default)]
pub struct Scope {
    /// Parent scope.
    pub parent: Option<ScopeRef>,
    /// Names in this scope.
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
    /// Constructs a new top-level file scope.
    pub fn new_top_level(builtins: &Arc<Scope>) -> Arc<Scope> {
        Self::new_with_parent(Arc::clone(builtins), Some(ImmutReason::Builtin))
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
    pub fn get(&self, name: &str) -> Option<Value> {
        let value_in_this_scope = self.names.lock().get(name).cloned();
        value_in_this_scope.or_else(|| self.parent.as_ref()?.scope.get(name))
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
                        return parent.scope.set(name, value);
                    }
                }
                e.insert(value);
            }
        }
    }
    /// Sets the value of a variable and returns `Ok` containing the old value
    /// if it is already defined, or does nothing and returns `Err` containing
    /// `value` if it is not defined.
    fn set_if_defined(&self, name: Substr, value: Value) -> Result<Value, Value> {
        match self.names.lock().entry(name.clone()) {
            hash_map::Entry::Occupied(mut e) => Ok(e.insert(value)),
            hash_map::Entry::Vacant(e) => {
                if let Some(parent) = &self.parent {
                    if parent.is_mutable() {
                        parent.scope.set_if_defined(name, value)
                    } else if let Some(old_value) = parent.scope.get(&name) {
                        e.insert(value);
                        Ok(old_value)
                    } else {
                        Err(value)
                    }
                } else {
                    Err(value)
                }
            }
        }
    }

    /// Applies `f` to the value of a variable in the current scope, first
    /// assigning `default` if it is not yet defined.
    ///
    /// `f` **must not** access the variable being modified via the current
    /// scope.
    fn atomic_modify(
        &self,
        name: Substr,
        f: impl FnOnce(&mut Value) -> Result<()>,
        default: Option<Value>,
    ) -> Result<()> {
        let existing_value = self.set_if_defined(name.clone(), Value::NULL);
        match existing_value.ok().or(default) {
            Some(mut value) => {
                let result = f(&mut value);
                self.set(name, value);
                result
            }
            None => Ok(()),
        }
    }

    /// Registers a function in the scope.
    pub fn register_func(&self, span: Span, name: Substr, overload: FnOverload) -> Result<()> {
        self.atomic_modify(
            name.clone(),
            |val| val.as_func_mut(span, Some(name)).push_overload(overload),
            Some(Value::NULL),
        )
    }

    /// Registers a built-in function in the scope.
    pub fn register_builtin_functions(
        &self,
        funcs: impl IntoIterator<Item = (&'static str, FnOverload)>,
    ) -> Result<()> {
        for (name, overload) in funcs {
            self.register_func(crate::BUILTIN_SPAN, name.into(), overload)?;
        }
        Ok(())
    }
}

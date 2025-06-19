use std::sync::Arc;

use arcstr::Substr;
use parking_lot::Mutex;

use crate::{
    FnOverload, FnValue, ImmutReason, Key, Map, Result, Span, SpecialVariables, TypeOf, Value,
    ValueData,
};

/// Reference to a parent scope.
#[derive(Debug, Clone)]
pub struct ParentScope {
    /// Parent scope.
    pub scope: Arc<Scope>,
    /// Reason that the parent scope is immutable, or `None` if the parent scope
    /// is mutable.
    pub immut_reason: Option<ImmutReason>,
}
impl ParentScope {
    /// Returns whether the parent scope is mutable.
    pub fn is_mutable(&self) -> bool {
        self.immut_reason.is_none()
    }
}

/// Scope containing variables, and optionally referencing a parent scope.
#[derive(Debug, Default)]
pub struct Scope {
    /// Parent scope.
    pub parent: Option<ParentScope>,
    /// Names in this scope.
    pub names: Mutex<Map>,
    /// Special variables, which are inherited via the call graph.
    pub special: SpecialVariables,
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
        Arc::new(Self::new_with_parent(parent_scope, None))
    }
    /// Constructs a new block scope with different special variables.
    ///
    /// The parent scope is **mutable**.
    pub fn new_with_block(
        parent_scope: Arc<Scope>,
        modify_special: impl FnOnce(&mut SpecialVariables) -> Result<()>,
    ) -> Result<Arc<Scope>> {
        let mut ret = Self::new_with_parent(parent_scope, None);
        modify_special(&mut ret.special)?;
        Ok(Arc::new(ret))
    }
    /// Constructs a new function scope.
    ///
    /// The parent scope is **immutable**.
    pub fn new_closure(
        caller_scope: &Scope,
        parent_scope: Arc<Scope>,
        fn_name: Option<Substr>,
    ) -> Arc<Scope> {
        let immut_reason = match fn_name {
            Some(name) => ImmutReason::NamedFn(name),
            None => ImmutReason::AnonymousFn,
        };
        let mut ret = Self::new_with_parent(parent_scope, Some(immut_reason));
        ret.special = caller_scope.special.clone();
        Arc::new(ret)
    }
    /// Constructs a new top-level file scope.
    pub fn new_top_level(builtins: &Arc<Scope>) -> Arc<Scope> {
        Arc::new(Self::new_with_parent(
            Arc::clone(builtins),
            Some(ImmutReason::Builtin),
        ))
    }
    fn new_with_parent(parent_scope: Arc<Scope>, immut_reason: Option<ImmutReason>) -> Scope {
        let registry = parent_scope.special.clone();
        Scope {
            names: Mutex::new(Map::new()),
            parent: Some(ParentScope {
                scope: parent_scope,
                immut_reason,
            }),
            special: registry,
        }
    }

    /// Returns the value of a variable.
    pub fn get(&self, name: &str) -> Option<Value> {
        let value_in_this_scope = self.names.lock().get(name).cloned();
        value_in_this_scope.or_else(|| self.parent.as_ref()?.scope.get(name))
    }

    /// Sets a variable, creating a new one if it does not already exist.
    pub fn set(&self, name: impl Into<Key>, value: Value) {
        let name = name.into();
        match self.names.lock().entry(name.clone()) {
            indexmap::map::Entry::Occupied(mut e) => {
                e.insert(value);
            }
            indexmap::map::Entry::Vacant(e) => {
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
    fn set_if_defined(&self, name: impl Into<Key>, value: Value) -> Result<Value, Value> {
        let name = name.into();
        match self.names.lock().entry(name.clone()) {
            indexmap::map::Entry::Occupied(mut e) => Ok(e.insert(value)),
            indexmap::map::Entry::Vacant(e) => {
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

    /// Adds a value to the current scope.
    ///
    /// This is equivalent to `set` for all values except functions, for which
    /// it merges the overrides. Conflicting overrides cause an error.
    pub fn add(&self, name: impl Into<Key>, value: Value) -> Result<()> {
        if let Ok(f) = value.as_ref::<FnValue>() {
            let name = name.into();
            for o in &f.overloads {
                self.register_func(value.span, name.clone(), o.clone())?;
            }
        } else {
            self.set(name, value);
        }
        Ok(())
    }

    /// Applies `f` to the value of a variable in the current scope, first
    /// assigning `default` if it is not yet defined.
    ///
    /// `f` **must not** access the variable being modified via the current
    /// scope.
    fn atomic_modify(
        &self,
        name: impl Into<Key>,
        f: impl FnOnce(&mut Value) -> Result<()>,
        default: Option<Value>,
    ) -> Result<()> {
        let name = name.into();
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

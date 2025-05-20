use std::{
    collections::{HashMap, hash_map},
    sync::Arc,
};

use arcstr::Substr;
use lazy_static::lazy_static;
use parking_lot::Mutex;

lazy_static! {
    pub static ref EMPTY_SCOPE: Arc<Scope> = Scope::new();
    pub static ref BUILTIN_SCOPE: Arc<Scope> = crate::builtins::new_builtins_scope();
}

use crate::{FnOverload, FnValue, ImmutReason, Result, Span, Value, ValueData};

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
    /// Constructs a new top-level file scope.
    pub fn new_top_level() -> Arc<Scope> {
        Self::new_with_parent(Arc::clone(&BUILTIN_SCOPE), Some(ImmutReason::Builtin))
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
                        return parent.scope.set_if_defined(name, value);
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

    // /// Applies `modify` to the value of a variable in the current scope. If the
    // /// variable is undefined, then `create` is called **instead** and the
    // /// result is assinged to variable.
    // ///
    // /// - If the variable is from an immutable parent scope, then it is first
    // ///   copied into the outermost mutable scope and then modified.
    // /// - If the value itself is immutable, then an error is emitted.
    // ///
    // /// `modify` and `create` **must not** access the current scope.
    // #[deprecated]
    // pub fn modify(
    //     &self,
    //     name: Substr,
    //     modify: impl FnOnce(&mut Value) -> Result<()>,
    //     create: impl FnOnce() -> Result<Option<Value>>,
    // ) -> Result<()> {
    //     match self.names.lock().entry(name.clone()) {
    //         // Exists in this scope
    //         hash_map::Entry::Occupied(mut e) => modify(e.get_mut()),

    //         // Doesn't exist in this scope
    //         hash_map::Entry::Vacant(e) => {
    //             let mut is_undefined = false;
    //             match &self.parent {
    //                 Some(parent) => match parent.immut_reason.clone() {
    //                     // Parent scope is immutable
    //                     Some(_) => {
    //                         match parent.scope.get(&name) {
    //                             // Exists in the immutable parent scope
    //                             Some(mut value) => {
    //                                 // Copy to this scope and modify
    //                                 modify(&mut value)?;
    //                                 self.set(name, value);
    //                                 return Ok(());
    //                             }
    //                             // Doesn't exist
    //                             None => is_undefined = true,
    //                         }
    //                     }

    //                     // Parent scope is mutable
    //                     None => parent.scope.modify(name, modify, || {
    //                         is_undefined = true;
    //                         Ok(None)
    //                     })?,
    //                 },

    //                 // No parent scope
    //                 _ => is_undefined = true,
    //             }
    //             if is_undefined {
    //                 if let Some(new_value) = create()? {
    //                     e.insert(new_value);
    //                 }
    //             }
    //             Ok(())
    //         }
    //     }
    // }

    /// Applies `modify` to the value of a variable in the current scope. If the
    /// variable is undefined, then `create` is called **instead** and the
    /// result is assinged to variable.
    ///
    /// `modify` and `create` **must not** access the current scope.
    pub fn atomic_modify(
        &self,
        name: Substr,
        modify: impl FnOnce(&mut Value) -> Result<()>,
        create: impl FnOnce() -> Result<Option<Value>>,
    ) -> Result<()> {
        match self.set_if_defined(name.clone(), Value::NULL) {
            Ok(mut existing_value) => {
                let result = modify(&mut existing_value);
                self.set(name, existing_value);
                result
            }
            Err(_) => {
                if let Some(new_value) = create()? {
                    self.set(name, new_value);
                }
                Ok(())
            }
        }
    }

    /// Registers a function in the scope.
    pub fn register_func(&self, span: Span, name: Substr, overload: FnOverload) -> Result<()> {
        self.atomic_modify(
            name,
            |val| val.as_func_mut(span).push_overload(overload.clone()),
            || {
                let mut f = FnValue::default();
                f.push_overload(overload.clone())?;
                Ok(Some(ValueData::Fn(Arc::new(f)).at(span)))
            },
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

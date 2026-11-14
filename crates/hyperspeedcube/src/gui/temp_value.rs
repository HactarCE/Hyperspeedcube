//! Temporary values stored in egui data.
//!
//! These values are persisted frame-to-frame, but not across restarts.

use std::any::Any;
use std::ops::{Deref, DerefMut};

/// Value stored in egui memory.
///
/// This struct dereferences to `T` via [`Deref`], so it can be used similarly
/// to smart pointers like `Rc` and `Arc`.
///
/// The value is persisted frame-to-frame, but not across restarts. It is
/// automatically stored in `egui.data()` when dropped. **Do not drop this while
/// the egui context is locked (such as in a `ui.data()` closure).**
#[derive(Debug)]
pub struct EguiValue<T: 'static + Any + Clone + Send + Sync> {
    ctx: egui::Context,
    id: egui::Id,
    value: Option<T>,
}

impl<T: 'static + Any + Clone + Send + Sync> EguiValue<T> {
    /// Loads the value from egui memory, using `Default::default()` if it is
    /// missing.
    pub fn load_or_default(ctx: &egui::Context, id: egui::Id) -> Self
    where
        T: Default,
    {
        Self::load_or_else(ctx, id, Default::default)
    }

    /// Loads the value or computes it from a closure if it is missing.
    pub fn load_or_else(ctx: &egui::Context, id: egui::Id, default: impl FnOnce() -> T) -> Self {
        Self {
            ctx: ctx.clone(),
            id,
            value: Some(Self::get(ctx, id).unwrap_or_else(default)),
        }
    }

    /// Loads the value, or computes it from a fallible closure if it is
    /// missing.
    pub fn load_or_try<E>(
        ctx: &egui::Context,
        id: egui::Id,
        try_default: impl FnOnce() -> Result<T, E>,
    ) -> Result<Self, E> {
        Ok(Self {
            ctx: ctx.clone(),
            id,
            value: Some(match Self::get(ctx, id) {
                Some(v) => v,
                None => try_default()?,
            }),
        })
    }

    /// Removes the value from egui data.
    pub fn remove(mut this: Self) {
        this.value.take();
        drop(this); // update egui data
    }

    /// Returns the stored value from egui data.
    fn get(ctx: &egui::Context, id: egui::Id) -> Option<T> {
        ctx.data_mut(|d| d.get_temp::<Option<T>>(id)).flatten()
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Deref for EguiValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().expect("temp value missing")
    }
}

impl<T: 'static + Any + Clone + Send + Sync> DerefMut for EguiValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().expect("temp value missing")
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Drop for EguiValue<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.ctx
                .data_mut(|d| d.insert_temp::<Option<T>>(self.id, Some(value)));
        }
    }
}

/// Optional value stored in egui memory.
///
/// This struct dereferences to `Option<T>` via [`Deref`], so it can be used
/// similarly to smart pointers like `Rc` and `Arc`.
///
/// The value is persisted frame-to-frame, but not across restarts. It is
/// automatically stored in `egui.data()` when dropped. **Do not drop this while
/// the egui context is locked (such as in a `ui.data()` closure).**
#[derive(Debug)]
pub struct EguiOptionValue<T: 'static + Any + Clone + Send + Sync> {
    ctx: egui::Context,
    id: egui::Id,
    value: Option<T>,
}

impl<T: 'static + Any + Clone + Send + Sync> EguiOptionValue<T> {
    /// Loads the value from egui memory.
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Self {
        Self {
            ctx: ctx.clone(),
            id,
            value: ctx.data_mut(|d| d.get_temp::<Option<T>>(id)).flatten(),
        }
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Deref for EguiOptionValue<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: 'static + Any + Clone + Send + Sync> DerefMut for EguiOptionValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: 'static + Any + Clone + Send + Sync> Drop for EguiOptionValue<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.ctx
                .data_mut(|d| d.insert_temp::<Option<T>>(self.id, Some(value)));
        } else {
            self.ctx.data_mut(|d| d.remove::<Option<T>>(self.id));
        }
    }
}

/// Boolean flag stored in egui memory, which defaults to `false`. Similar to
/// [`EguiValue`].
///
/// This struct dereferences to `bool` via [`Deref`], so it can be used
/// similarly to smart pointers like `Rc` and `Arc`.
///
/// The value is persisted frame-to-frame, but not across restarts. It is
/// automatically stored in `egui.memory()` when dropped. **Do not drop this
/// while the egui context is locked (such as in a `ui.memory()` closure).**
#[derive(Debug)]
pub struct EguiFlag {
    ctx: egui::Context,
    id: egui::Id,
    value: bool,
}

impl EguiFlag {
    /// Loads the flag from egui memory.
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Self {
        Self {
            ctx: ctx.clone(),
            id,
            value: ctx.data_mut(|d| d.get_temp::<()>(id)).is_some(),
        }
    }
}

impl Deref for EguiFlag {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for EguiFlag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl Drop for EguiFlag {
    fn drop(&mut self) {
        if self.value {
            self.ctx.data_mut(|d| d.insert_temp::<()>(self.id, ()));
        } else {
            self.ctx.data_mut(|d| d.remove::<()>(self.id));
        }
    }
}

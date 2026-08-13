//! Catalog of puzzles and related objects, along with functionality for loading
//! them.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet, HashMap, hash_map};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eyre::{Context, OptionExt, Result, bail, eyre};
use itertools::Itertools;
use parking_lot::{Condvar, MappedMutexGuard, Mutex, MutexGuard};
use serde::Serialize;

mod builder;
mod entry;
mod error;
mod generator;
mod hook;
mod list;
mod menu;
mod notify;
mod object;
mod params;
mod request;
mod subcatalog;

pub use builder::CatalogBuilder;
use entry::*;
pub use error::*;
pub use generator::*;
pub use hook::CatalogHook;
pub use hyperspeedcube_cli_types::catalog_id::*;
pub use list::*;
pub use menu::*;
pub use notify::{NotifyWhenDropped, Waiter};
pub use object::*;
pub use params::*;
pub use request::*;
pub use subcatalog::*;

use crate::{ColorSystem, Logger, Puzzle, TagSet, TwistSystem, Version};

/// Error indicating that the building the object was canceled.
#[derive(thiserror::Error, Debug, Default, Copy, Clone, PartialEq, Eq)]
#[error("canceled")]
pub struct Cancel;

/// Catalog of shapes, puzzles, twist systems, etc.
///
/// This type is a simple wrapper around `Arc<`[`CatalogData`]`>` and thus cheap
/// to clone.
///
/// To construct a catalog, use [`CatalogBuilder::new()`] and
/// [`CatalogBuilder::build()`].
#[derive(Debug, Default, Clone)]
pub struct Catalog(Arc<CatalogData>);

impl Deref for Catalog {
    type Target = CatalogData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Catalog {
    /// Requests an object to be built if it has not been built already,
    /// returning a [`Request`] that can be used to check status and recieve
    /// results.
    ///
    /// It may take time for the object to build. If you want to block the
    /// current thread until the object is built, see
    /// [`Self::build_blocking()`].
    ///
    /// **Note: Do not call this method from within an object generator.** Use
    /// [`BuildCtx::build_blocking()`] or [`BuildCtx::build_str_blocking()`]
    /// instead.
    ///
    /// **Note: The returned object might not have the ID as the request.** When
    /// this happens, it is called an "ID redirect."
    pub fn build<T: CatalogObject>(&self, id: &CatalogId) -> Request<T> {
        let parent_stack = &[];
        let (request, call_if_new) = self.new_request(id, parent_stack);
        if let Some(f) = call_if_new {
            std::thread::spawn(f);
        }
        request
    }

    /// Builds an object and blocks the current thread until it is complete.
    ///
    /// The result is cached.
    ///
    /// This method is equivalent to `catalog.build().get_blocking()` except
    /// that it avoids spawning a new thread.
    ///
    /// **Note: Do not call this method from within an object generator.** Use
    /// [`BuildCtx::build_blocking()`] or [`BuildCtx::build_str_blocking()`]
    /// instead.
    ///
    /// **Note: The returned object might not have the ID as the request.** When
    /// this happens, it is called an "ID redirect."
    pub fn build_blocking<T: CatalogObject>(&self, id: &CatalogId) -> CatalogResult<T> {
        let parent_stack = &[];
        let (request, call_if_new) = self.new_request(id, parent_stack);
        if let Some(f) = call_if_new {
            f();
        }
        request.get_blocking()
    }

    fn new_request<T: CatalogObject>(
        &self,
        id: &CatalogId,
        parent_ids: &[String],
    ) -> (Request<T>, Option<Box<dyn Send + FnOnce()>>) {
        let type_name = T::catalog_type_name();

        let Some(subcatalog) = self.get_subcatalog::<T>() else {
            let e = eyre!("{type_name} catalog is empty");
            return (Request::new_error(id, e), None);
        };

        // Make sure the generator exists before creating a cache entry
        let generator = match subcatalog.try_get_generator(&id.base) {
            Ok(g) => Arc::clone(&g),
            Err(e) => return (Request::new_error(id, e), None),
        };

        // Validate parameters to avoid cache entries for obvious errors
        if let Err(e) = generator.validate(id) {
            return (Request::new_error(id, e), None);
        }

        let (request, is_new) = subcatalog.request_cache_entry(self, id.clone());

        let call_if_new = if is_new {
            let RequestInner::Requested {
                generic_request, ..
            } = &request.inner
            else {
                panic!("invalid request contents for new cache entry");
            };
            let mut parent_ids = parent_ids.to_vec();
            parent_ids.push(id.to_string());
            let build_ctx = BuildCtx(Arc::new(BuildCtxInner {
                catalog: self.clone(),
                id: id.clone(),
                request_state: Arc::clone(&generic_request.data.state),
                canceled: Arc::clone(&generic_request.data.canceled),
                parent_ids,
            }));
            let id = id.clone();
            Some(Box::new(move || {
                // Generate
                let mut result = match generator.canonicalize(build_ctx.id()) {
                    Some(canonicalized_id) => build_ctx.build_blocking(&canonicalized_id), // redirect
                    None => {
                        build_ctx.push_task(format!(
                            "generating {} `{}`",
                            T::catalog_type_name(),
                            build_ctx.id(),
                        ));
                        let result = (generator.generate)(build_ctx.clone());
                        build_ctx.pop_task();
                        result
                    }
                }
                .map_err(|e| {
                    Arc::new(CatalogError {
                        type_name,
                        id,
                        task_stack: build_ctx.0.request_state.lock().task_stack.clone(),
                        cause: e.into(),
                    })
                });
                build_ctx.pop_task();

                // Run hooks
                if let Ok(obj) = &mut result
                    && let Some(subcat) = build_ctx.0.catalog.get_subcatalog()
                {
                    build_ctx.push_task("executing matching hooks".to_string());
                    for hook in &subcat.hooks {
                        if let Some(wildcard_values) = hook.id_pattern.match_wildcards(obj.id()) {
                            build_ctx.push_task(format!(
                                "executing hook matching `{}`",
                                hook.id_pattern
                            ));
                            if let Err(e) = (hook.callback)(obj, wildcard_values) {
                                (build_ctx.warn_fn())(
                                    e.wrap_err(format!("error in hook `{}`", hook.id_pattern)),
                                )
                            }
                            build_ctx.pop_task();
                        }
                    }
                    build_ctx.pop_task();
                }

                build_ctx.store_result(result);
            }) as Box<dyn 'static + Send + FnOnce()>)
        } else {
            None
        };

        (request, call_if_new)
    }
}

/// Data store for [`Catalog`].
///
/// Prefer interacting with [`Catalog`] directly.
#[derive(Debug, Default)]
pub struct CatalogData {
    /// Subcatalog for each type of [`CatalogObject`].
    pub subcatalogs: HashMap<TypeId, Box<dyn Send + Sync + Any>>,

    /// Puzzle list to display in the UI.
    pub puzzle_list: Vec<Arc<PuzzleListEntry>>,
    /// Menus, indexed by string ID.
    pub menus: HashMap<&'static str, Menu>,

    /// Alphabetized list of all puzzle definition authors.
    pub authors: BTreeSet<String>,

    /// Logger.
    pub logger: Logger,
}

impl CatalogData {
    /// Returns the subcatalog for `T`.
    pub fn get_subcatalog<T: CatalogObject>(&self) -> Option<&SubCatalog<T>> {
        self.subcatalogs
            .get(&TypeId::of::<T>())
            .map(|any| any.downcast_ref().expect("error downcasting subcatalog"))
    }

    /// Returns a generator by its ID, if it exists.
    pub fn get_generator<T: CatalogObject>(&self, id: &str) -> Option<&Arc<Generator<T>>> {
        self.get_subcatalog()?.generators.get(id)
    }

    fn get_subcatalog_mut<T: CatalogObject>(&mut self) -> &mut SubCatalog<T> {
        self.subcatalogs
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SubCatalog::<T>::default()))
            .downcast_mut()
            .expect("error downcasting subcatalog")
    }
}

#[cfg(test)]
mod tests;

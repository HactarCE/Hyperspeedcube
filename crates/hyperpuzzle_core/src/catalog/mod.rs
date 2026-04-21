//! Catalog of puzzles and related objects, along with functionality for loading
//! them.

use std::any::TypeId;
use std::collections::{HashMap, HashSet, hash_map};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use eyre::{OptionExt, Result, bail, ensure, eyre};
use itertools::Itertools;
use parking_lot::{Condvar, MappedMutexGuard, Mutex, MutexGuard};
use serde::Serialize;

mod builder;
mod entry;
mod generator;
mod menu;
mod metadata;
mod object;
mod params;
mod subcatalog;

pub use builder::CatalogBuilder;
pub use entry::*;
pub use generator::*;
pub use hyperspeedcube_cli_types::catalog_id::*;
pub use menu::*;
pub use metadata::*;
pub use object::*;
pub use params::*;
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
        &*self.0
    }
}

impl Catalog {
    /// Requests an object to be built if it has not been built already, and
    /// then immediately returns the cache entry for the object.
    ///
    /// It may take time for the object to build. If you want to block the
    /// current thread until the object is built, see
    /// [`Self::build_blocking()`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rubiks_cube = catalog.build::<Puzzle>("ft_cube(3)".parse().unwrap());
    /// println!("Requested Rubik's cube ...");
    /// loop {
    ///     let cache_entry_guard = rubiks_cube.lock();
    ///     match &*cache_entry_guard {
    ///         CacheEntry::NotStarted => {
    ///             std::thread::sleep(std::time::Duration::from_secs(1));
    ///             continue;
    ///         }
    ///         CacheEntry::Building { notify, .. } => {
    ///             let waiter = notify.waiter();
    ///             drop(cache_entry_guard);
    ///             waiter.wait();
    ///             continue;
    ///         }
    ///         CacheEntry::Ok(_) => {
    ///             println!("Success!");
    ///             break;
    ///         }
    ///         CacheEntry::Err(e) => {
    ///             println!("Error: {e}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    pub fn build<T: CatalogObject>(&self, id: &CatalogId) -> Arc<Mutex<CacheEntry<T>>> {
        let subcatalog = T::get_subcatalog(self);
        let mut cache_guard = subcatalog.cache.lock();
        match cache_guard.entry(id.to_string()) {
            hash_map::Entry::Occupied(e) => Arc::clone(e.get()),
            hash_map::Entry::Vacant(e) => {
                let this = self.clone();
                let cache_entry =
                    Arc::clone(e.insert(Arc::new(Mutex::new(CacheEntry::NotStarted))));
                drop(cache_guard);
                let id = id.clone();
                std::thread::spawn(move || {
                    if let Err(e) = this.build_blocking::<T>(&id) {
                        log::error!("Error building {id:?}: {e}");
                    }
                });
                cache_entry
            }
        }
    }

    /// Builds an object and blocks the current thread until it is complete.
    ///
    /// The result is cached.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rubiks_cube_result = catalog.build_blocking::<Puzzle>("ft_cube(3)".parse().unwrap());
    /// match rubiks_cube_result {
    ///     Ok(_puzzle) => println!("Success!"),
    ///     Err(e) => println!("Error: {e}"),
    /// }
    /// ```
    pub fn build_blocking<T: CatalogObject>(
        &self,
        id: &CatalogId,
    ) -> Result<Arc<T>, Arc<eyre::Report>> {
        let subcatalog = T::get_subcatalog(self);

        let type_str = T::CATALOG_TYPE_NAME;
        let mut id = id.clone();
        let mut redirect_sequence = vec![];

        loop {
            log::trace!("Requesting {type_str} {id:?}");
            if !redirect_sequence.is_empty() {
                log::trace!("(redirected from {redirect_sequence:?})");
            }

            redirect_sequence.push(id.clone());
            if redirect_sequence.len() > crate::MAX_ID_REDIRECTS {
                let msg = eyre!("too many ID redirects: {redirect_sequence:?}");
                self.logger.error(&msg);
                return Err(Arc::new(msg));
            }

            let generator = subcatalog.generators.get(&*id.base).ok_or_else(|| {
                eyre!(
                    "no {ty} or {ty} generator with ID {id:?}",
                    ty = T::CATALOG_TYPE_NAME,
                    id = id.base,
                )
            })?;

            let cache_entry = subcatalog.cache_entry(&id);
            let mut cache_entry_guard = cache_entry.lock();

            if let CacheEntry::NotStarted = &*cache_entry_guard {
                log::trace!("{type_str} {id:?} not yet started");
                // Mark that this object is being built.
                let progress = Arc::new(Mutex::new(Progress::default()));
                *cache_entry_guard = CacheEntry::Building {
                    progress: Arc::clone(&progress),
                    notify: NotifyWhenDropped::new(),
                };
                // Unlock the mutex before expensive object generation.
                log::trace!("Building {type_str} {id:?}");
                let cache_entry_value = MutexGuard::unlocked(&mut cache_entry_guard, || {
                    let build_ctx = BuildCtx::new(self, &progress);
                    CacheEntry::from((generator.generate)(build_ctx, id.args.clone()))
                });
                // Handle cancellation.
                if let CacheEntry::Err(e) = &cache_entry_value
                    && let Some(&Cancel) = e.downcast_ref()
                {
                    subcatalog.remove_cache_entry(&id);
                    return Err(Arc::clone(e));
                }
                // Store the result.
                log::trace!("Storing {type_str} {id:?}");
                *cache_entry_guard = CacheEntry::from(cache_entry_value);
            } else if let CacheEntry::Building { notify, .. } = &mut *cache_entry_guard {
                // If another thread is building the object, then wait for that.
                log::trace!("Waiting for another thread to build {type_str} {id:?}");
                let waiter = notify.waiter();
                MutexGuard::unlocked(&mut cache_entry_guard, || {
                    waiter.wait();
                });
                log::trace!("Done waiting on {id:?}");
            }

            match &*cache_entry_guard {
                // The object was requested but has not started being built.
                CacheEntry::NotStarted => {
                    return Err(Arc::new(eyre!(
                        "internal error: {type_str} {id:?} did not start building"
                    )));
                }

                // The object has already been built.
                CacheEntry::Ok(Redirectable::Redirect(new_id)) => {
                    id = new_id.parse().map_err(|e| Arc::new(eyre!("{e}")))?;
                }
                CacheEntry::Ok(Redirectable::Direct(output)) => return Ok(Arc::clone(output)),
                CacheEntry::Err(e) => return Err(Arc::clone(&e)), // This is why our error needs to be wrapped in `Arc`.

                // The object has already been built or is being built.
                CacheEntry::Building { .. } => {
                    return Err(Arc::new(eyre!("unexpected Building entry".to_owned())));
                }
            }
        }
    }

    /// Fetches the metadata for a puzzle and blocks the current thread until it
    /// is complete.
    ///
    /// This is typically fast, but is not guaranteed to be.
    ///
    /// The result is _not_ cached.
    pub fn get_puzzle_metadata_blocking(
        &self,
        id: &CatalogId,
    ) -> Result<Arc<CatalogMetadata>, eyre::Report> {
        let subcatalog = &self.puzzles;

        let type_str = Puzzle::CATALOG_TYPE_NAME;
        let mut id = id.clone();
        let mut redirect_sequence = vec![];

        loop {
            log::trace!("Requesting metadata for {type_str} {id:?}");
            if !redirect_sequence.is_empty() {
                log::trace!("(redirected from {redirect_sequence:?})");
            }

            redirect_sequence.push(id.clone());
            if redirect_sequence.len() > crate::MAX_ID_REDIRECTS {
                let msg = eyre!("too many ID redirects: {redirect_sequence:?}");
                self.logger.error(&msg);
                return Err(msg);
            }

            let generator = subcatalog.generators.get(&*id.base).ok_or_else(|| {
                eyre!(
                    "no {ty} or {ty} generator with ID {id:?}",
                    ty = Puzzle::CATALOG_TYPE_NAME,
                    id = id.base,
                )
            })?;

            let progress = Arc::new(Mutex::new(Progress::default()));
            let build_ctx = BuildCtx::new(self, &progress);
            match (generator.generate_meta)(build_ctx, id.args) {
                Ok(Redirectable::Direct(output)) => return Ok(output),
                Ok(Redirectable::Redirect(new_id)) => {
                    id = new_id.parse().map_err(|e| eyre!("{e}"))?;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Data store for [`Catalog`].
///
/// Prefer interacting with [`Catalog`] directly.
#[derive(Debug, Default)]
pub struct CatalogData {
    /// Puzzles.
    pub puzzles: SubCatalog<Puzzle>,
    /// Color systems.
    pub color_systems: SubCatalog<ColorSystem>,
    /// Twist systems.
    pub twist_systems: SubCatalog<TwistSystem>,

    /// Puzzle list.
    pub puzzle_list: Vec<Arc<CatalogMetadata>>,
    /// Menus, indexed by type ID.
    pub menus: HashMap<TypeId, Menu>,

    /// Alphabetized list of all puzzle definition authors.
    pub authors: Vec<String>,

    /// Logger.
    pub logger: Logger,
}

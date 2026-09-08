//! [`Request`] and related types.
//!
//! There are five types defined in this module, each containing the next:
//!
//! - [`Request`] (appears in the public API)
//! - [`RequestInner`] (enum to handle the case of an already-completed request)
//! - [``]
//!
//! If you are trying to get your bearings, read the documentation for each of
//! those types in order.

use super::*;

/// Request to build a catalog object.
///
/// When the last request to build an object is dropped, the task is canceled.
///
/// See [`Catalog::build()`] for more info.
#[derive(Debug, Clone)]
pub struct Request<T: CatalogObject> {
    pub(super) requested_id: CatalogId,
    pub(super) inner: RequestInner<T>,
}

impl<T: CatalogObject> Request<T> {
    /// Constructs an already-completed request that results in an error value.
    pub(super) fn new_error(requested_id: &CatalogId, e: eyre::Report) -> Self {
        Self {
            requested_id: requested_id.clone(),
            inner: RequestInner::Precomputed(Err(Arc::new(CatalogError {
                type_name: T::catalog_type_name(),
                id: requested_id.clone(),
                task_stack: vec![],
                cause: e.into(),
            }))),
        }
    }

    /// Returns the requested ID.
    ///
    /// This may be different from the ID of the object once it is built, such
    /// as in the case of an ID redirect.
    pub fn id(&self) -> &CatalogId {
        &self.requested_id
    }

    /// Returns the result if the task has completed ([`Ok`]), or a list of
    /// tasks from most general to most specific if it is in progress ([`Err`]).
    ///
    /// **Note: The returned object might not have the same ID as the request.**
    /// When this happens, it is called an "ID redirect."
    pub fn get(&self) -> Result<CatalogResult<T>, Vec<String>> {
        match &self.inner {
            RequestInner::Precomputed(result) => Ok(result.clone()),
            RequestInner::Requested {
                generic_request, ..
            } => {
                if generic_request.data.is_done() {
                    Ok(self.get_blocking())
                } else {
                    // race condition here is ok.
                    Err(generic_request.data.state.lock().flat_task_stack())
                }
            }
        }
    }

    /// Blocks until the task has completed and then retunrs the result.
    ///
    /// **Note: The returned object might not have the ID as the request.** When
    /// this happens, it is called an "ID redirect."
    pub fn get_blocking(&self) -> CatalogResult<T> {
        match &self.inner {
            RequestInner::Precomputed(result) => result.clone(),
            RequestInner::Requested {
                catalog,
                generic_request,
            } => {
                generic_request.data.waiter.clone().wait();
                // IIFE to mimic try_block
                (|| {
                    catalog
                        .get_subcatalog::<T>()
                        .ok_or_eyre("missing subcatalog")?
                        .cache
                        .lock()
                        .get(&self.requested_id.to_string())
                        .ok_or_eyre("missing cache entry")?
                        .as_done()
                        .ok_or_eyre("object is not yet built")
                        .cloned()
                })()
                .unwrap_or_else(|e| {
                    Err(Arc::new(CatalogError {
                        type_name: T::catalog_type_name(),
                        id: self.requested_id.clone(),
                        task_stack: vec![], // error occurred outside the generator
                        cause: e.into(),
                    }))
                })
            }
        }
    }
}

/// Enum to handle the case where the result of a request is already known at
/// the time the request is made.
///
/// This is contained within [`Request`].
#[derive(Debug, Clone)]
pub(super) enum RequestInner<T> {
    /// The result of the request is already known at the time the request is
    /// made.
    Precomputed(CatalogResult<T>),
    /// The request requires computation.
    Requested {
        catalog: Catalog,
        generic_request: GenericRequest,
    },
}

/// Wrapper around [`GenericRequestData`] that cancels a task when it is
/// dropped, if there are no other [`GenericRequest`]s alive. See
/// [`GenericRequestData`] for more info.
///
/// This is contained within [`RequestInner`].
#[derive(Debug, Clone)]
pub(super) struct GenericRequest {
    pub(super) data: Arc<GenericRequestData>,
}

impl Drop for GenericRequest {
    fn drop(&mut self) {
        (self.data.call_when_generic_request_is_dropped)();
    }
}

/// Data for a request, which is always contained in an [`Arc`]`.
///
/// For a given `Arc<GenericRequestData>`, There is at most one [`CacheEntry`]
/// that holds a reference to it. All other references are held by
/// [`GenericRequest`]s. When the last [`GenericRequest`] is dropped, the cache
/// entry is removed and then the `subrequest` field is dropped, which may
/// recursively cause another cache entry to be removed, etc.
pub(super) struct GenericRequestData {
    /// Mutable state for the request.
    pub(super) state: Arc<Mutex<GenericRequestState>>,
    /// Waiter, which can be used to block a thread until the request completes.
    pub(super) waiter: Waiter,
    /// Whether the task has been canceled.
    ///
    /// Semantically this could be in `state`, but it's probably slightly faster
    /// to check it without having to lock a mutex.
    pub(super) canceled: Arc<AtomicBool>,

    pub(super) call_when_generic_request_is_dropped: Box<dyn Send + Sync + Fn()>,
}

impl fmt::Debug for GenericRequestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestInner")
            .field("state", &self.state)
            .field("waiter", &self.waiter)
            .field("canceled", &self.canceled)
            .finish_non_exhaustive()
    }
}

impl GenericRequestData {
    pub(super) fn new<T: CatalogObject>(
        catalog: Catalog,
        id: CatalogId,
        waiter: Waiter,
    ) -> Arc<Self> {
        let state = Arc::new(Mutex::new(GenericRequestState::default()));
        let canceled = Arc::new(AtomicBool::new(false));
        Arc::new_cyclic(|this| {
            let this = this.clone();
            GenericRequestData {
                state: Arc::clone(&state),
                waiter,
                canceled: Arc::clone(&canceled),

                call_when_generic_request_is_dropped: Box::new(move || {
                    // Cancel the request if this is the last request (not
                    // including the `Arc<RequestData>` inside the subcatalog).

                    // 1 for the Arc being dropped + 1 for the cache entry
                    if this.strong_count() > 2 {
                        return; // early exit (optimization)
                    }

                    // Remove cache entry and drop subrequest if there is one
                    if let Some(subcatalog) = catalog.get_subcatalog::<T>()
                        && let mut cache_guard = subcatalog.cache.lock()
                        && let hash_map::Entry::Occupied(hash_map_entry) =
                            cache_guard.entry(id.to_string())
                        && let CacheEntry::Building { request_data, .. } = hash_map_entry.get()
                        && this.ptr_eq(&Arc::downgrade(request_data))
                        // 1 for the Arc being dropped + 1 for `self_request`
                        && this.strong_count() <= 2
                    {
                        hash_map_entry.remove();
                        let mut data_guard = state.lock();
                        let subrequest = data_guard.subrequest.take();
                        canceled.store(true, Ordering::Relaxed);
                        drop(data_guard);
                        drop(cache_guard);
                        // Drop the subrequest only after everything has been unlocked again
                        // because it may recurse
                        drop(subrequest);
                    }
                }),
            }
        })
    }

    fn is_done(&self) -> bool {
        self.waiter.is_done()
    }
}

/// Mutable state for a request.
///
/// This is contained within an `Arc<Mutex<T>>` inside [`GenericRequestData`].
#[derive(Debug, Default)]
pub(super) struct GenericRequestState {
    /// Stack of tasks, each described by a human-friendly one-liner.
    pub(super) task_stack: Vec<String>,
    /// Request for an object that is required as part of building this one.
    ///
    /// When the last [`Request`] for the parent object is dropped, then this
    /// field is also dropped. Note that when this happens, one reference to the
    /// subrequest still remains in the subrequest's [`CacheEntry`].
    pub(super) subrequest: Option<GenericRequest>,
}

impl GenericRequestState {
    fn flat_task_stack(&self) -> Vec<String> {
        let mut flat_task_stack = self.task_stack.clone();
        let mut optional_subrequest = self.subrequest.clone();
        while let Some(subrequest) = &optional_subrequest {
            let subrequest_data_guard = subrequest.data.state.lock();
            flat_task_stack.extend_from_slice(&subrequest_data_guard.task_stack);
            let optional_subsubrequest = subrequest_data_guard.subrequest.clone();
            drop(subrequest_data_guard);
            optional_subrequest = optional_subsubrequest;
        }
        flat_task_stack
    }
}

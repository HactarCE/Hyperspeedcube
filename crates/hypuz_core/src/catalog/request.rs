//! [`Request`] and related types.
//!
//! There are five types defined in this module, each containing the next:
//!
//! - [`Request`] — appears in the public API
//! - [`RequestInner`] — enum to handle the case of an already-completed request
//! - [`GenericRequest`] — struct containing an `Arc` to allow cheap cloning
//! - [`GenericRequestData`] — allows waiting and tracks cancelation
//! - [`GenericRequestState`] — shared mutable state (subrequests, diagnostics)
//!
//! If you are trying to get your bearings, read the documentation for each of
//! those types in order.

use super::*;

/// Status for a [`CatalogRequest`].
pub enum CatalogRequestStatus<T> {
    /// The request has been completed.
    Done(CatalogResult<T>),
    /// The request is in progress.
    ///
    /// The contained value is a list of in-progress tasks arranged from most
    /// general to most specific, like a stack trace.
    InProgress(Vec<String>),
}

impl<T> CatalogRequestStatus<T> {
    /// Returns the contents of the `Done` variant, if it done.
    pub fn as_done(&self) -> Option<&CatalogResult<T>> {
        match self {
            CatalogRequestStatus::Done(result) => Some(result),
            CatalogRequestStatus::InProgress(_) => None,
        }
    }

    /// Returns the contents of the `Done` variant, if it done.
    pub fn into_done(self) -> Option<CatalogResult<T>> {
        match self {
            CatalogRequestStatus::Done(result) => Some(result),
            CatalogRequestStatus::InProgress(_) => None,
        }
    }
}

/// Request to build a catalog object.
///
/// When the last request to build an object is dropped, the task is canceled.
///
/// See [`Catalog::build()`] for more info.
#[derive(Debug)]
pub struct Request<T: CatalogObject> {
    pub(super) requested_id: CatalogId,
    pub(super) inner: RequestInner<T>,
}

impl<T: CatalogObject> Clone for Request<T> {
    fn clone(&self) -> Self {
        Self {
            requested_id: self.requested_id.clone(),
            inner: self.inner.clone(),
        }
    }
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

    /// Returns whether the task is done.
    pub fn is_done(&self) -> bool {
        match &self.inner {
            RequestInner::Precomputed(_) => true,
            RequestInner::Requested {
                generic_request, ..
            } => generic_request.data.is_done(),
        }
    }

    /// Returns the result if the task has completed, or a list of tasks if it
    /// is in progress.
    ///
    /// **Note: The returned object might _not_ have the same ID as the
    /// request.** When this happens, it is called an "ID redirect."
    pub fn get(&self) -> CatalogRequestStatus<T> {
        match &self.inner {
            RequestInner::Precomputed(result) => CatalogRequestStatus::Done(result.clone()),
            RequestInner::Requested {
                generic_request, ..
            } => {
                if generic_request.data.is_done() {
                    CatalogRequestStatus::Done(self.get_blocking())
                } else {
                    // race condition between `.is_done()` and `.lock()` is ok.
                    // caller just gets slightly stale state
                    CatalogRequestStatus::InProgress(
                        generic_request.data.state.lock().flat_task_stack(),
                    )
                }
            }
        }
    }

    /// Blocks until the task has completed and then returns the result.
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

    /// Blocks until the task has been completed or canceled and then returns
    /// the result, or `None` if it was canceled.
    ///
    /// Holding a [`Request`] or calling [`Self::get_blocking()`] prevents it
    /// from being canceled. In contrast, this method does _not_ keep prevent
    /// the request from being canceled while it is blocking.
    pub fn get_blocking_weak(self) -> Option<CatalogResult<T>> {
        match self.inner {
            RequestInner::Precomputed(result) => return Some(result),
            RequestInner::Requested {
                catalog,
                generic_request,
            } => {
                let id = self.requested_id;
                let waiter = generic_request.data.waiter.clone();
                drop(generic_request); // allow cancelation
                waiter.wait();
                catalog.try_get(&id)
            }
        }
    }
}

/// Enum to handle the case where the result of a request is already known at
/// the time the request is made.
///
/// This is contained within [`Request`].
#[derive(Debug)]
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

impl<T> Clone for RequestInner<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Precomputed(result) => Self::Precomputed(result.clone()),
            Self::Requested {
                catalog,
                generic_request,
            } => Self::Requested {
                catalog: catalog.clone(),
                generic_request: generic_request.clone(),
            },
        }
    }
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

/// Data for a request, which is always contained in an [`Arc`].
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
    /// Cancel flag for the request.
    ///
    /// Semantically this could be in `state`, but it's probably slightly faster
    /// to check it without having to lock a mutex.
    pub(super) cancel_flag: CancelFlag,

    pub(super) call_when_generic_request_is_dropped: Box<dyn Send + Sync + Fn()>,
}

impl fmt::Debug for GenericRequestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestInner")
            .field("state", &self.state)
            .field("waiter", &self.waiter)
            .field("cancel_flag", &self.cancel_flag)
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
        let cancel_flag = CancelFlag::new();
        Arc::new_cyclic(|this| {
            let this = this.clone();
            GenericRequestData {
                state: Arc::clone(&state),
                waiter,
                cancel_flag: cancel_flag.clone(),

                call_when_generic_request_is_dropped: Box::new(move || {
                    // Cancel the request if this is the last request (not
                    // including the `Arc<RequestData>` inside the subcatalog).

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
                        cancel_flag.cancel();
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

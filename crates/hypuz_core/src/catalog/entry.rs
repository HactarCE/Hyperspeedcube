use super::*;

/// Entry in the catalog.
#[derive(Debug)]
pub(super) enum CacheEntry<T> {
    /// A worker thread is building the object or is waiting to build the
    /// object.
    Building {
        /// Shared request data.
        ///
        /// This cannot be a [`GenericRequest`] because that has a `Drop` impl.
        request_data: Arc<GenericRequestData>,

        /// Notifier for [`Waiter`]s. This field is never accessed except
        /// implicitly by its [`Drop`] impl.
        _notify: NotifyWhenDropped,
    },
    /// Object has been built.
    Done(CatalogResult<T>),
}

impl<T: CatalogObject> CacheEntry<T> {
    pub(super) fn as_done(&self) -> Option<&CatalogResult<T>> {
        match self {
            CacheEntry::Building { .. } => None,
            CacheEntry::Done(result) => Some(result),
        }
    }

    pub(super) fn to_request_inner(&self, catalog: &Catalog) -> RequestInner<T> {
        match self {
            CacheEntry::Building { request_data, .. } => RequestInner::Requested {
                catalog: catalog.clone(),
                generic_request: GenericRequest {
                    data: Arc::clone(request_data),
                },
            },
            CacheEntry::Done(result) => RequestInner::Precomputed(result.clone()),
        }
    }
}

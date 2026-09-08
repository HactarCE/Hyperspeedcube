use super::*;

/// Entry in the catalog.
#[derive(Debug)]
pub(super) enum CacheEntry<T> {
    /// A worker thread is building the object or is waiting to build the
    /// object.
    Building {
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
}

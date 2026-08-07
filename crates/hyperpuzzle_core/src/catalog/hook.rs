use super::*;

pub struct CatalogHook<T> {
    /// ID of the color scheme to modify.
    ///
    /// This ID may contain [`CatalogIdValue::Wildcard`], which match any ID
    /// value. The values filling in these wildcards are passed as parameters to
    /// the callback.
    pub id_pattern: CatalogId,

    /// Priority of the hook. Low-priority hooks are called **first** and
    /// high-priority hooks are called **last**.
    pub priority: i64,

    /// Callback to run to modify a color system.
    pub callback: Box<dyn Send + Sync + Fn(&mut Arc<T>, Vec<CatalogIdValue>) -> eyre::Result<()>>,
}

impl<T> fmt::Debug for CatalogHook<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogHook")
            .field("id_pattern", &self.id_pattern)
            .field("priority", &self.priority)
            .finish_non_exhaustive()
    }
}

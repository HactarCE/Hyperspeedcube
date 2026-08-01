use super::*;

/// Wrapper around [`GeneratorError`] that provides additional information about
/// the object being built and the task during which the error occurred.
#[derive(Debug)]
pub struct CatalogError {
    /// Type of object being built.
    pub type_name: &'static str,
    /// ID of the object.
    pub id: CatalogId,
    /// List of tasks in progress when the error occurred, from most general to
    /// most specific.
    ///
    /// This is approximately equivalent to a call stack.
    pub task_stack: Vec<String>,
    /// Lower-level cause of the error.
    pub cause: CatalogErrorCause,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            type_name,
            id,
            task_stack: _,
            cause,
        } = self;
        write!(f, "error building {type_name} `{id}`\ncaused by: {cause}")
    }
}

impl std::error::Error for CatalogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

/// Error encountered when building a catalog object, without information about
/// the object being built and the task during which the error occurred.
#[derive(Debug)]
pub enum CatalogErrorCause {
    /// Error encountered during a subrequest.
    Subrequest(Arc<CatalogError>),
    /// Error encountered directly.
    RootCause(eyre::Report),
}

impl fmt::Display for CatalogErrorCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogErrorCause::Subrequest(catalog_error) => write!(f, "{catalog_error}"),
            CatalogErrorCause::RootCause(report) => write!(f, "{report:?}"),
        }
    }
}

impl std::error::Error for CatalogErrorCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CatalogErrorCause::Subrequest(catalog_error) => Some(catalog_error),
            CatalogErrorCause::RootCause(_report) => None,
        }
    }
}

impl From<eyre::Report> for CatalogErrorCause {
    fn from(value: eyre::Report) -> Self {
        match value.downcast() {
            Ok(subrequest_error) => Self::Subrequest(subrequest_error),
            Err(eyre_report) => Self::RootCause(eyre_report),
        }
    }
}

/// Result stored in a [`Catalog`] and thus outputted by
/// [`Catalog::build_blocking()`] and [`Request::get_blocking()`].
pub type CatalogResult<T> = std::result::Result<Arc<T>, Arc<CatalogError>>;

use super::*;

/// Object with an ID (such as a puzzle or color system) that can be stored in
/// the catalog.
pub trait CatalogObject: 'static + Sized + Send + Sync {
    /// Name of the type of object.
    const CATALOG_TYPE_NAME: &'static str;

    /// Returns the metadata for a catalog object.
    fn meta(&self) -> &Arc<CatalogMetadata>;
    /// Returns the subcatalog containing this object type.
    fn get_subcatalog(catalog_data: &CatalogData) -> &SubCatalog<Self>;
    /// Returns a mutable reference to the subcatalog containing this object
    /// type.
    fn get_subcatalog_mut(catalog_data: &mut CatalogData) -> &mut SubCatalog<Self>;
}

impl CatalogObject for Puzzle {
    const CATALOG_TYPE_NAME: &'static str = "puzzle";

    fn meta(&self) -> &Arc<CatalogMetadata> {
        &self.meta
    }
    fn get_subcatalog(catalog_data: &CatalogData) -> &SubCatalog<Self> {
        &catalog_data.puzzles
    }
    fn get_subcatalog_mut(catalog_data: &mut CatalogData) -> &mut SubCatalog<Self> {
        &mut catalog_data.puzzles
    }
}

impl CatalogObject for ColorSystem {
    const CATALOG_TYPE_NAME: &'static str = "color system";

    fn meta(&self) -> &Arc<CatalogMetadata> {
        &self.meta
    }
    fn get_subcatalog(catalog_data: &CatalogData) -> &SubCatalog<Self> {
        &catalog_data.color_systems
    }
    fn get_subcatalog_mut(catalog_data: &mut CatalogData) -> &mut SubCatalog<Self> {
        &mut catalog_data.color_systems
    }
}

impl CatalogObject for TwistSystem {
    const CATALOG_TYPE_NAME: &'static str = "twist system";

    fn meta(&self) -> &Arc<CatalogMetadata> {
        &self.meta
    }
    fn get_subcatalog(catalog_data: &CatalogData) -> &SubCatalog<Self> {
        &catalog_data.twist_systems
    }
    fn get_subcatalog_mut(catalog_data: &mut CatalogData) -> &mut SubCatalog<Self> {
        &mut catalog_data.twist_systems
    }
}

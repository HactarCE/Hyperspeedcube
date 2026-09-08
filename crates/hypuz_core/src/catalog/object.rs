use super::*;

/// Object with an ID (such as a puzzle or color system) that can be stored in
/// the catalog.
pub trait CatalogObject: 'static + Sized + Send + Sync {
    /// Name of the type of object.
    fn catalog_type_name() -> &'static str;

    /// Returns the ID of a catalog object.
    fn id(&self) -> &CatalogId;
}

impl CatalogObject for Puzzle {
    fn catalog_type_name() -> &'static str {
        "puzzle"
    }

    fn id(&self) -> &CatalogId {
        &self.meta.id
    }
}

impl CatalogObject for ColorSystem {
    fn catalog_type_name() -> &'static str {
        "color system"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

impl CatalogObject for TwistSystem {
    fn catalog_type_name() -> &'static str {
        "twist system"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

impl CatalogObject for PuzzleListEntry {
    fn catalog_type_name() -> &'static str {
        "puzzle list entry"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

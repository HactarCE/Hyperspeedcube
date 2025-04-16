use super::*;

#[derive(Default)]
#[doc(hidden)]
pub struct Db {
    /// Puzzles.
    pub puzzles: SubCatalog<Puzzle>,
    /// Color systems.
    pub color_systems: SubCatalog<ColorSystem>,

    /// Sorted list of all puzzle definition authors.
    pub(super) authors: BTreeSet<String>,
}
impl Db {
    pub fn get<T: CatalogObject>(&self) -> &SubCatalog<T> {
        T::get_subcatalog(self)
    }
    pub fn get_mut<T: CatalogObject>(&mut self) -> &mut SubCatalog<T> {
        T::get_subcatalog_mut(self)
    }

    /// Returns a list of puzzle definition authors, in canonical order.
    pub fn authors(&self) -> impl Iterator<Item = &String> {
        self.authors.iter()
    }
}

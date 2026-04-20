use std::collections::HashSet;

use super::*;

/// Builder for a [`Catalog`].
///
/// This type is reference-counted and thus cheap to clone. Clones will
/// reference the same catalog builder, and attempts to add objects or
/// generators will return errors after the catalog has been constructed.
#[derive(Clone)]
pub struct CatalogBuilder {
    catalog_data: Arc<Mutex<Option<CatalogData>>>,
}

impl Default for CatalogBuilder {
    fn default() -> Self {
        Self {
            catalog_data: Arc::new(Mutex::new(Some(CatalogData::default()))),
        }
    }
}

impl CatalogBuilder {
    /// Constructs an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Locks the database.
    ///
    /// **WARNING: This is a low-level operation and can cause deadlocks. Prefer
    /// higher-level methods if possible.**
    fn lock_db(&self) -> Result<MappedMutexGuard<'_, CatalogData>> {
        MutexGuard::try_map(self.catalog_data.lock(), Option::as_mut)
            .map_err(|_| eyre!("catalog cannot be extended after construction"))
    }

    /// Returns the logger for the catalog.
    pub fn logger(&self) -> Result<Logger> {
        Ok(self.lock_db()?.logger.clone())
    }

    /// Adds an object to the catalog.
    pub fn add<T: CatalogObject>(&self, object: Arc<T>) -> Result<()> {
        self.add_generator(Arc::new(Generator::new_constant(object)))?;
        Ok(())
    }

    /// Adds a generator to the catalog.
    pub fn add_generator<T: CatalogObject>(&self, generator: Arc<Generator<T>>) -> Result<()> {
        T::get_subcatalog_mut(&mut *self.lock_db()?).add(generator)
    }

    /// Adds a puzzle generator to the catalog and to the puzzle list.
    pub fn add_puzzle_generator(&self, puzzle_generator: Arc<PuzzleGenerator>) -> Result<()> {
        let meta = Arc::clone(&puzzle_generator.meta);
        self.add_generator(puzzle_generator)?; // this is more likely to fail, so do it first
        self.add_to_puzzle_list(meta)?;
        Ok(())
    }

    /// Adds an entry to the puzzle list.
    ///
    /// This must be called manually for every individual puzzle, puzzle
    /// generator, and puzzle generator example.
    pub fn add_to_puzzle_list(&self, meta: Arc<CatalogMetadata>) -> Result<()> {
        self.lock_db()?.puzzle_list.push(meta);
        Ok(())
    }

    /// Constructs the catalog.
    pub fn build(self) -> Result<Catalog> {
        let mut catalog_data = self
            .catalog_data
            .lock()
            .take()
            .ok_or_eyre("catalog has already been constructed")?;

        // Assemble authors list.
        catalog_data.authors = catalog_data
            .puzzles
            .generators
            .values()
            .flat_map(|g| g.meta.tags.authors())
            .collect::<HashSet<&String>>() // deduplicate
            .into_iter()
            .cloned()
            .sorted() // alphabetize
            .collect();

        Ok(Catalog(Arc::new(catalog_data)))
    }
}

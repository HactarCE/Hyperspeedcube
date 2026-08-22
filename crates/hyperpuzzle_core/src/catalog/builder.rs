use super::*;

/// Builder for a [`Catalog`].
///
/// This type is reference-counted and thus cheap to clone. Clones will
/// reference the same catalog builder.
///
/// After the catalog has been constructed, attempts to add objects or
/// generators will return an error.
#[derive(Clone)]
pub struct CatalogBuilder {
    catalog_data: Arc<Mutex<Option<CatalogData>>>,
    /// Puzzles and puzzle generators to add to the puzzle list.
    puzzle_list: Arc<Mutex<Vec<CatalogId>>>,
}

impl Default for CatalogBuilder {
    fn default() -> Self {
        Self {
            catalog_data: Arc::new(Mutex::new(Some(CatalogData::default()))),
            puzzle_list: Arc::new(Mutex::new(vec![])),
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

    /// Adds a generator to the catalog.
    ///
    /// **Note: Adding a puzzle generator to the catalog using `add_generator()`
    /// does not automatically add it to the puzzle list.**
    pub fn add<T: CatalogObject>(&self, generator: Arc<Generator<T>>) -> Result<()> {
        self.lock_db()?.get_subcatalog_mut().add(generator)
    }

    /// Adds a hook to the catalog.
    pub fn add_hook<T: CatalogObject>(&self, hook: Arc<CatalogHook<T>>) -> Result<()> {
        self.lock_db()?.get_subcatalog_mut().add_hook(hook);
        Ok(())
    }

    /// Adds puzzle definition authors.
    pub fn add_authors<S: AsRef<str>>(
        &self,
        author_names: impl IntoIterator<Item = String>,
    ) -> Result<()> {
        self.lock_db()?.authors.extend(author_names);
        Ok(())
    }

    /// Adds an entry to the puzzle list.
    ///
    /// This must be called manually for every individual puzzle and puzzle
    /// generator that should appear in the puzzle list.
    pub fn add_to_puzzle_list(&self, id: &CatalogId) {
        self.puzzle_list.lock().push(id.clone());
    }
    /// Adds a puzzle generator to the puzzle list.
    ///
    /// Wrapper around [`Self::add_to_puzzle_list()`] for convenience.
    pub fn add_generator_to_puzzle_list(&self, id: &CatalogIdent) {
        self.add_to_puzzle_list(&CatalogId::new(id.clone(), [], None));
    }

    /// Creates a menu.
    ///
    /// Menus can be populated using [`Self::add_menu_node()`].
    ///
    /// Returns an error if the menu already exists.
    pub fn add_menu(&self, menu_id: &'static str, menu_name: String) -> Result<()> {
        match self.lock_db()?.menus.entry(menu_id) {
            hash_map::Entry::Occupied(e) => {
                bail!("menu already exists with name {:?}", e.get().name);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(Menu::new(menu_name));
                Ok(())
            }
        }
    }

    /// Adds a node to a menu.
    ///
    /// Returns an error if such a node already exists or if the menu does not
    /// exist.
    pub fn add_menu_node(
        &self,
        menu_id: &str,
        path: String,
        content: MenuContent,
        priority: i64,
        default: bool,
    ) -> Result<()> {
        self.lock_db()?
            .menus
            .get_mut(menu_id)
            .ok_or_eyre(
                "menu must be created using `CatalogBuilder::add_menu()` before it is populated",
            )?
            .add_node(path, content, priority, default)
    }

    /// Constructs the catalog.
    pub fn build(self) -> Result<Catalog> {
        let catalog_data = self
            .catalog_data
            .lock()
            .take()
            .ok_or_eyre("catalog has already been constructed")?;

        // Check for menu orphans.
        for menu in catalog_data.menus.values() {
            for orphan in menu.orphans() {
                catalog_data.logger.warn(format!(
                    "menu {:?} contains orphan at {:?}",
                    menu.name,
                    orphan.to_string(),
                ));
            }
        }

        let mut ret = Catalog(Arc::new(catalog_data));

        // Populate puzzle list
        let mut puzzle_list = vec![];
        for id in &*self.puzzle_list.lock() {
            match ret.build_blocking::<PuzzleListEntry>(id) {
                Ok(puzzle_list_entry) => puzzle_list.push(puzzle_list_entry),
                Err(e) => ret.logger.error(e),
            }
        }
        puzzle_list.sort(); // sort by ID
        puzzle_list.dedup_by_key(|entry| entry.id.clone());

        Arc::get_mut(&mut ret.0)
            .ok_or_eyre("catalog has already been shared")?
            .puzzle_list = puzzle_list;

        Ok(ret)
    }
}

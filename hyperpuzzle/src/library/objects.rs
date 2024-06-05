use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use hypershape::Space;
use mlua::FromLua;

use super::{LibraryDb, LibraryFile, LibraryFileLoadResult};
use crate::lua::PuzzleParams;

/// Cached puzzle defined in a Lua file.
pub(crate) type CachedPuzzle = Cached<PuzzleParams>;

/// Library object defined in Lua that is cached when constructed.
#[derive(Debug)]
pub(crate) struct Cached<P: LibraryObjectParams> {
    /// Parameters to construct the object.
    pub params: Arc<P>,
    /// Cached constructed object.
    pub constructed: Option<P::Constructed>,
}
impl<P: LibraryObjectParams> Cached<P> {
    /// Returns a new cached object that has not yet been constructed.
    pub fn new(params: P) -> Self {
        Self {
            params: Arc::new(params),
            constructed: None,
        }
    }
}

/// Complete set of parameters that defines a Lua library object.
///
/// This trait also contains assciated functions for operating on constructed
/// objects.
pub(crate) trait LibraryObjectParams: Sized + for<'lua> FromLua<'lua> {
    /// Name of the type of object.
    const NAME: &'static str;

    /// Type of the constructed object.
    type Constructed;

    /// Returns the map in a library from ID of an object of this type to file
    /// where the object is defined.
    fn get_file_map(lib: &LibraryDb) -> &BTreeMap<String, Arc<LibraryFile>>;
    /// Returns the map within a file from ID of an object of this type to file
    /// where the object is defined.
    fn get_id_map_within_file(
        result: &mut LibraryFileLoadResult,
    ) -> &mut HashMap<String, Cached<Self>>;

    /// Returns a default empty object. This is only allowed for some types.
    fn new_constructed(space: &Arc<Space>) -> mlua::Result<Self::Constructed>;
    /// Clones a constructed object into a new space.
    fn clone_constructed(
        existing: &Self::Constructed,
        space: &Arc<Space>,
    ) -> mlua::Result<Self::Constructed>;
    /// Builds an object from this set of parameters.
    fn build(&self, lua: &mlua::Lua, space: &Arc<Space>) -> mlua::Result<Self::Constructed>;
}

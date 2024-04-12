use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use hypershape::Space;
use mlua::FromLua;
use parking_lot::Mutex;

use crate::builder::{AxisSystemBuilder, ShapeBuilder, TwistSystemBuilder};
use crate::lua::{AxisSystemParams, PuzzleParams, ShapeParams, TwistSystemParams};
use crate::puzzle::Puzzle;

use super::{LibraryDb, LibraryFile, LibraryFileLoadResult};

pub type CachedShape = Cached<ShapeParams>;
pub type CachedAxisSystem = Cached<AxisSystemParams>;
pub type CachedTwistSystem = Cached<TwistSystemParams>;
pub type CachedPuzzle = Cached<PuzzleParams>;

#[derive(Debug)]
pub struct Cached<P: LibraryObjectParams> {
    pub params: Arc<P>,
    pub constructed: Option<P::Constructed>,
}
impl<P: LibraryObjectParams> Cached<P> {
    pub fn new(params: P) -> Self {
        Self {
            params: Arc::new(params),
            constructed: None,
        }
    }
}

pub(crate) trait LibraryObjectParams: Sized + for<'lua> FromLua<'lua> {
    const NAME: &'static str;

    type Constructed;

    fn get_file_map(lib: &LibraryDb) -> &BTreeMap<String, Arc<LibraryFile>>;
    fn get_id_map_within_file(
        result: &mut LibraryFileLoadResult,
    ) -> &mut HashMap<String, Cached<Self>>;

    fn new_constructed(space: &Arc<Mutex<Space>>) -> mlua::Result<Self::Constructed>;
    fn clone_constructed(
        existing: &Self::Constructed,
        space: &Arc<Mutex<Space>>,
    ) -> mlua::Result<Self::Constructed>;
    fn build(&self, lua: &mlua::Lua, space: &Arc<Mutex<Space>>) -> mlua::Result<Self::Constructed>;
}

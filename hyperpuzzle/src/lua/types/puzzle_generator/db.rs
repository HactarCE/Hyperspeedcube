use super::*;
use crate::library::LibraryDb;

/// Lua handle to the library of all known puzzles.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleGeneratorDb;
impl LuaUserData for LuaPuzzleGeneratorDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzlegeneratordb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().puzzle_generators.len())
        });

        methods.add_method("add", |lua, Self, spec| {
            let generator_spec = PuzzleGeneratorSpec::from_lua(spec, lua)?;
            LibraryDb::get(lua)?
                .lock()
                .puzzle_generators
                .insert(generator_spec.id.clone(), Arc::new(generator_spec));
            Ok(())
        });
    }
}

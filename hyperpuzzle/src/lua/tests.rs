use super::*;
use crate::library::LibraryDb;

#[test]
fn run_lua_tests() {
    let db = LibraryDb::new();
    let (logger, _log_rx) = LuaLogger::new();
    LuaLoader::new(db, logger)
        .unwrap()
        .run_test_suite("tests.lua", include_str!("tests.lua"));
}

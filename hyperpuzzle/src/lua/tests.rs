use super::*;
use crate::library::LibraryDb;

#[test]
fn run_lua_tests() {
    let db = LibraryDb::new();
    let (logger, log_rx) = LuaLogger::new();
    LuaLoader::new(db, logger).run_test_suite("tests.lua", include_str!("tests.lua"));
}

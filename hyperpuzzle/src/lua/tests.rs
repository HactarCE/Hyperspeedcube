use super::*;

#[test]
fn run_lua_tests() {
    LuaLoader::new().run_test_suite("tests.lua", include_str!("tests.lua"));
}

use hyperpuzzle_core::Logger;

#[test]
fn run_lua_tests() {
    super::loader::LuaLoader::new(&hyperpuzzle_core::Catalog::new(), &Logger::new())
        .unwrap()
        .run_test_suite("tests.lua", include_str!("tests.lua"));
}

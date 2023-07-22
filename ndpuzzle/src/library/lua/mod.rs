use rlua::Lua;

macro_rules! lua_module {
    ($filename:literal) => {
        ($filename, include_str!($filename))
    };
}

const LUA_MODULES: &[(&str, &str)] = &[
    lua_module!("strict.lua"),
    lua_module!("monkeypatch.lua"),
    lua_module!("util.lua"),
    lua_module!("vector.lua"),
];

pub fn new_lua() -> Lua {
    let lua = Lua::new_with(
        rlua::StdLib::BASE
            | rlua::StdLib::TABLE
            | rlua::StdLib::STRING
            | rlua::StdLib::UTF8
            | rlua::StdLib::MATH,
    );

    for (module_name, module_source) in LUA_MODULES {
        log::info!("Loading Lua module {module_name:?}");
        if let Err(e) = lua.context(|ctx| ctx.load(module_source).set_name(module_name)?.exec()) {
            panic!("error loading Lua module {module_name:?}:\n\n{e}\n\n");
        }
    }

    lua
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test_log::test]
    #[test]
    fn run_lua_tests() {
        let lua = new_lua();

        lua.context(|ctx| {
            for pair in ctx.globals().pairs::<String, rlua::Function>() {
                if let Ok((name, function)) = pair {
                    if name.starts_with("test_") {
                        println!("Running {name:?} ...");
                        if let Err(e) = function.call::<(), ()>(()) {
                            panic!("{e}");
                        }
                    }
                }
            }
        })
    }
}

use super::*;

mod builder;
mod color;
mod db;
mod spec;

pub use builder::LuaColorSystem;
pub use color::LuaColor;
pub use db::LuaColorSystemDb;
pub use spec::{from_generated_lua_table, from_lua_table};

fn default_color_from_str(lua: &Lua, s: Option<String>) -> Option<hyperpuzzle_core::DefaultColor> {
    match s?.parse() {
        Ok(c) => Some(c),
        Err(e) => {
            lua.warning(e.to_string(), false);
            None
        }
    }
}

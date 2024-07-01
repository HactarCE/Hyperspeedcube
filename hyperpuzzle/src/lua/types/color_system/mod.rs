use super::*;

mod builder;
mod color;
mod db;
mod params;

pub use builder::LuaColorSystem;
pub use color::LuaColor;
pub use db::LuaColorSystemDb;
pub use params::from_lua_table;

fn default_color_from_str(lua: &Lua, s: Option<String>) -> Option<crate::DefaultColor> {
    match s?.parse() {
        Ok(c) => Some(c),
        Err(e) => {
            lua.warning(e.to_string(), false);
            None
        }
    }
}

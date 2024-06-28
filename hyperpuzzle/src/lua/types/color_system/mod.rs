use super::*;

mod builder;
mod color;

pub use builder::LuaColorSystem;
pub use color::LuaColor;

fn default_color_from_str(lua: &Lua, s: Option<String>) -> Option<crate::DefaultColor> {
    match s?.parse() {
        Ok(c) => Some(c),
        Err(e) => {
            lua.warning(e.to_string(), false);
            None
        }
    }
}

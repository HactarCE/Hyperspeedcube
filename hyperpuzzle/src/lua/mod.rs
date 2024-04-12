//! Lua API for puzzle construction.

#[macro_use]
mod macros;
mod loader;
mod logger;
mod types;

pub(crate) use loader::*;
pub use logger::*;
pub use types::*;

#[cfg(test)]
mod tests;

fn current_filename(lua: &mlua::Lua) -> Option<String> {
    (0..)
        .map_while(|i| lua.inspect_stack(i))
        .filter_map(|debug| Some(debug.source().source?.to_string()))
        .find(|s| !s.starts_with('='))
}

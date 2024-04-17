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

/// Returns a table mapping between axis strings and axis numbers.
fn lua_axes_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table<'_>> {
    let axes_table = lua.create_table()?;
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate().take(6) {
        axes_table.set(LuaIndex(i), c.to_string())?;
        axes_table.set(c.to_string(), LuaIndex(i))?;
        axes_table.set(c.to_ascii_lowercase().to_string(), LuaIndex(i))?;
    }
    seal_table(lua, &axes_table)?;
    Ok(axes_table)
}

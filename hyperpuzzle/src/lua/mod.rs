//! Lua API for puzzle construction.

#[macro_use]
mod macros;
mod loader;
mod logger;
mod tags;
mod types;

pub(crate) use loader::*;
pub use logger::*;
pub use types::*;

#[cfg(test)]
mod tests;

fn lua_current_filename(lua: &mlua::Lua) -> Option<String> {
    (0..)
        .map_while(|i| lua.inspect_stack(i))
        // find user file
        .find_map(|debug| Some(debug.source().source?.strip_prefix('@')?.to_string()))
}

fn lua_stack_trace(lua: &mlua::Lua) -> String {
    lua.create_function(|_lua, ()| Err::<(), _>(mlua::Error::external("")))
        .and_then(|f| f.call(()))
        .unwrap_or_else(|e| e.to_string().trim().to_string())
}

/// Returns a table mapping between axis strings and axis numbers.
fn lua_axes_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let axes_table = lua.create_table()?;
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate().take(6) {
        axes_table.set(LuaIndex(i), c.to_string())?;
        axes_table.set(c.to_string(), LuaIndex(i))?;
        axes_table.set(c.to_ascii_lowercase().to_string(), LuaIndex(i))?;
    }
    seal_table(lua, &axes_table)?;
    Ok(axes_table)
}

fn lua_warn_fn<E: ToString>(lua: &mlua::Lua) -> impl '_ + Copy + Fn(E) {
    move |error| lua.warning(error.to_string(), false)
}

fn result_to_ok_or_warn<T, E>(
    warn_fn: impl Copy + Fn(E),
) -> impl Copy + Fn(Result<T, E>) -> Option<T> {
    move |result| match result {
        Ok(value) => Some(value),
        Err(e) => {
            warn_fn(e);
            None
        }
    }
}

fn deep_copy_value(lua: &mlua::Lua, value: mlua::Value) -> mlua::Result<mlua::Value> {
    match value {
        mlua::Value::Table(table) => Ok(mlua::Value::Table(deep_copy_table(lua, table)?)),
        _ => Ok(value),
    }
}
fn deep_copy_table(lua: &mlua::Lua, table: mlua::Table) -> mlua::Result<mlua::Table> {
    let kv_pairs = table
        .pairs()
        .map(|pair| {
            let (k, v) = pair?;
            Ok((deep_copy_value(lua, k)?, deep_copy_value(lua, v)?))
        })
        .collect::<mlua::Result<Vec<_>>>()?;
    lua.create_table_from(kv_pairs)
}

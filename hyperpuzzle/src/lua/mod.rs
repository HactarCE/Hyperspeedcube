//! Lua API for puzzle construction.

#[macro_use]
mod macros;
mod env;
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

fn create_sealed_table_with_index_metamethod(
    lua: &mlua::Lua,
    index_metamethod: mlua::Function,
) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    let newindex_metamethod =
        lua.create_function(|_lua, ()| Err::<(), _>(mlua::Error::external("table is sealed")))?;
    let metatable = lua.create_table_from([
        (mlua::MetaMethod::Index.name(), index_metamethod),
        (mlua::MetaMethod::NewIndex.name(), newindex_metamethod),
    ])?;
    table.set_metatable(Some(metatable));
    Ok(table)
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

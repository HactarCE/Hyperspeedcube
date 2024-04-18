use std::collections::HashSet;

use mlua::prelude::*;

#[derive(Debug, Default)]
struct WarningSet(HashSet<&'static str>);

/// Resets the list of warnings used by [`first_warning()`].
pub(crate) fn reset_warnings(lua: &Lua) {
    lua.set_app_data(WarningSet::default());
}

/// Returns `true` if this method has been called before in the current Lua
/// invocation with the same ID; otherwise returns `false`.
pub(crate) fn first_warning(lua: &Lua, warning_id: &'static str) -> bool {
    match lua.app_data_mut::<WarningSet>() {
        Some(mut warning_set) => warning_set.0.insert(warning_id),
        None => {
            log::error!("unable to track Lua warnings!");
            true
        }
    }
}

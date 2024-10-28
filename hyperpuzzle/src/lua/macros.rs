/// Unpacks a table into several fixed fields of specific types and returns an
/// error if any conversion fails or if there are extra fields.
macro_rules! unpack_table {
    (
        $lua:ident.unpack($table:ident { $($key:ident),+ $(,)? })
    ) => {
        let table: ::mlua::Table = $table;
        let mut pairs: ::hypermath::collections::VecMap<String, ::mlua::Value> =
            table.clone().pairs().into_iter().collect::<::mlua::Result<_>>()?;
        $(
            let key_str = $crate::lua::macros::key_str(stringify!($key));
            $key = ::mlua::ErrorContext::with_context(
                $lua.unpack(pairs.remove(key_str).into_lua($lua)?),
                |e| format!("bad value for key {key_str:?}: {e}"),
            )?;
        )+
        if let Some(extra_key) = pairs.keys().next() {
            return Err(::mlua::Error::external(format!(
                "unknown table key: {extra_key:?}; allowed keys: {:?}",
                [$($crate::lua::macros::key_str(stringify!($key))),+],
            )));
        }
    };
}

pub(crate) fn key_str(s: &str) -> &str {
    // strip `r#` prefix on raw identifiers
    // because we use `type` as a table key
    s.strip_prefix("r#").unwrap_or(s)
}

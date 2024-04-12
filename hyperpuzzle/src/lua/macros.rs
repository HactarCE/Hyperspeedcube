/// Unpacks a table into several fixed fields of specific types and returns an
/// error if any conversion fails or if there are extra fields.
macro_rules! unpack_table {
    (
        $lua:ident.unpack($table:ident { $($key:ident),+ $(,)? })
    ) => {
        let table: ::mlua::Table<'_> = $table;
        let mut pairs: ::std::collections::HashMap<String, ::mlua::Value<'_>> =
            table.clone().pairs().into_iter().collect::<::mlua::Result<_>>()?;
        $(
            $key = ::mlua::ErrorContext::with_context(
                $lua.unpack(pairs.remove(stringify!($key)).into_lua($lua)?),
                |_| format!("bad value for key {:?}", stringify!($key)),
            )?;
        )+
        if let Some(extra_key) = pairs.keys().next() {
            return Err(::mlua::Error::external(format!(
                "unknown table key: {extra_key:?}; allowed keys: {:?}",
                [$(stringify!($key)),+],
            )));
        }
    };
}

/// Unpacks a table into several fixed fields of specific types and returns an
/// error if any conversion fails or if there are extra fields.
macro_rules! let_from_map {
    // braces inside parens so that rustfmt formats call sites
    ($ctx:expr, $map:expr, {
        $( let $key:ident $(: $key_type:ty)?; )*
        $( ..$rest:ident )?
    }) => {
        let mut map: ::rhai::Map = $map;
        $(
            let key_str = $crate::macros::key_str(stringify!($key));
            let $key $(: $key_type)? = $crate::errors::InKey::in_key(
                $crate::convert::from_rhai_opt($ctx, map.remove(key_str)),
                key_str,
            )?;
        )*
        $(
            let $rest = std::mem::take(&mut map);
        )?
        if !map.is_empty() {
            return Err(format!(
                "unknown map keys: {:?}; allowed keys: {:?}",
                map.keys().collect::<Vec<_>>(),
                [$($crate::macros::key_str(stringify!($key))),+],
            )
            .into());
        }
    };
}

pub(crate) fn key_str(s: &str) -> &str {
    // strip `r#` prefix on raw identifiers
    // because we use `type` as a table key
    s.strip_prefix("r#").unwrap_or(s)
}

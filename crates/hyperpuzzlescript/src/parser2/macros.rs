/// Implements `std::format::Display` for a type using arguments to `write!()`.
macro_rules! impl_display {
    ( for $typename:ty, $( $fmt_arg:expr ),+ $(,)? ) => {
        impl std::fmt::Display for $typename {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $( $fmt_arg ),+ )
            }
        }
    };
}

/// Parses the first matching syntax rule using `try_parse()` or returns an
/// error listing all of them if none match.
macro_rules! parse_one_of {
    ( $p:expr, [ $( @ $varname:ident, )* $first_rule:expr $(, $rule:expr )* $(,)? ] $(,)? ) => {
        {
            let this_var_is_unique = $first_rule;
            parse_one_of!(
                $p,
                [
                    $( @ $varname, )*
                    @ this_var_is_unique,
                    $( $rule, )*
                ],
            )
        }
    };
    ( $p:expr, [ $( @ $varname:ident, )+ ], ) => {
        None
            $( .or_else(|| $p.try_parse(&$varname)) )+
            .unwrap_or_else(|| $p.expected(
                crate::util::join_with_conjunction("or", &[
                    $( $varname.to_string(), )+
                ])
            ))
    };
}

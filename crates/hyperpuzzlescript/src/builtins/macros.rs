macro_rules! hps_fns {
    (
        $(
            $( #[doc = $doc:literal] )*
            $( #[kwargs($($kwargs:tt)*)] )?
            fn $fn_name:ident($( $param:tt : $param_ty:ident $( ( $($param_ty_contents:tt)* ) )? ),* $(,)?)
                -> $ret_ty:ident $( ( $($ret_ty_contents:tt)* ) )?
            { $($body:tt)* }
        )*
    ) => {
        [$(
            // TODO: docs
            hps_fns!(
                @ stringify!($fn_name),
                [ $($doc),* ],
                [ $( ( $param: ( $param_ty $( ( $($param_ty_contents)* ) )? ) ) )* ],
                kwargs,
                ($ret_ty $( ( $($ret_ty_contents)* ) )?),
                {
                    unpack_kwargs!(kwargs, $($($kwargs)*)?);
                    $($body)*
                }
            )
        ),*]
    };
    (@ $fn_name:expr, $docs:tt, [ ($ctx:ident: (EvalCtx)) ($args:ident: (Args)) ], $kwargs:ident, $ret_ty:tt, $body:tt) => {
        hps_builtin_fn!(
            $fn_name, $docs, $ctx, $args, $kwargs,
            $crate::FnType { params: None, ret: ty_from_tokens! $ret_ty },
            $body
        )
    };
    (@ $fn_name:expr, $docs:tt, [ ($ctx:ident: (EvalCtx)) $($args:tt)* ], $($rest:tt)* ) => {
        hps_builtin_fn!($fn_name, $docs, $ctx, [$($args)*], $($rest)*)
    };
    (@ $fn_name:expr, $docs:tt, [ $($args:tt)* ], $($rest:tt)* ) => {
        hps_builtin_fn!($fn_name, $docs, _ctx, [$($args)*], $($rest)*)
    };
}

/// This macro takes a list of function definitions. Each function definition is
/// a tuple:
///
/// ```ignore
/// (
///     "fn_name",
///     |ctx, arg1: Type1, arg2: Type2, ..| -> RetType { .. }
/// )
/// ```
macro_rules! hps_short_fns {
    [
        $((
            $fn_name:literal, |
                $ctx:tt
                $(, $param:tt : $param_ty:ident $( ( $($param_ty_inner:tt)* ) )? )*
            | -> $ret_ty:ident $( ( $($ret_ty_inner:tt)* ) )?
            { $($body:tt)* }
        )),* $(,)?
    ] => {
        [$(hps_builtin_fn!(
            $fn_name,
            [], // docs
            $ctx,
            [$( ( $param: ($param_ty $(($($param_ty_inner)*))?) ) )*],
            kwargs,
            ($ret_ty $(($($ret_ty_inner)*))?),
            {
                unpack_kwargs!(kwargs);
                $($body)*
            }
        )),*]
    };
}

/// This macro can be invoked in either of two ways:
///
/// - name, docs, ctx, [ param1: (param1_ty), param2: (param2_ty) ], kwargs, (ret_ty), body
/// - name, docs, ctx, args, kwargs, fn_type, body
macro_rules! hps_builtin_fn {
    (
        $fn_name:expr, $docs:tt, $ctx:tt, [ $( ( $param:tt : $param_ty:tt ) )* ], $kwargs:ident,
        $ret_ty:tt, $body:tt
     ) => {
        hps_builtin_fn!(
            $fn_name, $docs, $ctx, args, $kwargs,
            $crate::FnType {
                params: Some(vec![$( ty_from_tokens! $param_ty ),*]),
                ret: ty_from_tokens! $ret_ty,
            },
            {
                #[allow(unused)]
                let mut args = args;
                #[allow(unused)]
                let mut i = 0;
                $(
                    unpack_val!(let $param = std::mem::take(&mut args[i]), $param_ty);
                    #[allow(unused_assignments)]
                    { i += 1; }
                )*
                $body
            }
        )
    };

    ( $fn_name:expr, $docs:tt, $ctx:tt, $args:ident, $kwargs:ident, $fn_type:expr, $body:tt ) => {
        (
            $fn_name,
            $crate::FnOverload {
                ty: $fn_type,
                call: std::sync::Arc::new(|$ctx, $args, $kwargs| {
                    let output: $crate::ValueData = $body.into();
                    Ok(output.at($crate::BUILTIN_SPAN))
                }),
                debug_info: $crate::FnDebugInfo::Internal($fn_name),
                parent_scope: None, // built-in functions never modify local variables
                docs: Some($docs.as_slice()).filter(|arr| !arr.is_empty()),
            },
        )
     };
}

macro_rules! hps_fn {
    ($fn_name:expr, || $($rest:tt)*) => {
        hps_fn!($fn_name, |_ctx| $($rest)*)
    };
    ($fn_name:expr, |$($param:tt : $param_ty:ident $( ( $($param_ty_contents:tt)* ) )?),*| -> $ret_ty:ident $( ( $($ret_ty_contents:tt)* ) )? { $($body:tt)* }) => {
        hps_fn!($fn_name, |_ctx, $($param : $param_ty $( ( $($param_ty_contents)* ) )?),*| -> $ret_ty $( ( $($ret_ty_contents)* ) )? { $($body)* })
    };
    ($fn_name:expr, |$ctx:ident $(, $param:tt : $param_ty:ident $( ( $($param_ty_contents:tt)* ) )?)* $(,)?| -> $ret_ty:ident $( ( $($ret_ty_contents:tt)* ) )? { $($body:tt)* }) => {
        hps_fn!(
            $fn_name,
            (
                Some(vec![$(ty_from_tokens!($param_ty $( ( $($param_ty_contents)* ) )?)),*]),
                ty_from_tokens!($ret_ty $( ( $($ret_ty_contents)* ) )?),
            ),
            |$ctx, args, kwargs| {
                #[allow(unused)]
                let mut args = args;
                #[allow(unused)]
                let mut i = 0;
                $(
                    unpack_val!(let $param = std::mem::take(&mut args[i]), $param_ty $( ( $($param_ty_contents)* ) )?);
                    #[allow(unused_assignments)]
                    {
                        i += 1;
                    }
                )*
                unpack_kwargs!(kwargs);
                $($body)*
            }
        )
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), || -> $($rest:tt)*) => {
        hps_fn!($fn_name, ($params, $ret), | | -> $($rest)*)
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$args:ident $(,)?| { $($body:tt)* }) => {
        hps_fn!($fn_name, ($params, $ret), |_ctx, $args, kwargs| { $($body)* })
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$ctx:ident, $args:ident, $kwargs:ident $(,)?| { $($body:tt)* }) => {
        (
            $fn_name,
            $crate::FnOverload {
                ty: $crate::FnType { params: $params, ret: $ret },
                call: std::sync::Arc::new(|$ctx, $args, $kwargs| {
                    let output = { $($body)* };
                    Ok($crate::ValueData::from(output).at($crate::BUILTIN_SPAN))
                }),
                debug_info: $crate::FnDebugInfo::Internal($fn_name),
                parent_scope: None, // built-in functions never modify local variables
                docs: None,
            },
        )
    };
    (|$($param:tt : $param_ty:ident),*| $body:expr) => {
        compile_error!("missing return type")
    };
    (|$($param:tt $(: $param_ty:ident)?),*| $($rest:tt)*) => {
        compile_error!("missing argument type")
    };
}

macro_rules! ty_from_tokens {
    ( ( $($inner:tt)* ) ) => { ty_from_tokens!($($inner)*) };

    // Standard types
    (List) => { $crate::Type::List(None) };
    ($ty:ident ?) => { ty_from_tokens!($ty).optional() };

    ($collection_ty:ident ( $($inner:tt)* )) => {
        $crate::Type::$collection_ty(Some(std::boxed::Box::new(ty_from_tokens!($($inner)*))))
    };
    ($collection_ty:ident ( $($inner:tt)* ) ?) => {
        $crate::Type::Union(vec![
            $crate::Type::$collection_ty(Some(std::boxed::Box::new(ty_from_tokens!($($inner)*)))),
            ty_from_tokens!(Null),
        ])
    };
    ($($tok:tt)*) => { $crate::Type::$($tok)* };
}

macro_rules! unpack_val {
    (let ($dst:tt, $span:tt) = $val:expr, $($ty:tt)*) => {
        let val = $val;
        let $span = val.span;
        let $dst = unpack_val!(val, $($ty)*);
    };
    (let $dst:tt = $val:expr, $($ty:tt)*) => {
        let val = $val;
        let $dst = unpack_val!(val, $($ty)*);
    };

    // Standard types
    ($val:ident, ($($ty:tt)*)) => { unpack_val!($val, $($ty)*) };
    ($val:ident, Any)  => { $val };
    ($val:ident, Null) => { unpack_val!(@$val, (Null), $crate::ValueData::Null => ()) };
    ($val:ident, Bool) => { unpack_val!(@$val, (Bool), $crate::ValueData::Bool(b) => b) };
    ($val:ident, Num)  => { unpack_val!(@$val, (Num),  $crate::ValueData::Num(n) => n) };
    ($val:ident, Str)  => { unpack_val!(@$val, (Str),  $crate::ValueData::Str(s) => s) };
    ($val:ident, List) => { unpack_val!(@$val, (List), $crate::ValueData::List(l) => l) };
    ($val:ident, EmptyList) => { $val.typecheck(Type::EmptyList)? };
    ($val:ident, NonEmptyList) => {
        $val.typecheck(Type::NonEmptyList(None))?;
        unpack_val!(@$val, (List), $crate::ValueData::List(l) => l)
    };
    ($val:ident, Map)  => { unpack_val!(@$val, (Map),  $crate::ValueData::Map(m) => m) };
    ($val:ident, Fn)   => { unpack_val!(@$val, (Fn),   $crate::ValueData::Fn(f) => f) };
    ($val:ident, Vec)  => { unpack_val!(@$val, (Vec),  $crate::ValueData::Vec(v) => v) };
    ($val:ident, Type)  => { unpack_val!(@$val, (Type),  $crate::ValueData::Type(t) => t) };
    // Euclid types
    ($val:ident, EuclidPoint) => {
        unpack_val!(@$val, (EuclidPoint),  $crate::ValueData::EuclidPoint(v) => v)
    };
    ($val:ident, EuclidTransform) => {
        unpack_val!(@$val, (EuclidTransform),  $crate::ValueData::EuclidTransform(v) => v)
    };
    ($val:ident, EuclidPlane) => {
        unpack_val!(@$val, (EuclidPlane),  $crate::ValueData::EuclidPlane(v) => v)
    };
    ($val:ident, EuclidRegion) => {
        unpack_val!(@$val, (EuclidRegion),  $crate::ValueData::EuclidRegion(v) => v)
    };
    // Special predicates
    ($val:ident, Int)  => { $val.as_int()? };
    ($val:ident, Nat)  => { $val.as_uint()? };
    // Fallback
    ($val:ident, $other:ident) => { unpack_val!(@$val, ($other), $crate::ValueData::$other(inner) => inner) };

    // Optional
    ($val:ident, $primitive:ident ?) => {
        // TODO: error message doesn't say that `Null` is allowed
        match $val.is_null() {
            true => None,
            false => Some(unpack_val!($val, $primitive)),
        }
    };

    // Collection types
    ($val:ident, List ( $($inner:tt)* )) => {
        unpack_val!(@$val, (List ( $($inner)* )), $crate::ValueData::List(l) => {
            std::sync::Arc::unwrap_or_clone(l)
                .into_iter()
                .map(|elem| {
                    unpack_val!(let e = elem, $($inner)*);
                    Ok(e)
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        })
    };

    (@$val:ident, ($($expected_ty:tt)*), $pattern:pat => $ret:expr) => {
        match $val.data {
            $pattern => $ret,
            _ => return Err($val.type_error(ty_from_tokens!($($expected_ty)*))),
        }
    };
}

macro_rules! unpack_kwargs {
    ($kwargs:expr, $target:ident) => {
        let $target = $kwargs;
    };
    ($kwargs:expr $(, $name:ident: $ty:ident $( ( $($ty_contents:tt)* ) )? $( = $default:expr )?)+ $(,)?) => {
        unpack_kwargs!($kwargs $(, $name: ( $ty $( ( $($ty_contents)* ) )? ) $( = $default )?)+)
    };
    ($kwargs:expr $(, $name:ident: ( $($ty:tt)* ) $( = $default:expr )?)* $(,)?) => {
        #[allow(unused_mut)]
        let mut kwargs = $kwargs;
        $(
            let $name = match kwargs.swap_remove(stringify!($name)) {
                Some(val) => unpack_val!(val, $($ty)*),
                None => {
                    #[allow(unused)]
                    let mut val = None;
                    $( val = Some($default); )?
                    val.ok_or_else(|| {
                        $crate::Error::MissingRequiredNamedParameter(stringify!($name).into())
                            .at($crate::BUILTIN_SPAN)
                    })?
                }
            };
        )*

        if !kwargs.is_empty() {
            return Err($crate::Error::UnusedFnArgs {
                args: kwargs.into_iter().map(|(k, v)| (k, v.span)).collect(),
            }
            .at($crate::BUILTIN_SPAN));
        }
    };
}

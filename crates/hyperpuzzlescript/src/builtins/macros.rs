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
            |$ctx, args| {
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
                $($body)*
            }
        )
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), || -> $($rest:tt)*) => {
        hps_fn!($fn_name, ($params, $ret), | | -> $($rest)*)
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$args:ident $(,)?| { $($body:tt)* }) => {
        hps_fn!($fn_name, ($params, $ret), |_ctx, $args| { $($body)* })
    };
    ($fn_name:expr, ($params:expr, $ret:expr $(,)?), |$ctx:ident, $args:ident $(,)?| { $($body:tt)* }) => {
        (
            $fn_name,
            $crate::FnOverload {
                ty: $crate::FnType { params: $params, ret: $ret },
                call: std::sync::Arc::new(|$ctx, $args| {
                    let output = { $($body)* };
                    Ok($crate::ValueData::from(output).at($crate::BUILTIN_SPAN))
                }),
                debug_info: $crate::FnDebugInfo::Internal($fn_name),
                parent_scope: None, // built-in functions never modify local variables
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
    // Standard types
    (Fn) => {
        $crate::Type::Fn(std::default::Default::default())
    };
    (List) => {
        $crate::Type::List(std::boxed::Box::new($crate::Type::Any))
    };
    (Map) => {
        $crate::Type::Map(std::boxed::Box::new($crate::Type::Any))
    };
    // Special predicates
    (Int) => { $crate::Type::Num };
    (Uint) => { $crate::Type::Num };
    // Euclid types
    (EPoint) => { $crate::Type::EuclidPoint };
    (ETransform) => { $crate::Type::EuclidTransform };
    (EPlane) => { $crate::Type::EuclidPlane };
    (ERegion) => { $crate::Type::EuclidRegion };

    ($collection_ty:ident ( $($inner:tt)* )) => {
        $crate::Type::$collection_ty(std::boxed::Box::new(ty_from_tokens!($($inner)*)))
    };
    ($($tok:tt)*) => { $crate::Type::$($tok)* };
}

macro_rules! unpack_val {
    (let $dst:ident = $($rest:tt)*) => { unpack_val!(let ($dst, _) = $($rest)*) };
    (let ($dst:ident, $span:tt) = $val:expr, $($ty:tt)*) => {
        let val = $val;
        let $span = val.span;
        let $dst = unpack_val!(val, $($ty)*);
    };

    // Standard types
    ($val:ident, Any)  => { $val };
    ($val:ident, Null) => { unpack_val!(@$val, (Null), $crate::ValueData::Null => ()) };
    ($val:ident, Bool) => { unpack_val!(@$val, (Bool), $crate::ValueData::Bool(b) => b) };
    ($val:ident, Num)  => { unpack_val!(@$val, (Num),  $crate::ValueData::Num(n) => n) };
    ($val:ident, Str)  => { unpack_val!(@$val, (Str),  $crate::ValueData::Str(s) => s) };
    ($val:ident, List) => { unpack_val!(@$val, (List), $crate::ValueData::List(l) => l) };
    ($val:ident, Map)  => { unpack_val!(@$val, (Map),  $crate::ValueData::Map(m) => m) };
    ($val:ident, Fn)   => { unpack_val!(@$val, (Fn),   $crate::ValueData::Fn(f) => f) };
    ($val:ident, Vec)  => { unpack_val!(@$val, (Vec),  $crate::ValueData::Vec(v) => v) };
    // Euclid types
    ($val:ident, EPoint) => {
        unpack_val!(@$val, (EPoint),  $crate::ValueData::EuclidPoint(v) => v)
    };
    ($val:ident, ETransform) => {
        unpack_val!(@$val, (ETransform),  $crate::ValueData::EuclidTransform(v) => v)
    };
    ($val:ident, EPlane) => {
        unpack_val!(@$val, (EPlane),  $crate::ValueData::EuclidPlane(v) => v)
    };
    ($val:ident, ERegion) => {
        unpack_val!(@$val, (ERegion),  $crate::ValueData::EuclidRegion(v) => v)
    };
    // Special predicates
    ($val:ident, Int)  => { $val.as_int()? };
    ($val:ident, Uint)  => { $val.as_uint()? };
    // Fallback
    ($val:ident, $other:ident) => { compile_error!(concat!("unsupported type: ", stringify!($other))) };

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

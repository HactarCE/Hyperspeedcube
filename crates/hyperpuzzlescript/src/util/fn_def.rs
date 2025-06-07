//! Macros for defining HPS functions in Rust.

/// Defines a list of functions.
///
/// This macro can be called with a list of `fn` definitions or a list of
/// closures. These two approaches have tradeoffs.
///
/// # Examples
///
/// ## Using `fn` definitions
///
/// - **Supports:** documentation, keyword arguments, variadics
/// - **Does not support:** operator overloads
///
/// ```
/// # use hyperpuzzlescript::*;
/// # use std::sync::Arc;
/// let mut scope = Scope::new();
/// scope.register_builtin_functions(hps_fns![
///     /// This is user-facing documentation.
///     ///
///     /// Wonderful!
///     #[kwargs(a_required: i64, b_optional: Option<Num>, c_with_default: u8 = 15)]
///     fn my_function(ctx: EvalCtx, (arg1, span): String, optional_arg2: Arc<Map>) -> String {
///         if arg1.is_empty() {
///             return Err(Error::User("got your span!".into()).at(span));
///         }
///         if a_required == 0 && c_with_default == 15 {
///             return Err(Error::User("oh no!".into()).at(ctx.caller_span))
///         }
///         arg1
///     }
///     // For overloads, just add more definitions of the same function.
///     // `ctx` and `#[kwargs(...)]` are optional.
///     // Return type annotations are required. Use `()` for `Type::Null`.
///     fn my_function(arg1: String) -> () {
///         println!("{arg1}");
///     }
///     // Sequential and/or keyword arguments may be variable.
///     #[kwargs(var_containing_kwargs)]
///     fn variadic(var_containing_args: Args) -> usize {
///         var_containing_args.len() + var_containing_kwargs.len()
///     }
/// ])
/// .expect("error defining built-in functions");
/// ```
///
/// ## Using closures
///
/// - More compact than the `fn` definitions
/// - **Supports:** operator overloads
/// - **Does not support:** documentation, keyword arguments, variadics
///
/// ```
/// # use hyperpuzzlescript::*;
/// # use std::sync::Arc;
/// let mut scope = Scope::new();
/// scope.register_builtin_functions(hps_fns![
///     // For overloads, just add more definitions of the same function.
///     // `ctx` is **required** and must **not** have a type annotation.
///     // Return type annotations are required. Use `()` for `Type::Null`.
///     ("my_function", |ctx, (arg1, span): String, optional_arg2: Arc<Map>| -> String {
///         if arg1.is_empty() {
///             return Err(Error::User("got your span!".into()).at(span));
///         }
///         if optional_arg2.is_empty() {
///             return Err(Error::User("oh no!".into()).at(ctx.caller_span))
///         }
///         arg1
///     }),
///     // Use `_` to ignore `ctx`.
///     ("my_function", |_, arg1: String| -> () { println!("{arg1}") }),
///     // Operator overload
///     ("+", |_, a: String, b: Num| -> String { format!("{a}{b}") }),
/// ])
/// .expect("error defining built-in functions");
/// ```
#[macro_export]
macro_rules! hps_fns {
    [
        $(
            $( #[doc = $doc:literal] )*
            $( #[kwargs($($kwargs:tt)*)] )?
            fn $fn_name:ident($($params:tt)*) -> $ret:ty
            $body:block
        )*
    ] => {
        [$(
            (
                stringify!($fn_name),
                $crate::FnOverload {
                    ty: fn_type!([$($params)*] -> $ret),
                    call: std::sync::Arc::new(move |ctx, args, kwargs| {
                        #[allow(unused, clippy::redundant_locals)]
                        let ctx = ctx;
                        unpack_args!(ctx, args, [$($params)*]);
                        unpack_kwargs!(kwargs$(, $($kwargs)*)?);
                        let ret: $ret = $body;
                        Ok($crate::ValueData::from(ret).at($crate::BUILTIN_SPAN))
                    }),
                    debug_info: $crate::FnDebugInfo::Internal(stringify!($fn_name)),
                    parent_scope: None, // built-in functions never modify local variables
                    docs: Some([$($doc),*].as_slice()).filter(|arr| !arr.is_empty()),
                },
            )
        ),*]
    };

    [
        $((
            $fn_name:literal,
            | $ctx:tt $(, $($param:tt : $param_ty:ty),* $(,)?)? | -> $ret:ty
            $body:block
        )),* $(,)?
    ] => {
        [$(
            (
                $fn_name,
                $crate::FnOverload {
                    ty: fn_type!([$($($param: $param_ty),*)?] -> $ret),
                    call: std::sync::Arc::new(move |ctx, args, kwargs| {
                        // `$param_ty:ty` makes the literal identifier `Args`
                        // opaque so we can't do variadics here.,
                        unpack_args!(ctx, args, [$($($param: $param_ty),*)?]);
                        let $ctx = ctx;
                        unpack_kwargs!(kwargs);
                        let ret: $ret = $body;
                        Ok($crate::ValueData::from(ret).at($crate::BUILTIN_SPAN))
                    }),
                    debug_info: $crate::FnDebugInfo::Internal($fn_name),
                    parent_scope: None, // built-in functions never modify local variables
                    docs: None,
                },
            )
        ),*]
    };
}

/// Constructs a [`crate::FnType`] from a list of parameters and a return type.
#[macro_export]
macro_rules! fn_type {
    ([$ctx:tt: EvalCtx $(, $($rest:tt)*)?] -> $ret:ty) => {
        fn_type!([$($($rest)*)?] -> $ret)
    };
    ([$args:tt: Args $(,)?] -> $ret:ty) => {
        $crate::FnType {
            params: vec![],
            is_variadic: true,
            ret: $crate::hps_ty::<$ret>(),
        }
    };
    ([$($param:tt : $param_ty:ty),* $(,)?] -> $ret:ty) => {
        $crate::FnType {
            params: vec![$( $crate::hps_ty::<$param_ty>() ),*],
            is_variadic: false,
            ret: $crate::hps_ty::<$ret>(),
        }
    };
}

/// Unpacks arguments using [`crate::util::pop_arg()`] and
/// [`crate::util::expect_end_of_args()`].
#[macro_export]
macro_rules! unpack_args {
    ($ctx:ident, $args:ident, [$ctx_out:tt : EvalCtx $(, $($rest:tt)*)?]) => {
        let $ctx_out = $ctx;
        unpack_args!($ctx, $args, [$($($rest)*)?]);
    };
    ($ctx:ident, $args:ident, [$args_out:tt : Args $(,)?]) => {
        let $args_out = $args;
    };
    ($ctx:ident, $args:ident, [$($param:tt : $param_ty:ty),* $(,)?]) => {
        #[allow(unused_mut)]
        let mut args = $args.into_iter();
        $(
            let $param = $crate::util::pop_arg::<fn_arg_ty!($param: $param_ty)>(
                &mut args,
                $crate::BUILTIN_SPAN,
            )?;
        )*
        $crate::util::expect_end_of_args(args)?;
    };
}

/// Unpacks keyword arguments using [`pop_kwarg!`] and
/// [`crate::util::expect_end_of_kwargs()`].
#[macro_export]
macro_rules! unpack_kwargs {
    ($kwargs:expr, $target:ident $(,)?) => {
        #[allow(unused_mut)]
        let mut $target = $kwargs;
    };
    ($kwargs:expr $(, $param:tt: $param_ty:ty $( = $default:expr )?)* $(,)?) => {
        #[allow(unused_mut)]
        let mut kwargs = $kwargs;
        $(
            pop_kwarg!(kwargs, $param: $param_ty $( = $default )?);
        )*
        $crate::util::expect_end_of_kwargs(kwargs, $crate::BUILTIN_SPAN)?;
    };
}

/// Unpacks a keyword argument using [`crate::util::pop_kwarg()`], with support
/// for an optional parameter.
///
/// # Examples
///
/// ```
/// # use hyperpuzzlescript::*;
/// struct MyKwargsOutput {
///     a: Num,
///     b: Num,
///     c: Option<Str>,
///     d: Str,
///     d_span: Span,
/// }
///
/// fn unpack_my_kwargs(mut kwargs: Map) -> Result<MyKwargsOutput> {
///     // `kwargs` must be mutable, and the surrounding
///     // function must return `hyperpuzzlescript::Result<T>`.
///
///     hyperpuzzlescript::pop_kwarg!(kwargs, a: Num); // required
///     hyperpuzzlescript::pop_kwarg!(kwargs, b: Num = 15.0); // optional with default value
///     hyperpuzzlescript::pop_kwarg!(kwargs, c: Option<Str>); // optional
///     hyperpuzzlescript::pop_kwarg!(kwargs, (d, d_span): Str); // with span
///     Ok(MyKwargsOutput { a, b, c, d, d_span })
/// }
/// ```
#[macro_export]
macro_rules! pop_kwarg {
    ($kwargs:ident, $name:tt: $param_ty:ty = $default:expr) => {
        let $name = $crate::util::pop_kwarg::<Option<$param_ty>>(
            &mut $kwargs,
            fn_arg_name!($name),
            $crate::BUILTIN_SPAN,
        )?
        .unwrap_or_else(|| -> $param_ty { $default });
    };
    ($kwargs:ident, $name:tt: $param_ty:ty) => {
        let $name = $crate::util::pop_kwarg::<fn_arg_ty!($name: $param_ty)>(
            &mut $kwargs,
            fn_arg_name!($name),
            $crate::BUILTIN_SPAN,
        )?;
    };
}

/// Returns the name to use when unpacking an argument.
#[macro_export]
macro_rules! fn_arg_name {
    (($name:ident, $span:ident)) => {
        stringify!($name)
    };
    ($name:tt) => {
        stringify!($name)
    };
}

/// Returns the type to use when unpacking an argument.
///
/// This is either the type `T`, or `Spanned<T>` if the span should be included.
#[macro_export]
macro_rules! fn_arg_ty {
    (($name:ident, $span:ident) : $ty:ty) => { $crate::Spanned<$ty> };
    ($name:tt : $ty:ty) => { $ty };
}

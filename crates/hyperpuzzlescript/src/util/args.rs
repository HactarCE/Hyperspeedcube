use crate::{Error, EvalCtx, FromValue, Map, Result, Span, SpecialVar, Value};

/// Removes and returns the next argument from `args`.
///
/// Returns an error if there is no next argument (and `T` is non-nullable) or
/// if the argument is not of type `T`.
pub fn pop_arg<T: FromValue>(args: &mut impl Iterator<Item = Value>, arg_span: Span) -> Result<T> {
    match args.next() {
        Some(v) => T::from_value(v),
        None => T::from_value(Value::NULL).map_err(|_| {
            Error::MissingRequiredPositionalParameter { ty: T::hps_ty() }.at(arg_span)
        }),
    }
}

/// Returns an error if there are any more arguments.
pub fn expect_end_of_args(mut args: impl Iterator<Item = Value>) -> Result<()> {
    match args.next() {
        None => Ok(()),
        Some(arg) => Err(Error::UnusedPositionalFnArgs.at(arg.span)),
    }
}

/// Removes and returns the keyword argument with name `name`.
///
/// Returns an error if the argument does not exist (and `T` is non-nullable) or
/// if the argument is not of type `T`.
pub fn pop_kwarg<T: FromValue>(kwargs: &mut Map, name: &str, key_span: Span) -> Result<T> {
    pop_map_key_generic(kwargs, name).unwrap_or_else(|| {
        let name = name.into();
        let ty = T::hps_ty();
        Err(Error::MissingRequiredNamedParameter { name, ty }.at(key_span))
    })
}

/// Returns an error if there are any more keyword arguments.
pub fn expect_end_of_kwargs(kwargs: Map, caller_span: Span) -> Result<()> {
    if kwargs.is_empty() {
        Ok(())
    } else {
        let names = kwargs.into_iter().map(|(k, v)| (k, v.span)).collect();
        Err(Error::UnusedNamedFnArgs { names }.at(caller_span))
    }
}

/// Removes and returns the value associated to `name` in a map.
///
/// Returns an error if the entry does not exist (and `T` is non-nullable) or if
/// the value is not of type `T`.
pub fn pop_map_key<T: FromValue>(map: &mut Map, map_span: Span, key: &str) -> Result<T> {
    pop_map_key_generic(map, key)
        .unwrap_or_else(|| Err(Error::MissingRequiredMapKey { key: key.into() }.at(map_span)))
}

/// Same as [`pop_map_key()`], but produces a better error message when the map
/// does not have a span. The provided span should be the span of the function
/// that was supposed to add the key to the map.
pub fn pop_map_key_in_special_var<T: FromValue>(
    map: &mut Map,
    fn_span: Span,
    special_var: SpecialVar,
    key: &str,
) -> Result<T> {
    pop_map_key_generic(map, key).unwrap_or_else(|| {
        Err(Error::MissingRequiredMapKeyInSpecialVar {
            key: key.into(),
            special_var,
        }
        .at(fn_span))
    })
}

/// Returns an error if there are any more entries.
pub fn expect_end_of_map(map: Map, map_span: Span) -> Result<()> {
    if map.is_empty() {
        Ok(())
    } else {
        let keys = map.into_iter().map(|(k, v)| (k, v.span)).collect();
        Err(Error::UnusedMapKeys { keys }.at(map_span))
    }
}

fn pop_map_key_generic<T: FromValue>(map: &mut Map, name: &str) -> Option<Result<T>> {
    match map.swap_remove(name) {
        Some(v) => Some(T::from_value(v)),
        None => match T::from_value(Value::NULL) {
            Ok(v) => Some(Ok(v)),
            Err(_) => None,
        },
    }
}

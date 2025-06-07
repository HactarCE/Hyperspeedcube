use crate::{Error, FromValue, Map, Result, Span, Value};

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
    match kwargs.swap_remove(name) {
        Some(v) => T::from_value(v),
        None => T::from_value(Value::NULL).map_err(|_| {
            Error::MissingRequiredNamedParameter {
                name: name.into(),
                ty: T::hps_ty(),
            }
            .at(key_span)
        }),
    }
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

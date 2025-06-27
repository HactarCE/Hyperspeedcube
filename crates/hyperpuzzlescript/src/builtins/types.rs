//! Built-in types and the type operators `|` and `?`

use crate::{Builtins, Result, Type};

/// Adds the built-in types and operators.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    for ty in [
        Type::Any,
        Type::Null,
        Type::Bool,
        Type::Num,
        Type::Str,
        Type::List(None),
        Type::Map,
        Type::Fn,
        Type::Type,
        Type::Int,
        Type::Nat,
        Type::EmptyList,
        Type::NonEmptyList(None),
        Type::Vec,
        Type::EuclidPoint,
        Type::EuclidTransform,
        Type::EuclidPlane,
        Type::EuclidBlade,
        Type::Cga2dBlade1,
        Type::Cga2dBlade2,
        Type::Cga2dBlade3,
        Type::Cga2dAntiscalar,
    ] {
        builtins.set(ty.to_string(), ty)?;
    }

    builtins.set_fns(hps_fns![
        ("|", |_, a: Type, b: Type| -> Type { a | b }),
        ("?", |_, t: Type| -> Type { t.optional() })
    ])
}

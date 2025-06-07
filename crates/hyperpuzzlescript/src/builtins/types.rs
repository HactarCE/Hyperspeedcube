use crate::{Result, Scope, Type, ValueData};

pub fn define_in(scope: &Scope) -> Result<()> {
    for ty in [
        Type::Any,
        Type::Null,
        Type::Bool,
        Type::Num,
        Type::Str,
        Type::List(None),
        Type::Map,
        Type::Fn,
        Type::Int,
        Type::Nat,
        Type::EmptyList,
        Type::NonEmptyList(None),
        Type::Vec,
        Type::Type,
    ] {
        scope.set(ty.to_string(), ValueData::Type(ty).at(crate::BUILTIN_SPAN));
    }

    scope.register_builtin_functions(hps_fns![
        ("|", |_, a: Type, b: Type| -> Type { Type::unify(a, b) }),
        ("?", |_, t: Type| -> Type { t.optional() })
    ])
}

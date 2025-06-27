//! N-dimensional Euclidean geometry functionality.

use crate::{Result, Scope, Type, ValueData};

mod plane;
mod point;
mod transform;

/// Adds the built-in operators and functions to the scope inside the namespace
/// `euclid`.
pub fn define_in(scope: &Scope) -> Result<()> {
    let euclid_scope = Scope::new();

    plane::define_in(&euclid_scope)?;
    point::define_in(&euclid_scope)?;
    transform::define_in(&euclid_scope)?;

    for (name, ty) in [
        ("Point", Type::EuclidPoint),
        ("Transform", Type::EuclidTransform),
        ("Plane", Type::EuclidPlane),
        ("Region", Type::EuclidRegion),
        ("Blade", Type::EuclidBlade),
    ] {
        euclid_scope.set(name, ValueData::Type(ty).at(crate::BUILTIN_SPAN));
    }

    let euclid = ValueData::from(&*euclid_scope).at(crate::BUILTIN_SPAN);
    scope.set("euclid", euclid);
    Ok(())
}

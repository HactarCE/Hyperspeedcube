use crate::{Result, Scope, ValueData};

mod plane;
mod point;

pub fn define_in(scope: &Scope) -> Result<()> {
    let euclid_scope = Scope::new();
    plane::define_in(&euclid_scope)?;
    point::define_in(&euclid_scope)?;
    let euclid = ValueData::from(&*euclid_scope).at(crate::BUILTIN_SPAN);
    scope.set("euclid", euclid);
    Ok(())
}

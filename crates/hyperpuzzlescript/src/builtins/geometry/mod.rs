use crate::{Result, Scope};

mod euclid;
mod vec;

pub fn define_in(scope: &Scope) -> Result<()> {
    euclid::define_in(scope)?;
    vec::define_in(scope)?;
    Ok(())
}

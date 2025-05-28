use crate::{Result, Scope};

mod vec;

pub fn define_in(scope: &Scope) -> Result<()> {
    vec::define_in(scope)?;
    Ok(())
}

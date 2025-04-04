use super::*;

pub(super) fn try_to_float(v: &Dynamic) -> Result<f64> {
    v.as_float()
        .or_else(|_| Ok(v.as_int()? as f64))
        .map_err(|ty: &str| format!("expected number, got {ty}").into())
}

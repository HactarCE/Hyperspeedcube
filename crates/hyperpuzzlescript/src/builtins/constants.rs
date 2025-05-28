use std::f64::consts::{PI, TAU};

use crate::{BUILTIN_SPAN, Result, Scope, ValueData};

const PHI: f64 = 1.618_033_988_749_895_f64;

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.set("pi", ValueData::Num(PI).at(BUILTIN_SPAN));
    scope.set("π", ValueData::Num(PI).at(BUILTIN_SPAN));

    scope.set("tau", ValueData::Num(TAU).at(BUILTIN_SPAN));
    scope.set("τ", ValueData::Num(TAU).at(BUILTIN_SPAN));

    scope.set("phi", ValueData::Num(PHI).at(BUILTIN_SPAN));
    scope.set("φ", ValueData::Num(PHI).at(BUILTIN_SPAN));

    scope.set("deg", ValueData::Num(1.0_f64.to_radians()).at(BUILTIN_SPAN));

    scope.set("inf", ValueData::Num(f64::INFINITY).at(BUILTIN_SPAN));
    scope.set("∞", ValueData::Num(f64::INFINITY).at(BUILTIN_SPAN));

    Ok(())
}

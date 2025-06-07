use ecow::eco_format;
use hypermath::{Vector, is_approx_nonzero, vector};

use crate::{Error, Map, Num, Result, Scope, Span, Type, Value, ValueData};

mod vec;

pub fn define_in(scope: &Scope) -> Result<()> {
    vec::define_in(scope)?;
    Ok(())
}

pub(super) fn construct_vec(span: Span, args: &[Value], kwargs: Map) -> Result<Vector> {
    match args {
        [] => {
            unpack_kwargs!(
                kwargs,
                x: Num = 0.0,
                y: Num = 0.0,
                z: Num = 0.0,
                w: Num = 0.0,
                v: Num = 0.0,
                u: Num = 0.0,
                t: Num = 0.0,
            );
            let mut ret = vector![];
            for (i, n) in [x, y, z, w, v, u, t].iter().enumerate() {
                if is_approx_nonzero(n) {
                    ret.resize_and_set(i as u8, *n);
                }
            }
            Ok(ret)
        }

        [arg] => match &arg.data {
            ValueData::Num(n) => Ok(vector![*n]),
            ValueData::Vec(v) => Ok(v.clone()),
            ValueData::EuclidPoint(p) => Ok(p.0.clone()),
            _ => Err(arg.type_error(Type::from_iter([Type::Num, Type::Vec, Type::EuclidPoint]))),
        },

        _ if args.len() > hypermath::MAX_NDIM as usize => Err(Error::User(eco_format!(
            "vector too long (max is {})",
            hypermath::MAX_NDIM,
        ))
        .at(span)),

        _ => args.iter().map(|arg| arg.ref_to::<f64>()).collect(),
    }
}

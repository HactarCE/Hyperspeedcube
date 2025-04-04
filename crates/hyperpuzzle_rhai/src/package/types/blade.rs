// TODO: clean this up or delete it

use hypermath::pga::*;
use hypermath::prelude::*;

use super::*;

fn try_collect_to_vector(values: &[Dynamic]) -> Result<Blade> {
    values
        .iter()
        .map(util::try_to_float)
        .collect::<Result<Vector>>()
        .map(Blade::from_vector)
}

fn try_collect_to_point(values: &[Dynamic]) -> Result<Blade> {
    values
        .iter()
        .map(util::try_to_float)
        .collect::<Result<Vector>>()
        .map(Blade::from_point)
}

fn try_blade_as_vector_or_point(blade: &Blade) -> Result<Vector> {
    Ok(blade
        .to_point()
        .or_else(|| blade.to_vector())
        .ok_or("expected vector or point")?)
}

fn set_vector_or_point_coordinate(blade: &mut Blade, i: u8, new_value: f64) -> Result<()> {
    *blade = blade.to_ndim_at_least(i + 1);
    let mult = blade
        .get(Axes::E0)
        .and_then(|&e0| hypermath::util::try_recip(e0))
        .unwrap_or(1.0);
    *blade
        .get_mut(Axes::euclidean(i))
        .ok_or("expected a vector or point")? = new_value * mult;
    Ok(())
}

fn blades_eq(a: &Blade, b: &Blade) -> bool {
    match (a.weight_is_zero(), b.weight_is_zero()) {
        (true, true) => hypermath::approx_eq(a, b),
        (false, false) => a * b.weight_norm() == b * a.weight_norm(),
        _ => false,
    }
}

pub fn register(module: &mut Module) {
    module.combine_flatten(exported_module!(rhai_mod));

    // Vector constructors
    FuncRegistration::new("vec").set_into_module(module, |x, y| try_collect_to_vector(&[x, y]));
    FuncRegistration::new("vec")
        .set_into_module(module, |x, y, z| try_collect_to_vector(&[x, y, z]));
    FuncRegistration::new("vec")
        .set_into_module(module, |x, y, z, w| try_collect_to_vector(&[x, y, z, w]));
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v| {
        try_collect_to_vector(&[x, y, z, w, v])
    });
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v, u| {
        try_collect_to_vector(&[x, y, z, w, v, u])
    });
    FuncRegistration::new("vec").set_into_module(module, |x, y, z, w, v, u, t| {
        try_collect_to_vector(&[x, y, z, w, v, u, t])
    });

    // Point constructors
    FuncRegistration::new("point").set_into_module(module, |x, y| try_collect_to_point(&[x, y]));
    FuncRegistration::new("point")
        .set_into_module(module, |x, y, z| try_collect_to_point(&[x, y, z]));
    FuncRegistration::new("point")
        .set_into_module(module, |x, y, z, w| try_collect_to_point(&[x, y, z, w]));
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v| {
        try_collect_to_point(&[x, y, z, w, v])
    });
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v, u| {
        try_collect_to_point(&[x, y, z, w, v, u])
    });
    FuncRegistration::new("point").set_into_module(module, |x, y, z, w, v, u, t| {
        try_collect_to_point(&[x, y, z, w, v, u, t])
    });

    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate() {
        let name = c.to_ascii_lowercase().to_string();

        let getter = FuncRegistration::new_getter(&name);
        getter.set_into_module(module, move |blade: &mut Blade| -> Result<f64> {
            Ok(try_blade_as_vector_or_point(blade)?.get(i as u8))
        });

        let setter = FuncRegistration::new_setter(&name);
        setter.set_into_module(module, move |blade: &mut Blade, new_value: f64| {
            set_vector_or_point_coordinate(blade, i as u8, new_value)
        });
    }
}

#[export_module]
mod rhai_mod {
    use hypermath::VectorRef;

    pub fn to_string(b: &mut Blade) -> String {
        if let Some(point) = b.to_point() {
            format!("point{point}")
        } else if let Some(vector) = b.to_vector() {
            format!("vec{vector}")
        } else {
            b.to_string()
        }
    }
    pub fn to_debug(b: &mut Blade) -> String {
        b.to_string()
    }

    // Functions
    #[rhai_fn(return_raw)]
    pub fn cross(u: Blade, v: Blade) -> Result<Blade> {
        Ok(Blade::from_vector(Vector::cross_product_3d(
            &u.to_vector().ok_or("expected vector")?,
            &v.to_vector().ok_or("expected vector")?,
        )))
    }
    pub fn dot(u: Blade, v: Blade) -> f64 {
        Blade::dot(&u, &v).unwrap_or(0.0)
    }

    // Operators
    #[rhai_fn(return_raw, name = "+")]
    pub fn add(u: Blade, v: Blade) -> Result<Blade> {
        let g1 = u.grade();
        let g2 = v.grade();
        if g1 == g2 {
            Ok(u.weight_normalize() + v.weight_normalize())
        } else {
            Err(format!("cannot add blades with grades {g1} and {g2}").into())
        }
    }
    #[rhai_fn(return_raw, name = "-")]
    pub fn sub(u: Blade, v: Blade) -> Result<Blade> {
        let g1 = u.grade();
        let g2 = v.grade();
        if g1 == g2 {
            Ok(u.weight_normalize() - v.weight_normalize())
        } else {
            Err(format!("cannot add blades with grades {g1} and {g2}").into())
        }
    }
    #[rhai_fn(return_raw, name = "*")]
    pub fn mul_vec_float(vector: Blade, scalar: Dynamic) -> Result<Blade> {
        Ok(vector * util::try_to_float(&scalar)?)
    }
    #[rhai_fn(return_raw, name = "*")]
    pub fn mul_float_vec(scalar: Dynamic, vector: Blade) -> Result<Blade> {
        Ok(vector * util::try_to_float(&scalar)?)
    }
    #[rhai_fn(return_raw, name = "/")]
    pub fn div_vec_float(vector: Blade, scalar: Dynamic) -> Result<Blade> {
        Ok(vector * util::try_to_float(&scalar)?)
    }
    #[rhai_fn(name = "==")]
    pub fn approx_eq_vec_vec(u: Blade, v: Blade) -> bool {
        blades_eq(&u, &v)
    }
    #[rhai_fn(name = "!=")]
    pub fn approx_neq_vec_vec(u: Blade, v: Blade) -> bool {
        !blades_eq(&u, &v)
    }

    // Vector constructor
    #[rhai_fn(return_raw, name = "vec")]
    pub fn vec1(x: Dynamic) -> Result<Blade> {
        x.as_array_ref()
            .map(|array| {
                array
                    .iter()
                    .map(util::try_to_float)
                    .collect::<Result<Vector>>()
                    .map(Blade::from_vector)
            })
            .unwrap_or_else(|_| try_collect_to_vector(&[x]))
    }

    // Point constructor
    #[rhai_fn(return_raw, name = "point")]
    pub fn point1(x: Dynamic) -> Result<Blade> {
        x.as_array_ref()
            .map(|array| {
                array
                    .iter()
                    .map(util::try_to_float)
                    .collect::<Result<Vector>>()
                    .map(Blade::from_point)
            })
            .unwrap_or_else(|_| try_collect_to_vector(&[x]))
    }
}

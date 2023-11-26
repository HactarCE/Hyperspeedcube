use hypermath::prelude::*;
use hypershape::prelude::*;

use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "manifold", convert_str = "manifold or multivector"]
    pub struct LuaManifold(ManifoldRef) = |lua| {
        <_>(LuaMultivector(m)) => Ok(LuaManifold::construct_from_multivector(lua, m)?),
        <LuaTable<'_>>(t) => Ok(LuaManifold::construct_from_table(lua, t)?),
    }
}

impl LuaUserData for LuaNamedUserData<ManifoldRef> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("ndim", |lua, Self(this), ()| {
            LuaSpace::with(lua, |space| Ok(space.ndim_of(this)))
        });
    }
}

impl LuaManifold {
    fn construct_from_multivector(lua: LuaContext<'_>, m: Multivector) -> LuaResult<ManifoldRef> {
        LuaSpace::with(lua, |space| {
            space
                .add_manifold(Blade::try_from(m).map_err(LuaError::external)?)
                .map_err(LuaError::external)
        })
    }

    fn construct_from_table<'lua>(
        lua: LuaContext<'_>,
        t: LuaTable<'lua>,
    ) -> LuaResult<ManifoldRef> {
        Ok(Self::construct_plane_or_sphere(t)?.to_manifold(lua)?.0)
    }

    fn construct_plane_or_sphere<'lua>(t: LuaTable<'lua>) -> LuaResult<LuaPlaneOrSphere> {
        let arg_count = t.clone().pairs::<LuaValue<'_>, LuaValue<'_>>().count();

        let ensure_args_len = |n| {
            if n == arg_count {
                Ok(())
            } else {
                Err(LuaError::external(
                    "bad manifold construction; too many keys",
                ))
            }
        };

        if t.contains_key(1)? {
            // anonymous vector + ...
            let LuaVector(v) = t.get(1)?;
            if t.contains_key("distance")? {
                // anonymous vector + distance
                let distance = t.get("distance")?;
                ensure_args_len(2)?;
                Ok(LuaPlaneOrSphere::Plane {
                    normal: v,
                    distance,
                })
            } else {
                // anonymous vector
                ensure_args_len(1)?;
                LuaPlaneOrSphere::plane_from_pole(v)
            }
        } else if t.contains_key("pole")? {
            // pole
            let LuaVector(pole) = t.get("pole")?;
            ensure_args_len(1)?;
            LuaPlaneOrSphere::plane_from_pole(pole)
        } else if t.contains_key("normal")? {
            // normal + ...
            let LuaVector(normal) = t.get("normal")?;
            if t.contains_key("point")? {
                // normal + point
                ensure_args_len(2)?;
                let LuaVector(point) = t.get("point")?;
                LuaPlaneOrSphere::plane_from_point_and_normal(point, normal)
            } else if t.contains_key("distance")? {
                // normal + distance
                ensure_args_len(2)?;
                let distance = t.get("distance")?;
                Ok(LuaPlaneOrSphere::Plane { normal, distance })
            } else {
                // normal
                ensure_args_len(1)?;
                Ok(LuaPlaneOrSphere::Plane {
                    normal,
                    distance: 0.0,
                })
            }
        } else if t.contains_key("radius")? {
            // radius + ...
            let radius = t.get("radius")?;
            if t.contains_key("center")? {
                // radius + center
                let LuaVector(center) = t.get("center")?;
                ensure_args_len(2)?;
                Ok(LuaPlaneOrSphere::Sphere { center, radius })
            } else {
                // radius
                ensure_args_len(1)?;
                Ok(LuaPlaneOrSphere::Sphere {
                    center: Vector::EMPTY,
                    radius,
                })
            }
        } else {
            Err(LuaError::external("bad manifold construction; expected keys such as `distance`, `center`, `normal`, `point`, `pole`, and `radius`"))
        }
    }

    pub fn construct_sphere<'lua>(
        lua: LuaContext<'lua>,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaManifold> {
        if let Ok(radius) = lua.unpack_multi::<Float>(args.clone()) {
            LuaPlaneOrSphere::Sphere {
                center: Vector::EMPTY,
                radius,
            }
            .to_manifold(lua)
        } else {
            let arg: LuaValue<'_> = lua.unpack_multi(args)?;
            let t = lua_convert!(match (lua, &arg, "sphere constructor (number or table)") {
                <LuaTable>(t) => Ok(t),
            })?;

            match Self::construct_plane_or_sphere(t)? {
                m @ LuaPlaneOrSphere::Sphere { .. } => m.to_manifold(lua),
                LuaPlaneOrSphere::Plane { .. } => {
                    return Err(LuaError::external(
                        "expected sphere constructor but got plane constructor",
                    ))
                }
            }
        }
    }

    pub fn construct_plane<'lua>(
        lua: LuaContext<'lua>,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaManifold> {
        if let Ok(LuaConstructVector(v)) = lua.unpack_multi(args.clone()) {
            LuaPlaneOrSphere::plane_from_pole(v)?.to_manifold(lua)
        } else {
            let arg: LuaValue<'_> = lua.unpack_multi(args)?;
            let t = lua_convert!(match (lua, &arg, "plane constructor (vector or table)") {
                <LuaTable>(t) => Ok(t),
            })?;

            match Self::construct_plane_or_sphere(t)? {
                m @ LuaPlaneOrSphere::Plane { .. } => m.to_manifold(lua),
                LuaPlaneOrSphere::Sphere { .. } => {
                    return Err(LuaError::external(
                        "expected plane constructor but got sphere constructor",
                    ));
                }
            }
        }
    }
}

enum LuaPlaneOrSphere {
    Plane { normal: Vector, distance: Float },
    Sphere { center: Vector, radius: Float },
}
impl LuaPlaneOrSphere {
    fn plane_from_pole(pole: Vector) -> LuaResult<Self> {
        let distance = pole.mag2();
        let normal = pole
            .normalize()
            .ok_or_else(|| LuaError::external("plane pole cannot be zero"))?;
        Ok(LuaPlaneOrSphere::Plane { normal, distance })
    }
    fn plane_from_point_and_normal(point: Vector, normal: Vector) -> LuaResult<Self> {
        let normal = normal
            .normalize()
            .ok_or_else(|| LuaError::external("normal vector cannot be zero"))?;
        let distance = point.dot(&normal);
        Ok(LuaPlaneOrSphere::Plane { normal, distance })
    }

    fn to_blade(&self, ndim: u8) -> Blade {
        match self {
            LuaPlaneOrSphere::Plane { normal, distance } => Blade::ipns_plane(normal, *distance),
            LuaPlaneOrSphere::Sphere { center, radius } => Blade::ipns_sphere(center, *radius),
        }
        .ipns_to_opns(ndim)
    }

    fn to_manifold(&self, lua: LuaContext<'_>) -> LuaResult<LuaManifold> {
        LuaSpace::with(lua, |space| {
            match space.add_manifold(self.to_blade(space.ndim())) {
                Ok(m) => Ok(LuaManifold(m)),
                Err(e) => Err(LuaError::external(e)),
            }
        })
    }
}

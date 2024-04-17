use hypermath::prelude::*;

use super::*;

/// Lua wrapper for a set of manifolds.
#[derive(Debug, Clone)]
pub struct LuaManifoldSet(pub Vec<Blade>);

impl<'lua> FromLua<'lua> for LuaManifoldSet {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaManifold(m)) = lua.unpack(value.clone()) {
            Ok(Self(vec![m]))
        } else if let Ok(LuaSequence(manifolds)) = lua.unpack(value.clone()) {
            Ok(Self(
                manifolds.into_iter().map(|LuaManifold(m)| m).collect(),
            ))
        } else {
            lua_convert_err(&value, "manifold or table of manifolds")
        }
    }
}

/// Lua conversion wrapper for a manifold.
///
/// This is not actually a Lua type since it does not implement [`LuaUserData`].
#[derive(Debug, Clone)]
pub struct LuaManifold(pub Blade);

impl<'lua> FromLua<'lua> for LuaManifold {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(m) = cast_userdata(lua, &value) {
            Ok(m)
        } else if let Ok(LuaVector(v)) = cast_userdata(lua, &value) {
            LuaManifoldParams::plane_from_pole(v)?.to_manifold(lua)
        } else if let Ok(LuaMultivector(m)) = lua.unpack(value.clone()) {
            Blade::try_from(m).map(Self).into_lua_err()
        } else if let LuaValue::Table(t) = value {
            Self::construct_plane_or_sphere(t)?.to_manifold(lua)
        } else {
            lua_convert_err(&value, "manifold, vector, multivector, or table")
        }
    }
}

impl LuaUserData for LuaManifold {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("manifold"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(m), ()| {
            Ok(format!("manifold({m})"))
        });

        methods.add_meta_method(LuaMetaMethod::Unm, |_lua, Self(m), ()| Ok(Self(-m)));

        methods.add_method("ndim", |_lua, Self(m), ()| Ok(m.cga_opns_ndim()));
    }
}

impl LuaManifold {
    /// Constructs a plane or sphere from a table of values.
    fn construct_plane_or_sphere(t: LuaTable<'_>) -> LuaResult<LuaManifoldParams> {
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
                Ok(LuaManifoldParams::Plane {
                    normal: v,
                    distance,
                })
            } else {
                // anonymous vector
                ensure_args_len(1)?;
                LuaManifoldParams::plane_from_pole(v)
            }
        } else if t.contains_key("pole")? {
            // pole
            let LuaVector(pole) = t.get("pole")?;
            ensure_args_len(1)?;
            LuaManifoldParams::plane_from_pole(pole)
        } else if t.contains_key("normal")? {
            // normal + ...
            let LuaVector(normal) = t.get("normal")?;
            if t.contains_key("point")? {
                // normal + point
                ensure_args_len(2)?;
                let LuaVector(point) = t.get("point")?;
                LuaManifoldParams::plane_from_point_and_normal(point, normal)
            } else if t.contains_key("distance")? {
                // normal + distance
                ensure_args_len(2)?;
                let distance = t.get("distance")?;
                Ok(LuaManifoldParams::Plane { normal, distance })
            } else {
                // normal
                ensure_args_len(1)?;
                Ok(LuaManifoldParams::Plane {
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
                Ok(LuaManifoldParams::Sphere { center, radius })
            } else {
                // radius
                ensure_args_len(1)?;
                Ok(LuaManifoldParams::Sphere {
                    center: Vector::EMPTY,
                    radius,
                })
            }
        } else {
            Err(LuaError::external(
                "bad manifold construction; \
                 expected keys such as `distance`, `center`, \
                 `normal`, `point`, `pole`, and `radius`",
            ))
        }
    }

    /// Constructs a sphere from a single value, which may be a number or a
    /// table.
    pub fn construct_sphere<'lua>(lua: &'lua Lua, arg: LuaValue<'lua>) -> LuaResult<LuaManifold> {
        if let Ok(LuaNumberNoConvert(radius)) = lua.unpack(arg.clone()) {
            LuaManifoldParams::Sphere {
                center: Vector::EMPTY,
                radius: radius as Float,
            }
            .to_manifold(lua)
        } else if let Ok(LuaValue::Table(t)) = lua.unpack(arg.clone()) {
            match Self::construct_plane_or_sphere(t)? {
                m @ LuaManifoldParams::Sphere { .. } => m.to_manifold(lua),
                LuaManifoldParams::Plane { .. } => Err(LuaError::external(
                    "expected sphere constructor but got plane constructor",
                )),
            }
        } else {
            lua_convert_err(&arg, "sphere constructor (number or table)")
        }
    }

    /// Constructs a plane from a multivalue, which may be a vector, a series of
    /// numbers representing a vector, or a table.
    pub fn construct_plane<'lua>(
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaManifold> {
        if args.len() > 1 {
            let LuaVectorFromMultiValue(v) = lua.unpack_multi(args)?;
            LuaManifoldParams::plane_from_pole(v)?.to_manifold(lua)
        } else if let Ok(LuaVectorFromMultiValue(v)) = lua.unpack_multi(args.clone()) {
            LuaManifoldParams::plane_from_pole(v)?.to_manifold(lua)
        } else {
            let value: LuaValue<'_> = lua.unpack_multi(args)?;
            match value {
                LuaValue::Table(t) => match Self::construct_plane_or_sphere(t)? {
                    m @ LuaManifoldParams::Plane { .. } => m.to_manifold(lua),
                    LuaManifoldParams::Sphere { .. } => Err(LuaError::external(
                        "expected plane constructor but got sphere constructor",
                    )),
                },
                v => lua_convert_err(&v, "plane constructor (vector or table)"),
            }
        }
    }

    /// Transforms the manifold by `t`.
    pub fn transform(&self, t: &Isometry) -> Self {
        Self(t.transform_blade(&self.0))
    }
}

/// Type representing a Lua description of a manifold.
enum LuaManifoldParams {
    Plane { normal: Vector, distance: Float },
    Sphere { center: Vector, radius: Float },
}
impl LuaManifoldParams {
    /// Constructs a plane from a pole, which is a vector from the origin to the
    /// nearest point on the plane. The pole is always perpendicular to the
    /// plane.
    fn plane_from_pole(pole: Vector) -> LuaResult<Self> {
        let distance = pole.mag();
        let normal = pole
            .normalize()
            .ok_or_else(|| LuaError::external("plane pole cannot be zero"))?;
        Ok(LuaManifoldParams::Plane { normal, distance })
    }
    /// Constructs a plane from a point and a normal vector.
    fn plane_from_point_and_normal(point: Vector, normal: Vector) -> LuaResult<Self> {
        let normal = normal
            .normalize()
            .ok_or_else(|| LuaError::external("normal vector cannot be zero"))?;
        let distance = point.dot(&normal);
        Ok(LuaManifoldParams::Plane { normal, distance })
    }

    /// Returns a blade representing the manifoild.
    fn to_blade(&self, space_ndim: u8) -> LuaResult<Blade> {
        Ok(match self {
            LuaManifoldParams::Plane { normal, distance } => {
                let normal_ndim = normal.ndim();
                if normal_ndim > space_ndim {
                    return Err(LuaError::external(format!(
                        "plane normal has {normal_ndim} dimensions \
                         but space is only {space_ndim}D",
                    )));
                }
                Blade::ipns_plane(normal, *distance)
            }
            LuaManifoldParams::Sphere { center, radius } => {
                let center_ndim = center.ndim();
                if center_ndim > space_ndim {
                    return Err(LuaError::external(format!(
                        "sphere center has {center_ndim} dimensions \
                         but space is only {space_ndim}D",
                    )));
                }
                Blade::ipns_sphere(center, *radius)
            }
        }
        .ipns_to_opns(space_ndim))
    }

    /// Returns a Lua value representing the manifold.
    fn to_manifold(&self, lua: &Lua) -> LuaResult<LuaManifold> {
        LuaSpace::with(lua, |space| self.to_blade(space.ndim()).map(LuaManifold))
    }
}

use std::sync::Arc;

use hypermath::prelude::*;
use hypershape::prelude::*;
use parking_lot::Mutex;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaManifoldSet(pub ManifoldSet);

impl<'lua> FromLua<'lua> for LuaManifoldSet {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaManifold { manifold, .. }) = lua.unpack(value.clone()) {
            Ok(Self(ManifoldSet::from_iter([manifold])))
        } else if let Ok(LuaSequence(manifolds)) = lua.unpack(value.clone()) {
            Ok(Self(
                manifolds
                    .into_iter()
                    .map(|LuaManifold { manifold, .. }| manifold)
                    .collect(),
            ))
        } else {
            lua_convert_err(&value, "manifold or table of manifolds")
        }
    }
}

#[derive(Debug, Clone)]
pub struct LuaManifold {
    pub manifold: ManifoldRef,
    pub space: Arc<Mutex<Space>>,
}

impl<'lua> FromLua<'lua> for LuaManifold {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(m) = cast_userdata(lua, &value) {
            Ok(m)
        } else if let Ok(LuaVector(v)) = cast_userdata(lua, &value) {
            LuaPlaneOrSphere::plane_from_pole(v)?.to_manifold(lua)
        } else if let Ok(LuaMultivector(m)) = lua.unpack(value.clone()) {
            let manifold = LuaSpace::with(lua, |space| space.add_manifold(Blade::try_from(m)?))?;
            let LuaSpace(space) = LuaSpace::get(lua)?;
            Ok(LuaManifold { manifold, space })
        } else if let LuaValue::Table(t) = value {
            Self::construct_plane_or_sphere(t)?.to_manifold(lua)
        } else {
            lua_convert_err(&value, "manifold, vector, multivector, or table")
        }
    }
}

impl LuaManifold {
    fn construct_plane_or_sphere(t: LuaTable<'_>) -> LuaResult<LuaPlaneOrSphere> {
        let distance: Option<Float>;

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
            Err(LuaError::external(
                "bad manifold construction; \
                 expected keys such as `distance`, `center`, \
                 `normal`, `point`, `pole`, and `radius`",
            ))
        }
    }

    pub fn construct_sphere<'lua>(lua: &'lua Lua, arg: LuaValue<'lua>) -> LuaResult<LuaManifold> {
        if let Ok(LuaNumberNoConvert(radius)) = lua.unpack(arg.clone()) {
            LuaPlaneOrSphere::Sphere {
                center: Vector::EMPTY,
                radius: radius as Float,
            }
            .to_manifold(lua)
        } else if let Ok(LuaValue::Table(t)) = lua.unpack(arg.clone()) {
            match Self::construct_plane_or_sphere(t)? {
                m @ LuaPlaneOrSphere::Sphere { .. } => m.to_manifold(lua),
                LuaPlaneOrSphere::Plane { .. } => Err(LuaError::external(
                    "expected sphere constructor but got plane constructor",
                )),
            }
        } else {
            lua_convert_err(&arg, "sphere constructor (number or table)")
        }
    }

    pub fn construct_plane<'lua>(
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaManifold> {
        if args.len() > 1 {
            let LuaVectorFromMultiValue(v) = lua.unpack_multi(args)?;
            LuaPlaneOrSphere::plane_from_pole(v)?.to_manifold(lua)
        } else if let Ok(LuaVectorFromMultiValue(v)) = lua.unpack_multi(args.clone()) {
            LuaPlaneOrSphere::plane_from_pole(v)?.to_manifold(lua)
        } else {
            let value: LuaValue<'_> = lua.unpack_multi(args)?;
            match value {
                LuaValue::Table(t) => match Self::construct_plane_or_sphere(t)? {
                    m @ LuaPlaneOrSphere::Plane { .. } => m.to_manifold(lua),
                    LuaPlaneOrSphere::Sphere { .. } => Err(LuaError::external(
                        "expected plane constructor but got sphere constructor",
                    )),
                },
                v => lua_convert_err(&v, "plane constructor (vector or table)"),
            }
        }
    }

    pub fn transform(&self, t: &Isometry) -> LuaResult<Self> {
        let mut space = self.space.lock();
        let transformed_blade = t.transform_blade(&space.blade_of(self.manifold));
        match space.add_manifold(transformed_blade) {
            Ok(manifold) => Ok(Self {
                manifold,
                space: self.space.clone(),
            }),
            Err(e) => Err(e.into_lua_err()),
        }
    }
}

impl LuaUserData for LuaManifold {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("manifold"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("manifold({})", this.manifold))
        });

        methods.add_meta_method(LuaMetaMethod::Unm, |_lua, Self { manifold, space }, ()| {
            Ok(Self {
                manifold: -*manifold,
                space: Arc::clone(space),
            })
        });

        methods.add_method("ndim", |lua, Self { manifold, .. }, ()| {
            LuaSpace::with(lua, |space| LuaResult::Ok(space.ndim_of(manifold)))
        });
    }
}

enum LuaPlaneOrSphere {
    Plane { normal: Vector, distance: Float },
    Sphere { center: Vector, radius: Float },
}
impl LuaPlaneOrSphere {
    fn plane_from_pole(pole: Vector) -> LuaResult<Self> {
        let distance = pole.mag();
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

    fn to_blade(&self, space_ndim: u8) -> LuaResult<Blade> {
        Ok(match self {
            LuaPlaneOrSphere::Plane { normal, distance } => {
                let normal_ndim = normal.ndim();
                if normal_ndim > space_ndim {
                    return Err(LuaError::external(format!(
                        "plane normal has {normal_ndim} dimensions \
                         but space is only {space_ndim}D",
                    )));
                }
                Blade::ipns_plane(normal, *distance)
            }
            LuaPlaneOrSphere::Sphere { center, radius } => {
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

    fn to_manifold(&self, lua: &Lua) -> LuaResult<LuaManifold> {
        LuaSpace::with(lua, |space| {
            match space.add_manifold(self.to_blade(space.ndim())?) {
                Ok(manifold) => {
                    let LuaSpace(space) = LuaSpace::get(lua)?;
                    Ok(LuaManifold { manifold, space })
                }
                Err(e) => Err(LuaError::external(e)),
            }
        })
    }
}

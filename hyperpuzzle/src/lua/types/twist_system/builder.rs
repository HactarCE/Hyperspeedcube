use std::borrow::Cow;
use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::builder::{NamingScheme, PuzzleBuilder, TwistBuilder};
use crate::lua::lua_warn_fn;
use crate::puzzle::Twist;

/// Lua handle to a twist system under construction.
#[derive(Debug, Clone)]
pub struct LuaTwistSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaTwistSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twistsystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            let len = this.lock().twists.len();
            Ok(format!("twistsystem(len={len})"))
        });

        LuaIdDatabase::<Twist>::add_db_metamethods(methods, |Self(puz)| puz.lock());
        LuaNamedIdDatabase::<Twist>::add_named_db_methods(methods, |Self(puz)| puz.lock());

        methods.add_method("add", |lua, this, (axis, transform, data)| {
            this.add(lua, axis, transform, data)
        });
    }
}

impl<'lua> LuaIdDatabase<'lua, Twist> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "twist";
    const ELEMENT_NAME_PLURAL: &'static str = "twists";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Twist> {
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .unwrap_or_else(|| lua_convert_err(&value, "axis, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.twists.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Twist]> {
        Cow::Owned(self.twists.alphabetized())
    }
}

impl<'lua> LuaNamedIdDatabase<'lua, Twist> for PuzzleBuilder {
    fn names(&self) -> &NamingScheme<Twist> {
        &self.twists.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Twist> {
        &mut self.twists.names
    }
}

impl LuaTwistSystem {
    /// Adds a new twist.
    fn add<'lua>(
        &self,
        lua: &'lua Lua,
        axis: LuaAxis,
        transform: LuaTransform,
        data: Option<LuaTable<'lua>>,
    ) -> LuaResult<Option<LuaTwist>> {
        let multipliers: Option<bool>;
        let inverse: Option<bool>;
        let prefix: Option<String>;
        let name: Option<String>;
        let suffix: Option<String>;
        let inv_name: Option<String>;
        let inv_suffix: Option<String>;
        let name_fn: Option<LuaFunction<'_>>;
        let qtm: Option<usize>;
        let gizmo_pole_distance: Option<f32>;
        if let Some(data_table) = data {
            unpack_table!(lua.unpack(data_table {
                multipliers,
                inverse,
                prefix,
                name,
                suffix,
                inv_name,
                inv_suffix,
                name_fn,
                qtm,
                gizmo_pole_distance,
            }));
        } else {
            // These are reasonable defaults, especially for 3D.
            multipliers = Some(true);
            inverse = Some(true);
            prefix = axis.name();
            name = None;
            suffix = None;
            inv_name = None;
            inv_suffix = None;
            name_fn = None;
            qtm = None;
            gizmo_pole_distance = None;
        }

        let do_naming = prefix.is_some()
            || name.is_some()
            || suffix.is_some()
            || inv_name.is_some()
            || inv_suffix.is_some()
            || name_fn.is_some();

        let inverse = inverse.unwrap_or(false);
        let multipliers = multipliers.unwrap_or(false);

        let suffix = suffix.unwrap_or_default();
        let inv_suffix = inv_suffix.unwrap_or_else(|| match &inv_name {
            Some(_) => suffix.clone(),
            None => "'".to_string(),
        });

        if name_fn.is_some() && (name.is_some() || inv_name.is_some()) {
            return Err(LuaError::external(
                "when `name_fn` is specified, `name` and `inv_name` must not be specified",
            ));
        }

        let prefix = prefix.unwrap_or_default();
        let name = name.unwrap_or_default();
        let inv_name = inv_name.unwrap_or_else(|| name.clone());

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            lua.warning("twist has QTM value less than 1", false);
        }

        let mut puz = self.0.lock();

        if gizmo_pole_distance.is_some() && puz.ndim() != 4 {
            return Err(LuaError::external("twist gizmo is only supported in 4D"));
        }

        let twists = &mut puz.twists;
        let axis = axis.id;

        let base_transform = transform.0;

        let get_name = |i: i32| {
            if let Some(name_fn) = &name_fn {
                name_fn.call(i)
            } else if do_naming {
                match i {
                    1 => Ok(format!("{prefix}{name}{suffix}")),
                    -1 => Ok(format!("{prefix}{inv_name}{inv_suffix}")),
                    2.. => Ok(format!("{prefix}{name}{i}{suffix}")),
                    ..=-2 => Ok(format!("{prefix}{inv_name}{}{inv_suffix}", -i)),
                    0 => Err(LuaError::external("bad twist multiplier")),
                }
            } else {
                Ok(String::new())
            }
        };

        let transform = base_transform.clone();
        let Some(first_twist_id) = twists
            .add_named(
                TwistBuilder {
                    axis,
                    transform,
                    qtm,
                    gizmo_pole_distance,
                },
                get_name(1)?,
                lua_warn_fn(lua),
            )
            .into_lua_err()?
        else {
            return Ok(None);
        };
        if inverse {
            let transform = base_transform.reverse();
            twists
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm,
                        gizmo_pole_distance,
                    },
                    get_name(-1)?,
                    lua_warn_fn(lua),
                )
                .into_lua_err()?;
        }

        let mut previous_transform = base_transform.clone();
        for i in 2.. {
            if !multipliers {
                break;
            }

            // Check whether we've exceeded the max repeat count.
            if i > crate::MAX_TWIST_REPEAT as i32 {
                return Err(LuaError::external(format!(
                    "twist transform takes too long to repeat! exceeded maximum of {}",
                    crate::MAX_TWIST_REPEAT,
                )));
            }

            let transform = &previous_transform * &base_transform;

            // Check whether we've reached the inverse.
            if inverse {
                if previous_transform.is_self_reverse()
                    || transform.is_equivalent_to(&previous_transform.reverse())
                {
                    break;
                }
            } else {
                if transform.is_ident() {
                    break;
                }
            }
            previous_transform = transform.clone();

            twists
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm: qtm * i as usize,
                        gizmo_pole_distance: None, // no gizmo for multiples
                    },
                    get_name(i)?,
                    lua_warn_fn(lua),
                )
                .into_lua_err()?;

            if inverse {
                let transform = previous_transform.reverse();
                twists
                    .add_named(
                        TwistBuilder {
                            axis,
                            transform,
                            qtm: qtm * i as usize,
                            gizmo_pole_distance: None, // no gizmo for multiples
                        },
                        get_name(-i)?,
                        lua_warn_fn(lua),
                    )
                    .into_lua_err()?;
            }
        }

        Ok(Some(puz.wrap_id(first_twist_id)))
    }
}

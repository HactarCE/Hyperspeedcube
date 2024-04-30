use std::borrow::Cow;
use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::builder::{NamingScheme, TwistBuilder, TwistSystemBuilder};
use crate::lua::lua_warn_fn;
use crate::puzzle::Twist;

/// Lua handle to a twist system under construction.
#[derive(Debug, Clone)]
pub struct LuaTwistSystem(pub Arc<Mutex<TwistSystemBuilder>>);

impl LuaUserData for LuaTwistSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twistsystem"));

        fields.add_field_method_get("axes", |_lua, this| {
            Ok(LuaAxisSystem(Arc::clone(&this.0.lock().axes)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            let this = this.lock();
            let len = this.len();
            if let Some(id) = &this.id {
                Ok(format!("twistsystem({id:?}, len={len})"))
            } else {
                Ok(format!("twistsystem(len={len})"))
            }
        });

        TwistSystemBuilder::add_db_metamethods(methods, |Self(shape)| shape.lock());
        TwistSystemBuilder::add_named_db_methods(methods, |Self(shape)| shape.lock());

        methods.add_method("add", |lua, this, data| this.add(lua, data));
    }
}

impl<'lua> LuaIdDatabase<'lua, Twist> for TwistSystemBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "twist";
    const ELEMENT_NAME_PLURAL: &'static str = "twists";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Twist> {
        // TODO: lookup by axis vector
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .unwrap_or_else(|| lua_convert_err(&value, "axis, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Twist]> {
        Cow::Owned(self.alphabetized())
    }
}

impl<'lua> LuaNamedIdDatabase<'lua, Twist> for TwistSystemBuilder {
    fn names(&self) -> &NamingScheme<Twist> {
        &self.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Twist> {
        &mut self.names
    }
}

impl LuaTwistSystem {
    /// Adds a new twist.
    fn add<'lua>(&self, lua: &'lua Lua, data: LuaTable<'lua>) -> LuaResult<Option<LuaTwist>> {
        let axis: LuaAxis;
        let transform: LuaTransform;
        let multipliers: Option<bool>;
        let prefix: Option<String>;
        let name: Option<String>;
        let suffix: Option<String>;
        let inv_name: Option<String>;
        let inv_suffix: Option<String>;
        let inverse: Option<bool>;
        let name_fn: Option<LuaFunction<'_>>;

        unpack_table!(lua.unpack(data {
            axis,
            transform,
            multipliers,
            prefix,
            name,
            suffix,
            inv_name,
            inv_suffix,
            inverse,
            name_fn,
        }));

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

        let mut twists = self.0.lock();
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
                TwistBuilder { axis, transform },
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
                    TwistBuilder { axis, transform },
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
                    TwistBuilder { axis, transform },
                    get_name(i)?,
                    lua_warn_fn(lua),
                )
                .into_lua_err()?;

            if inverse {
                let transform = previous_transform.reverse();
                twists
                    .add_named(
                        TwistBuilder { axis, transform },
                        get_name(-i)?,
                        lua_warn_fn(lua),
                    )
                    .into_lua_err()?;
            }
        }

        Ok(Some(twists.wrap_id(first_twist_id)))
    }
}

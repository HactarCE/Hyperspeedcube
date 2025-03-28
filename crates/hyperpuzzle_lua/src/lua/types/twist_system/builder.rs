use std::sync::Arc;

use hyperpuzzle_core::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::builder::{NameSet, NamingScheme, PuzzleBuilder, TwistBuilder};
use crate::lua::lua_warn_fn;

/// Lua handle to a twist system under construction.
#[derive(Debug, Clone)]
pub struct LuaTwistSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaTwistSystem {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("twistsystem"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            let len = this.lock().twists.len();
            Ok(format!("twistsystem(len={len})"))
        });

        LuaIdDatabase::<Twist>::add_db_metamethods(methods, |Self(puz)| puz);
        LuaNamedIdDatabase::<Twist>::add_named_db_methods(methods, |Self(puz)| puz);

        methods.add_method("add", |lua, this, (axis, transform, data)| {
            this.add(lua, axis, transform, data)
        });
    }
}

impl LuaIdDatabase<Twist> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "twist";
    const ELEMENT_NAME_PLURAL: &'static str = "twists";

    fn value_to_id(&self, lua: &Lua, value: LuaValue) -> LuaResult<Twist> {
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
}

impl LuaNamedIdDatabase<Twist> for PuzzleBuilder {
    fn names(&self) -> &NamingScheme<Twist> {
        &self.twists.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Twist> {
        &mut self.twists.names
    }
}

impl LuaTwistSystem {
    /// Adds a new twist.
    fn add(
        &self,
        lua: &Lua,
        axis: LuaAxis,
        transform: LuaTransform,
        data: Option<LuaTable>,
    ) -> LuaResult<Option<LuaTwist>> {
        let multipliers: Option<bool>;
        let inverse: Option<bool>;
        let prefix: Option<LuaNameSet>;
        let name: Option<LuaNameSet>;
        let suffix: Option<LuaNameSet>;
        let inv_name: Option<LuaNameSet>;
        let inv_suffix: Option<LuaNameSet>;
        let name_fn: Option<LuaFunction>;
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
            multipliers = None;
            inverse = None;
            prefix = None;
            name = None;
            suffix = None;
            inv_name = None;
            inv_suffix = None;
            name_fn = None;
            qtm = None;
            gizmo_pole_distance = None;
        }
        let prefix = prefix.map(|LuaNameSet(s)| s);
        let name = name.map(|LuaNameSet(s)| s);
        let suffix = suffix.map(|LuaNameSet(s)| s);
        let inv_name = inv_name.map(|LuaNameSet(s)| s);
        let inv_suffix = inv_suffix.map(|LuaNameSet(s)| s);

        let prefix = prefix.or(axis.name());

        fn is_name_specified(name_set: &Option<NameSet>) -> bool {
            name_set
                .as_ref()
                .is_some_and(|s| !s.is_empty_string_or_never())
        }

        let do_naming = is_name_specified(&prefix)
            || is_name_specified(&name)
            || is_name_specified(&suffix)
            || is_name_specified(&inv_name)
            || is_name_specified(&inv_suffix)
            || name_fn.is_some();

        let ndim = self.0.lock().ndim();
        let inverse = inverse.unwrap_or(ndim == 3);
        let multipliers = multipliers.unwrap_or(ndim == 3);

        let suffix = suffix.unwrap_or(NameSet::EMPTY_STRING);
        let inv_suffix = inv_suffix.unwrap_or_else(|| match &inv_name {
            Some(_) => suffix.clone(),
            None => NameSet::from("'"),
        });

        if name_fn.is_some() && (name.is_some() || inv_name.is_some()) {
            return Err(LuaError::external(
                "when `name_fn` is specified, `name` and `inv_name` must not be specified",
            ));
        }

        let prefix = prefix.unwrap_or(NameSet::EMPTY_STRING);
        let name = name.unwrap_or(NameSet::EMPTY_STRING);
        let inv_name = inv_name.unwrap_or_else(|| name.clone());

        let qtm = qtm.unwrap_or(1);
        if qtm < 1 {
            lua.warning("twist has QTM value less than 1", false);
        }

        let mut puz = self.0.lock();

        if gizmo_pole_distance.is_some() && puz.ndim() != 3 && puz.ndim() != 4 {
            return Err(LuaError::external(
                "twist gizmo is only supported in 3D and 4D",
            ));
        }

        let twists = &mut puz.twists;
        let axis = axis.id;

        let base_transform = transform.0;

        let get_name = |i: i32| {
            if let Some(name_fn) = &name_fn {
                name_fn.call(i).map(|LuaNameSet(s)| s)
            } else if do_naming {
                match i {
                    1 => Ok(NameSet::new_seq([&prefix, &name, &suffix])),
                    -1 => Ok(NameSet::new_seq([&prefix, &inv_name, &inv_suffix])),
                    2.. => {
                        let mult = i.to_string().into();
                        Ok(NameSet::new_seq([&prefix, &name, &mult, &suffix]))
                    }
                    ..=-2 => {
                        let mult = (-i).to_string().into();
                        Ok(NameSet::new_seq([&prefix, &inv_name, &mult, &inv_suffix]))
                    }
                    0 => Err(LuaError::external("bad twist multiplier")),
                }
            } else {
                Ok(NameSet::EMPTY_STRING)
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
                    include_in_scrambles: true,
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
            let is_equivalent_to_reverse = base_transform.is_self_reverse();
            twists
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm,
                        gizmo_pole_distance: gizmo_pole_distance.filter(|_| ndim > 3),
                        include_in_scrambles: !is_equivalent_to_reverse,
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
            } else if transform.is_ident() {
                break;
            }
            previous_transform = transform.clone();

            twists
                .add_named(
                    TwistBuilder {
                        axis,
                        transform,
                        qtm: qtm * i as usize,
                        gizmo_pole_distance: None, // no gizmo for multiples
                        include_in_scrambles: true,
                    },
                    get_name(i)?,
                    lua_warn_fn(lua),
                )
                .into_lua_err()?;

            if inverse {
                let transform = previous_transform.reverse();
                let is_equivalent_to_reverse = previous_transform.is_self_reverse();
                twists
                    .add_named(
                        TwistBuilder {
                            axis,
                            transform,
                            qtm: qtm * i as usize,
                            gizmo_pole_distance: None, // no gizmo for multiples
                            include_in_scrambles: !is_equivalent_to_reverse,
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

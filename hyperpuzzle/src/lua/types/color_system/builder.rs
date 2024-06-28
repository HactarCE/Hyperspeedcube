use std::borrow::Cow;
use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::builder::{CustomOrdering, NamingScheme, PuzzleBuilder};
use crate::lua::lua_warn_fn;
use crate::puzzle::Color;

/// Lua handle to the color system of a shape under construction.
#[derive(Debug, Clone)]
pub struct LuaColorSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaColorSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("colorsystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        LuaIdDatabase::<Color>::add_db_metamethods(methods, |Self(puz)| puz.lock());
        LuaNamedIdDatabase::<Color>::add_named_db_methods(methods, |Self(puz)| puz.lock());
        LuaOrderedIdDatabase::<Color>::add_ordered_db_methods(methods, |Self(puz)| puz.lock());

        methods.add_method("add", |lua, this, data| this.add(lua, data));

        methods.add_method("set_defaults", |lua, this, new_default_colors| {
            this.set_default_colors(lua, new_default_colors)
        });
    }
}

impl<'lua> LuaIdDatabase<'lua, Color> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "color";
    const ELEMENT_NAME_PLURAL: &'static str = "colors";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Color> {
        // TODO: lookup by surface (single surface, or exact set of surfaces)
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .or_else(|| self.value_to_id_by_index(lua, &value))
            .unwrap_or_else(|| lua_convert_err(&value, "color, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.shape.colors.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Color]> {
        Cow::Borrowed(self.shape.colors.ordering.ids_in_order())
    }
}

impl<'lua> LuaOrderedIdDatabase<'lua, Color> for PuzzleBuilder {
    fn ordering(&self) -> &CustomOrdering<Color> {
        &self.shape.colors.ordering
    }
    fn ordering_mut(&mut self) -> &mut CustomOrdering<Color> {
        &mut self.shape.colors.ordering
    }
}

impl<'lua> LuaNamedIdDatabase<'lua, Color> for PuzzleBuilder {
    fn names(&self) -> &NamingScheme<Color> {
        &self.shape.colors.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Color> {
        &mut self.shape.colors.names
    }
}

impl LuaColorSystem {
    /// Adds a new color.
    fn add<'lua>(&self, lua: &'lua Lua, data: LuaValue<'lua>) -> LuaResult<LuaColor> {
        let name: Option<String>;
        let surfaces: LuaHyperplaneSet;
        let default_color: Option<String>;
        if let Ok(s) = lua.unpack(data.clone()) {
            name = s;
            surfaces = LuaHyperplaneSet::default();
            default_color = None;
        } else if let Ok(h) = lua.unpack(data.clone()) {
            name = None;
            surfaces = h;
            default_color = None;
        } else if let LuaValue::Table(t) = data {
            unpack_table!(lua.unpack(t {
                name,
                surfaces,
                default_color,
            }));
        } else {
            return lua_convert_err(&data, "hyperplane or table");
        };

        let mut puz = self.0.lock();
        let colors = &mut puz.shape.colors;
        let id = colors.add(surfaces.0).into_lua_err()?;
        colors.get_mut(id).into_lua_err()?.default_color = default_color;
        colors.names.set_short_name(id, name, lua_warn_fn(lua));
        Ok(puz.wrap_id(id))
    }

    /// Sets some default colors, leaving other unmodified.
    fn set_default_colors<'lua>(
        &self,
        lua: &'lua Lua,
        new_default_colors: LuaValue<'lua>,
    ) -> LuaResult<()> {
        // First, assemble a list of all the new default colors.
        let mut puz = self.0.lock();

        // This is similar to `LuaIdDatabase::mapping_from_value()`, but it
        // allows the keys themselves to be tables of values (and then adds
        // `[n]` to the end of the value string).
        let mut kv_pairs: Vec<(Color, Option<String>)> = vec![];
        match new_default_colors {
            LuaValue::Table(t) => {
                for pair in t.pairs() {
                    let (key, value): (LuaValue<'_>, Option<String>) = pair?;
                    // IIFE to mimic try_block
                    let result = (|| {
                        if let LuaValue::Table(t2) = key {
                            // Table of values -> color set
                            for (i, k) in t2.sequence_values().enumerate() {
                                let v = match &value {
                                    Some(s) => Some(format!("{s} [{}]", i + 1)),
                                    None => None,
                                };
                                kv_pairs.push((puz.value_to_id(lua, k?)?, v));
                            }
                        } else {
                            kv_pairs.push((puz.value_to_id(lua, key)?, value));
                        }
                        LuaResult::Ok(())
                    })();
                    if let Err(e) = result {
                        lua.warning(e.to_string(), true);
                    }
                }
            }

            LuaValue::Function(f) => {
                for &id in &*puz.ids_in_order() {
                    let value = f.call(puz.wrap_id(id))?;
                    kv_pairs.push((id, value));
                }
            }

            mapping_value => return lua_convert_err(&mapping_value, "table or function"),
        }

        for (k, v) in kv_pairs {
            puz.shape.colors.get_mut(k).into_lua_err()?.default_color = v;
        }

        Ok(())
    }
}

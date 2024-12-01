use std::borrow::Cow;
use std::sync::Arc;

use itertools::Itertools;
use parking_lot::Mutex;

use super::*;
use crate::builder::{CustomOrdering, NamingScheme, PuzzleBuilder};
use crate::lua::lua_warn_fn;
use crate::puzzle::Color;
use crate::{DefaultColor, PerColor};

/// Lua handle to the color system of a shape under construction.
#[derive(Debug, Clone)]
pub struct LuaColorSystem(pub Arc<Mutex<PuzzleBuilder>>);

impl LuaUserData for LuaColorSystem {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("colorsystem"));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaIdDatabase::<Color>::add_db_metamethods(methods, |Self(puz)| &*puz);
        LuaNamedIdDatabase::<Color>::add_named_db_methods(methods, |Self(puz)| &*puz);
        LuaOrderedIdDatabase::<Color>::add_ordered_db_methods(methods, |Self(puz)| &puz);

        methods.add_method("add", |lua, this, data| this.add(lua, data));

        methods.add_method("add_scheme", |lua, this, (name, data)| {
            this.add_scheme(lua, name, data)
        });

        methods.add_method("set_defaults", |lua, this, new_default_colors| {
            this.set_default_colors(lua, new_default_colors)
        });
    }
}

impl LuaIdDatabase<Color> for PuzzleBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "color";
    const ELEMENT_NAME_PLURAL: &'static str = "colors";

    fn value_to_id(&self, lua: &Lua, value: LuaValue) -> LuaResult<Color> {
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

impl LuaOrderedIdDatabase<Color> for PuzzleBuilder {
    fn ordering(&self) -> &CustomOrdering<Color> {
        &self.shape.colors.ordering
    }
    fn ordering_mut(&mut self) -> &mut CustomOrdering<Color> {
        &mut self.shape.colors.ordering
    }
}

impl LuaNamedIdDatabase<Color> for PuzzleBuilder {
    fn names(&self) -> &NamingScheme<Color> {
        &self.shape.colors.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Color> {
        &mut self.shape.colors.names
    }
}

impl LuaColorSystem {
    /// Adds a new color.
    fn add(&self, lua: &Lua, data: LuaValue) -> LuaResult<LuaColor> {
        let name: Option<LuaNameSet>;
        let display: Option<String>;
        let default: Option<String>;
        if let Ok(s) = lua.unpack(data.clone()) {
            name = s;
            display = None;
            default = None;
        } else if let LuaValue::Table(t) = data {
            unpack_table!(lua.unpack(t {
                name,
                display,
                default,
            }));
        } else {
            return lua_convert_err(&data, "hyperplane or table");
        };
        let name = name.map(|LuaNameSet(name_set)| name_set);

        let mut puz = self.0.lock();
        let colors = &mut puz.shape.colors;
        let id = colors.add().into_lua_err()?;
        colors.set_default_color(id, default_color_from_str(lua, default));
        colors.names.set_name(id, name, lua_warn_fn(lua));
        colors.names.set_display(id, display, lua_warn_fn(lua));
        Ok(puz.wrap_id(id))
    }

    /// Adds a new named color scheme.
    fn add_scheme(&self, lua: &Lua, name: String, data: LuaValue) -> LuaResult<()> {
        let mapping = self.color_mapping_from_value(lua, data)?;
        self.0.lock().shape.colors.add_scheme(name, mapping);
        Ok(())
    }

    /// Sets the default color scheme.
    fn set_default_colors(&self, lua: &Lua, data: LuaValue) -> LuaResult<()> {
        let default_scheme_name = self.0.lock().shape.colors.default_scheme_name().to_owned();
        self.add_scheme(lua, default_scheme_name, data)
    }

    fn color_mapping_from_value(
        &self,
        lua: &Lua,
        mapping_to_color_names: LuaValue,
    ) -> LuaResult<PerColor<Option<DefaultColor>>> {
        let puz = self.0.lock();

        // This is similar to `LuaIdDatabase::mapping_from_value()`, but it
        // allows the keys themselves to be tables of values (and then adds
        // `[n]` to the end of the value string).
        let mut mapping: PerColor<Option<DefaultColor>> = std::iter::repeat(None)
            .take(puz.shape.colors.len())
            .collect();
        match mapping_to_color_names {
            LuaValue::Table(t) => {
                for pair in t.pairs() {
                    let (key, value): (LuaValue, String) = pair?;
                    // IIFE to mimic try_block
                    let result = (|| {
                        if let LuaValue::Table(t2) = key {
                            // Table of values -> color set
                            for (i, k) in t2.sequence_values().enumerate() {
                                let value = DefaultColor::Set {
                                    set_name: value.clone(),
                                    index: i, // 0-indexed
                                };
                                mapping[puz.value_to_id(lua, k?)?] = Some(value);
                            }
                        } else {
                            mapping[puz.value_to_id(lua, key)?] =
                                default_color_from_str(lua, Some(value));
                        }
                        LuaResult::Ok(())
                    })();
                    if let Err(e) = result {
                        lua.warning(e.to_string(), true);
                    }
                }
            }

            LuaValue::Function(f) => {
                let colors = puz
                    .ids_in_order()
                    .iter()
                    .map(|&id| puz.wrap_id(id))
                    .collect_vec();
                drop(puz); // Avoid a deadlock in `f.call()`
                for color in colors {
                    let id = color.id;
                    let value = f.call(color)?;
                    mapping[id] = default_color_from_str(lua, Some(value));
                }
            }

            mapping_value => return lua_convert_err(&mapping_value, "table or function"),
        }

        Ok(mapping)
    }
}

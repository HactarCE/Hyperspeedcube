use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;

use crate::builder::{CustomOrdering, NamingScheme, ShapeBuilder};
use crate::puzzle::Color;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaColorSystem(pub Arc<Mutex<ShapeBuilder>>);

impl LuaUserData for LuaColorSystem {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("colorsystem"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        ShapeBuilder::add_db_metamethods(methods, |Self(shape)| shape.lock());
        ShapeBuilder::add_named_db_methods(methods, |Self(shape)| shape.lock());
        ShapeBuilder::add_ordered_db_methods(methods, |Self(shape)| shape.lock());

        methods.add_method("add", |lua, this, data| this.add(lua, data));

        methods.add_method("set_defaults", |lua, this, new_default_colors| {
            this.set_default_colors(lua, new_default_colors)
        });
    }
}

impl<'lua> LuaIdDatabase<'lua, Color> for ShapeBuilder {
    const ELEMENT_NAME_SINGULAR: &'static str = "color";
    const ELEMENT_NAME_PLURAL: &'static str = "colors";

    fn value_to_id(&self, lua: &'lua Lua, value: LuaValue<'lua>) -> LuaResult<Color> {
        // TODO: lookup by manifold (single manifold, or exact set of manifolds)
        self.value_to_id_by_userdata(lua, &value)
            .or_else(|| self.value_to_id_by_name(lua, &value))
            .or_else(|| self.value_to_id_by_index(lua, &value))
            .unwrap_or_else(|| lua_convert_err(&value, "color, string, or integer index"))
    }

    fn db_arc(&self) -> Arc<Mutex<Self>> {
        self.arc()
    }
    fn db_len(&self) -> usize {
        self.colors.len()
    }
    fn ids_in_order(&self) -> Cow<'_, [Color]> {
        Cow::Borrowed(self.colors.ordering.ids_in_order())
    }
}

impl<'lua> LuaOrderedIdDatabase<'lua, Color> for ShapeBuilder {
    fn ordering(&self) -> &CustomOrdering<Color> {
        &self.colors.ordering
    }
    fn ordering_mut(&mut self) -> &mut CustomOrdering<Color> {
        &mut self.colors.ordering
    }
}

impl<'lua> LuaNamedIdDatabase<'lua, Color> for ShapeBuilder {
    fn names(&self) -> &NamingScheme<Color> {
        &self.colors.names
    }
    fn names_mut(&mut self) -> &mut NamingScheme<Color> {
        &mut self.colors.names
    }
}

impl LuaColorSystem {
    fn add<'lua>(&self, lua: &'lua Lua, data: LuaValue<'lua>) -> LuaResult<LuaColor> {
        let name: Option<String>;
        let manifolds: LuaManifoldSet;
        let default_color: Option<String>;
        if let Ok(m) = lua.unpack(data.clone()) {
            name = None;
            manifolds = m;
            default_color = None;
        } else if let LuaValue::Table(t) = data {
            unpack_table!(lua.unpack(t {
                name,
                manifolds,
                default_color,
            }));
        } else {
            return lua_convert_err(&data, "manifold or table");
        };

        let mut shape = self.0.lock();
        let id = shape.colors.add(manifolds.0).into_lua_err()?;
        shape.colors.get_mut(id).into_lua_err()?.default_color = default_color;
        shape.colors.names.set(id, name).into_lua_err()?;
        Ok(shape.wrap_id(id))
    }

    fn set_default_colors<'lua>(
        &self,
        lua: &'lua Lua,
        new_default_colors: LuaValue<'lua>,
    ) -> LuaResult<()> {
        // First, assemble a list of all the new default colors.
        let mut shape = self.0.lock();

        let kv_pairs: Vec<(Color, Option<String>)> =
            shape.mapping_from_value(lua, new_default_colors)?;

        for (k, v) in kv_pairs {
            shape.colors.get_mut(k).into_lua_err()?.default_color = v;
        }

        Ok(())
    }
}

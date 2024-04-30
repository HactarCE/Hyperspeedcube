use hypermath::pga::Motor;
use hypermath::Hyperplane;
use itertools::Itertools;

use super::*;
use crate::builder::ShapeBuilder;
use crate::puzzle::Color;

/// Lua handle to a color in the color system of a shape under construction.
pub type LuaColor = LuaDbEntry<Color, ShapeBuilder>;

impl LuaUserData for LuaColor {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("color"));

        LuaNamedIdDatabase::add_named_db_entry_fields(fields);
        LuaOrderedIdDatabase::add_ordered_db_entry_fields(fields);

        fields.add_field_method_get("surfaces", |lua, this| {
            let db = this.db.lock();
            let surfaces = db.colors.get(this.id).into_lua_err()?.surfaces();
            let lua_surfaces = surfaces.iter().cloned().map(LuaHyperplane);
            let t = lua.create_table_from(lua_surfaces.enumerate())?;
            seal_table(lua, &t)?;
            Ok(t)
        });

        fields.add_field_method_get("default_color", |_lua, this| {
            let db = this.db.lock();
            Ok(db.colors.get(this.id).into_lua_err()?.default_color.clone())
        });
        fields.add_field_method_set("default_color", |_lua, this, new_default_color| {
            let mut db = this.db.lock();
            db.colors.get_mut(this.id).into_lua_err()?.default_color = new_default_color;
            Ok(())
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            if let Some(name) = this.db.lock().names().get(this.id) {
                Ok(format!("color({name:?})"))
            } else {
                Ok(format!("color({})", this.id))
            }
        });
    }
}

impl LuaColor {
    /// Returns the hyperplane for each surface that is assigned this color.
    pub fn hyperplanes(&self) -> LuaResult<Vec<Hyperplane>> {
        let db = self.db.lock();
        Ok(db.colors.get(self.id).into_lua_err()?.surfaces().to_vec())
    }

    /// Returns the color that contains an equivalent surface set to this color,
    /// but transformed by `t`.
    pub fn transform(&self, t: &Motor) -> LuaResult<Option<Self>> {
        let db = self.db.lock();
        let transformed_surfaces = self
            .hyperplanes()?
            .into_iter()
            .map(|hyperplane| t.transform(&hyperplane))
            .collect_vec();
        Ok(db
            .colors
            .surface_set_to_id(&transformed_surfaces)
            .map(|id| db.wrap_id(id)))
    }
}

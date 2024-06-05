use hypermath::pga::Motor;
use hypermath::Hyperplane;
use itertools::Itertools;

use super::*;
use crate::builder::PuzzleBuilder;
use crate::puzzle::Color;

/// Lua handle to a color in the color system of a shape under construction.
pub type LuaColor = LuaDbEntry<Color, PuzzleBuilder>;

impl LuaUserData for LuaColor {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("color"));

        LuaNamedIdDatabase::add_named_db_entry_fields(fields);
        LuaOrderedIdDatabase::add_ordered_db_entry_fields(fields);

        fields.add_field_method_get("surfaces", |lua, this| {
            let puz = this.db.lock();
            let colors = &puz.shape.colors;
            let surfaces = colors.get(this.id).into_lua_err()?.surfaces();
            let lua_surfaces = surfaces.iter().cloned().map(LuaHyperplane);
            let t = lua.create_table_from(lua_surfaces.enumerate())?;
            seal_table(lua, &t)?;
            Ok(t)
        });

        fields.add_field_method_get("default_color", |_lua, this| {
            let puz = this.db.lock();
            let colors = &puz.shape.colors;
            Ok(colors.get(this.id).into_lua_err()?.default_color.clone())
        });
        fields.add_field_method_set("default_color", |_lua, this, new_default_color| {
            let mut puz = this.db.lock();
            let colors = &mut puz.shape.colors;
            colors.get_mut(this.id).into_lua_err()?.default_color = new_default_color;
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
        let puz = self.db.lock();
        let colors = &puz.shape.colors;
        Ok(colors.get(self.id).into_lua_err()?.surfaces().to_vec())
    }

    /// Returns the color that contains an equivalent surface set to this color,
    /// but transformed by `t`.
    pub fn transform(&self, t: &Motor) -> LuaResult<Option<Self>> {
        let puz = self.db.lock();
        let colors = &puz.shape.colors;
        let transformed_surfaces = self
            .hyperplanes()?
            .into_iter()
            .map(|hyperplane| t.transform(&hyperplane))
            .collect_vec();
        Ok(colors
            .surface_set_to_id(&transformed_surfaces)
            .map(|id| puz.wrap_id(id)))
    }
}

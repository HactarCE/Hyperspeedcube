use hypermath::{Blade, Isometry};
use itertools::Itertools;
use std::sync::Arc;

use crate::builder::ShapeBuilder;
use crate::puzzle::Color;

use super::*;

// TODO: `ColorSystemBuilder`?
pub type LuaColor = LuaDbEntry<Color, ShapeBuilder>;

impl LuaUserData for LuaColor {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("color"));

        LuaNamedIdDatabase::add_named_db_entry_fields(fields);
        LuaOrderedIdDatabase::add_ordered_db_entry_fields(fields);

        fields.add_field_method_get("manifolds", |lua, this| {
            let db = this.db.lock();
            let manifolds = db.colors.get(this.id).into_lua_err()?.manifolds();
            let lua_manifolds = manifolds.iter().map(|manifold| {
                let space = Arc::clone(&db.space);
                LuaManifold { manifold, space }
            });
            let t = lua.create_table_from(lua_manifolds.enumerate())?;
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
    pub fn blades(&self) -> LuaResult<Vec<Blade>> {
        let db = self.db.lock();
        let space = db.space.lock();
        let manifold_set = db.colors.get(self.id).into_lua_err()?.manifolds();
        Ok(manifold_set.iter().map(|m| space.blade_of(m)).collect())
    }

    pub fn transform(&self, t: &Isometry) -> LuaResult<Option<Self>> {
        let db = self.db.lock();
        let mut space = db.space.lock();
        let transformed_manifold_set = self
            .blades()?
            .into_iter()
            .map(|b| t.transform_blade(&b))
            .map(|blade| space.add_manifold(blade))
            .try_collect()
            .into_lua_err()?;

        Ok(db
            .colors
            .manifold_set_to_id()
            .get(&transformed_manifold_set)
            .map(|&id| db.wrap_id(id)))
    }
}

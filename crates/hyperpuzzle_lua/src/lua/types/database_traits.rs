use std::fmt;
use std::sync::Arc;

use hypermath::collections::*;
use itertools::Itertools;
use parking_lot::Mutex;

use super::*;
use crate::builder::NamingScheme;
use crate::lua::{lua_warn_fn, result_to_ok_or_warn};

/// Lua handle to an object in a collection, indexed by some ID.
pub struct LuaDbEntry<I, D> {
    /// ID of the object.
    pub id: I,
    /// Underlying database.
    pub db: Arc<Mutex<D>>,
}
impl<I: fmt::Debug, D> fmt::Debug for LuaDbEntry<I, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LuaDbEntry").field("id", &self.id).finish()
    }
}
impl<I: Clone, D> Clone for LuaDbEntry<I, D> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            db: Arc::clone(&self.db),
        }
    }
}
impl<I: PartialEq, D> PartialEq for LuaDbEntry<I, D> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && Arc::ptr_eq(&self.db, &other.db)
    }
}
impl<I: Eq, D> Eq for LuaDbEntry<I, D> {}

impl<I, D> FromLua for LuaDbEntry<I, D>
where
    Self: 'static + LuaUserData + Clone,
{
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

/// Database of Lua values stored in a [`GenericVec<I>`].
pub trait LuaIdDatabase<I: IndexNewtype>: 'static + Sized + Send
where
    LuaDbEntry<I, Self>: LuaUserData,
{
    /// User-friendly string for a single object in the collection.
    const ELEMENT_NAME_SINGULAR: &'static str;
    /// User-friendly string for multiple objects in the collection.
    const ELEMENT_NAME_PLURAL: &'static str;

    /// Converts the ID of an entry to a [`LuaValue`].
    fn wrap_id(&self, id: I) -> LuaDbEntry<I, Self> {
        let db = self.db_arc();
        LuaDbEntry { id, db }
    }
    /// Converts a [`LuaValue`] to an entry ID, or returns an error if no such
    /// entry exists. Many different types are accepted depending on the
    /// collection; most often, names and indices are accepted.
    fn value_to_id(&self, lua: &Lua, value: LuaValue) -> LuaResult<I>;

    /// Converts a [`LuaValue`] to an entry ID if it is a [`LuaDbEntry`]
    /// userdata value, or returns `None` if it is not.
    fn value_to_id_by_userdata(&self, lua: &Lua, value: &LuaValue) -> Option<LuaResult<I>>
    where
        LuaDbEntry<I, Self>: LuaUserData,
    {
        let LuaDbEntry { id, db } = cast_userdata(lua, value).ok()?;
        Some(match Arc::ptr_eq(&self.db_arc(), &db) {
            true => Ok(id),
            false => Err(LuaError::external(
                "cannot operate on entries from a different database",
            )),
        })
    }

    /// Converts a [`LuaValue`] to an entry ID if it is an index, or returns
    /// `None` if it is not.
    fn value_to_id_by_index(&self, lua: &Lua, value: &LuaValue) -> Option<LuaResult<I>> {
        let LuaIndex(i) = lua.unpack(value.clone()).ok()?;
        let len = self.db_len();
        if i < len {
            Some(I::try_from_usize(i).map_err(LuaError::external))
        } else {
            Some(Err(LuaError::external(if len == 1 {
                format!(
                    "index {} is out of range; there is only 1 {}",
                    i + 1,
                    Self::ELEMENT_NAME_SINGULAR,
                )
            } else {
                format!(
                    "index {} is out of range; there are only {} {}",
                    i + 1,
                    self.db_len(),
                    Self::ELEMENT_NAME_PLURAL,
                )
            })))
        }
    }

    /// Returns an `Arc` reference to the DB.
    fn db_arc(&self) -> Arc<Mutex<Self>>;
    /// Returns the number of entries in the database.
    fn db_len(&self) -> usize;
    /// Returns a list of IDs in the database in canonical order.
    fn ids_in_order(&self) -> hypermath::collections::generic_vec::IndexIter<I> {
        I::iter(self.db_len())
    }

    /// Constructs a mapping from ID to `T` from a Lua value, which may be a
    /// table of pairs `(id, T)` or a function from ID to `T`.
    fn mapping_from_value<T: FromLua>(
        this: &Mutex<Self>,
        lua: &Lua,
        mapping_value: LuaValue,
    ) -> LuaResult<Vec<(I, T)>> {
        let db = this.lock();
        match mapping_value {
            LuaValue::Table(t) => Ok(t
                .pairs()
                .map(|pair| {
                    let (id, new_value) = pair?;
                    LuaResult::Ok((db.value_to_id(lua, id)?, new_value))
                })
                .filter_map(result_to_ok_or_warn(lua_warn_fn::<LuaError>(lua)))
                .collect()),

            LuaValue::Function(f) => {
                let ids_in_order = db
                    .ids_in_order()
                    .map(|id| db.wrap_id(id.clone()))
                    .collect_vec();

                drop(db); // Unlock mutex

                Ok(ids_in_order
                    .into_iter()
                    .map(|db_entry| Ok((db_entry.id.clone(), f.call(db_entry)?)))
                    .filter_map(result_to_ok_or_warn(lua_warn_fn::<LuaError>(lua)))
                    .collect())
            }

            _ => lua_convert_err(&mapping_value, "table or function"),
        }
    }

    /// Defines the following methods:
    /// - `__tostring` (metamethod)
    /// - `__index` (metamethod)
    /// - `__len` (metamethod)
    fn add_db_metamethods<T: 'static + mlua::UserData, M: LuaUserDataMethods<T>>(
        methods: &mut M,
        as_mutex_db: fn(&T) -> &Mutex<Self>,
    ) {
        methods.add_meta_method(LuaMetaMethod::ToString, move |lua, this, ()| {
            let type_name = T::type_name(lua)?;
            let ptr = as_mutex_db(this).lock().db_arc().data_ptr();
            Ok(format!("{type_name}({ptr:p})"))
        });

        methods.add_meta_method(LuaMetaMethod::Index, move |lua, this, index| {
            let db = as_mutex_db(this).lock();
            match db.value_to_id(lua, index) {
                Ok(id) => Ok(Some(db.wrap_id(id))),
                Err(_) => Ok(None),
            }
        });
        methods.add_meta_method(LuaMetaMethod::Len, move |_lua, this, ()| {
            let db = as_mutex_db(this).lock();
            Ok(db.db_len())
        });
    }

    /// Defines the following fields on a database entry:
    /// - `index`
    fn add_db_entry_fields<F: LuaUserDataFields<LuaDbEntry<I, Self>>>(fields: &mut F) {
        fields.add_field_method_get("index", |_lua, this| Ok(this.id.to_usize()));
    }
}

/// Extension of [`LuaIdDatabase`] to support naming elements.
pub trait LuaNamedIdDatabase<I: IndexNewtype>: LuaIdDatabase<I>
where
    LuaDbEntry<I, Self>: LuaUserData,
{
    /// Returns a reference to the naming scheme of the database.
    fn names(&self) -> &NamingScheme<I>;
    /// Returns a mutable reference to the naming scheme of the database.
    fn names_mut(&mut self) -> &mut NamingScheme<I>;

    /// Converts a [`LuaValue`] to an entry ID if it is a string containing an
    /// element name, or returns `None` if it is not.
    #[must_use]
    fn value_to_id_by_name(&self, _lua: &Lua, value: &LuaValue) -> Option<LuaResult<I>> {
        let s = value.as_str()?;
        Some(match self.names().names_to_ids().get(&*s) {
            Some(id) => Ok(id.clone()),
            None => Err(LuaError::external(format!("no entry named {s:?}"))),
        })
    }

    /// Renames all elements.
    fn rename_all(
        &mut self,
        lua: &Lua,
        ids_and_new_names: Vec<(I, Option<LuaNameSet>)>,
    ) -> LuaResult<NamingScheme<I>> {
        // We need to rename all the entries at once, so just construct a new
        // naming scheme from scratch.
        let mut new_names = NamingScheme::new(self.names().regex());

        // Set the new names.
        for (id, new_name) in ids_and_new_names {
            let new_name = new_name.map(|LuaNameSet(name_set)| name_set);
            new_names.set_name(id, new_name, lua_warn_fn(lua));
        }

        Ok(new_names)
    }

    /// Defines the following methods on a database:
    /// - `rename`
    fn add_named_db_methods<T: 'static, M: LuaUserDataMethods<T>>(
        methods: &mut M,
        as_mutex_db: fn(&T) -> &Mutex<Self>,
    ) {
        // Renames all elements according to a table or function.
        methods.add_method("rename", move |lua, this, new_names| {
            // First, assemble a list of all the renames that need to happen.
            let ids_and_new_names: Vec<(I, Option<LuaNameSet>)> =
                LuaIdDatabase::mapping_from_value(as_mutex_db(this), lua, new_names)?;

            // Now lock the database and do all the renames at once.
            let mut db = as_mutex_db(this).lock();
            *db.names_mut() = db.rename_all(lua, ids_and_new_names)?;
            Ok(())
        });
    }

    /// Defines the following fields on a database entry:
    /// - `name`
    fn add_named_db_entry_fields<F: LuaUserDataFields<LuaDbEntry<I, Self>>>(fields: &mut F) {
        fields.add_field_method_get("name", |_lua, this| {
            let db = this.db.lock();
            Ok(db.names().get(this.id.clone()).cloned().map(LuaNameSet))
        });
        fields.add_field_method_set("name", |lua, this, new_name: Option<LuaNameSet>| {
            let mut db = this.db.lock();
            let new_name = new_name.map(|LuaNameSet(name_set)| name_set);
            db.names_mut()
                .set_name(this.id.clone(), new_name, lua_warn_fn(lua));
            Ok(())
        });
    }

    /// Defines the following methods on a database entry:
    /// - concatenation operator `..`
    fn add_named_db_entry_methods<M: LuaUserDataMethods<LuaDbEntry<I, Self>>>(methods: &mut M) {
        methods.add_meta_function(LuaMetaMethod::Concat, |lua, (lhs, rhs)| {
            let (a, b) = if let Ok(this) = cast_userdata::<LuaDbEntry<I, Self>>(lua, &lhs) {
                let db = this.db.lock();
                (
                    db.names()
                        .get(this.id.clone())
                        .map(|s| LuaNameSet(s.clone()))
                        .into_lua(lua)?,
                    rhs,
                )
            } else if let Ok(this) = cast_userdata::<LuaDbEntry<I, Self>>(lua, &rhs) {
                let db = this.db.lock();
                (
                    lhs,
                    db.names()
                        .get(this.id.clone())
                        .map(|s| LuaNameSet(s.clone()))
                        .into_lua(lua)?,
                )
            } else {
                return Err(LuaError::external("invalid metamethod call"));
            };

            lua.globals()
                .get::<LuaFunction>("builtin_concat")?
                .call::<LuaNameSet>((a, b))
        });
    }
}

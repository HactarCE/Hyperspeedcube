use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::ApproxHashMap;
use itertools::Itertools;

use super::*;

/// Lua orbit object.
#[derive(Debug, Clone)]
pub struct LuaOrbit {
    symmetry: LuaSymmetry,
    init: Vec<Transformable>,

    /// Whether names have been assigned.
    has_names: bool,
    /// Indices into `orbit_list`, in iteration order. If `None`, it is assumed
    /// to be equivalent to `0..orbit_len.len()`.
    order: Option<Vec<usize>>,
    /// Elements, in the order that they were generated.
    orbit_list: Vec<(Motor, Option<String>, Vec<Transformable>)>,

    iter_index: Arc<AtomicUsize>,
}

impl<'lua> FromLua<'lua> for LuaOrbit {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaOrbit {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("orbit"));

        fields.add_field_method_get("symmetry", |_lua, this| Ok(this.symmetry.clone()));
        fields.add_field_method_get("init", |lua, this| {
            lua.create_sequence_from(
                this.init
                    .iter()
                    .map(|t| t.into_nillable_lua(lua))
                    .collect::<LuaResult<Vec<_>>>()?,
            )
        });

        fields.add_field_method_get("has_names", |_lua, this| Ok(this.has_names));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, this, ()| {
            Ok(this.orbit_list.len())
        });

        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, ()| {
            // Get the index of the Lua iteration.
            let iter_index = this.iter_index.fetch_add(1, Ordering::Relaxed);
            // Look up that index in the custom ordering, if there is one.
            let orbit_index = match &this.order {
                Some(order) => order.get(iter_index).copied(),
                None => Some(iter_index),
            };

            // Return multiple values.
            let mut values = vec![];
            if let Some(i) = orbit_index {
                if let Some((transform, name, objects)) = this.orbit_list.get(i) {
                    // The first value is the transform.
                    values.push(LuaTransform(transform.clone()).into_lua(lua)?);
                    // Then push the objects.
                    for obj in objects {
                        values.push(obj.into_nillable_lua(lua)?);
                    }
                    // If custom names are given, then the last value is the
                    // custom name.
                    if this.has_names {
                        values.push(name.as_deref().into_lua(lua)?);
                    }
                }
            }
            Ok(LuaMultiValue::from_vec(values))
        });

        methods.add_method("iter", |_lua, this, ()| {
            Ok(Self {
                iter_index: Arc::new(AtomicUsize::new(0)),
                ..this.clone()
            })
        });

        methods.add_method("names", |lua, this, ()| {
            this.has_names
                .then(|| {
                    lua.create_sequence_from(
                        this.orbit_list.iter().map(|(_, name, _)| name.clone()),
                    )
                })
                .transpose()
        });

        methods.add_method("with", |lua, this, names_and_order_table| {
            if this.order.is_some() {
                return Err(LuaError::external("orbit already has names and ordering"));
            }
            let names_and_order = names_and_order_from_table(lua, names_and_order_table)?;
            let mut lookup = ApproxHashMap::new();
            for (i, (_motor, _empty_name, objects)) in this.orbit_list.iter().enumerate() {
                lookup.insert(objects.clone(), i);
            }
            let mut order = vec![];
            let mut new_orbit_list = this.orbit_list.clone();
            let mut seen: Vec<bool> = vec![false; new_orbit_list.len()];
            for (name, motor) in names_and_order {
                if let Some(&index) = lookup.get(&motor.transform(&this.init)) {
                    seen[index] = true;
                    let (_motor, name_mut, _objects) = &mut new_orbit_list[index];
                    if let Some(old_name) = name_mut {
                        let msg =
                            format!("duplicate in symmetry orbit order: {old_name:?} and {name:?}");
                        lua.warning(msg, false);
                    } else {
                        *name_mut = Some(name);
                        order.push(index);
                    }
                }
            }

            // Check for missing elements.
            for i in seen.iter().positions(|&b| !b) {
                order.push(i);
            }

            Ok(Self {
                symmetry: this.symmetry.clone(),
                init: this.init.clone(),

                has_names: true,
                order: Some(order),
                orbit_list: new_orbit_list,

                iter_index: Arc::new(AtomicUsize::new(0)),
            })
        });
    }
}

impl LuaOrbit {
    /// Returns the orbit of `init` under `symmetry`.
    pub fn new(symmetry: LuaSymmetry, init: Vec<Transformable>) -> Self {
        let orbit_list = symmetry
            .orbit(init.clone())
            .into_iter()
            // Assign empty names.
            .map(|(motor, objects)| (motor, None, objects))
            .collect_vec();
        Self {
            symmetry,
            init,

            has_names: false,
            order: None,
            orbit_list,

            iter_index: Arc::new(AtomicUsize::new(0)),
        }
    }
    /// Returns the symmetry used to generate the orbit.
    pub fn symmetry(&self) -> &LuaSymmetry {
        &self.symmetry
    }
    /// Returns the initial seed objects that this is the orbit of.
    pub fn init(&self) -> &[Transformable] {
        &self.init
    }
    /// Returns whether the orbit has custom names assigned to any elements.
    pub fn has_names(&self) -> bool {
        // Ok technically it's possible to have `names = Some(vec![])` in which
        // case no elements have any names. But the comment above is accurate
        // enough.
        self.has_names
    }
    /// Returns an iterator over the whole orbit.
    pub fn iter_in_order(
        &self,
    ) -> impl Iterator<Item = &(Motor, Option<String>, Vec<Transformable>)> {
        match &self.order {
            Some(order) => order.iter().flat_map(|&i| self.orbit_list.get(i)).collect(),
            None => self.orbit_list.iter().collect_vec(),
        }
        .into_iter()
    }
}

/// Constructs an assignment of names and ordering based on a table for a
/// particular symmetry group.
pub fn names_and_order_from_table<'lua>(
    lua: &'lua Lua,
    table: LuaTable<'lua>,
) -> LuaResult<Vec<(String, Motor)>> {
    let symmetry = table.get::<_, LuaSymmetry>("symmetry")?;

    let mut order = vec![];
    // Some values are given directly.
    let mut known = HashMap::<String, Motor>::new();
    // Some must be computed based on other values.
    let mut unknown = HashMap::<String, Vec<(String, Motor)>>::new();

    for entry in table.sequence_values::<LuaValue<'_>>() {
        let [new_name, key]: [LuaValue<'_>; 2] = <_>::from_lua(entry?, lua)?;
        let new_name = String::from_lua(new_name, lua)?;
        order.push(new_name.clone());

        let mirror_seq = LuaTable::from_lua(key.clone(), lua)?;
        let mut iter = mirror_seq.sequence_values::<LuaValue<'_>>();
        match iter.next() {
            None => {
                let motor = symmetry.motor_for_mirror_seq([])?;
                known.insert(new_name, motor);
            }
            Some(init) => {
                let init_name = String::from_lua(init?, lua)?;
                let mirror_indices: Vec<usize> = iter
                    .map(|v| LuaIndex::from_lua(v?, lua).map(|LuaIndex(i)| i))
                    .try_collect()?;
                let motor = symmetry.motor_for_mirror_seq(mirror_indices)?;
                unknown
                    .entry(init_name)
                    .or_default()
                    .push((new_name, motor));
            }
        }
    }

    // Resolve lazy evaluation.
    let mut queue = known.keys().cloned().collect_vec();
    while let Some(next_known) = queue.pop() {
        if let Some(unprocessed) = unknown.remove(&next_known) {
            for (new_name, motor) in unprocessed {
                let value = motor * &known[&next_known];
                known.insert(new_name.clone(), value);
                queue.push(new_name);
            }
        }
    }
    if let Some(unprocessed_key) = unknown.keys().next() {
        lua.warning(format!("unknown key {unprocessed_key:?}"), false);
    }

    // Assemble into ordered list.
    Ok(order
        .into_iter()
        .filter_map(|name| {
            let motor = known.remove(&name)?;
            Some((name, motor))
        })
        .collect())
}

use std::sync::Arc;

use hyperpuzzle_core::{LogLine, Logger};
use itertools::Itertools;
use mlua::prelude::*;
use parking_lot::Mutex;

use super::*;
use crate::builder::NameSet;

/// Returns a table mapping between axis strings and axis numbers.
fn lua_axes_table(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let axes_table = lua.create_table()?;
    for (i, c) in hypermath::AXIS_NAMES.chars().enumerate().take(6) {
        axes_table.set(LuaIndex(i), c.to_string())?;
        axes_table.set(c.to_string(), LuaIndex(i))?;
        axes_table.set(c.to_ascii_lowercase().to_string(), LuaIndex(i))?;
    }
    seal_table(lua, &axes_table)?;
    Ok(axes_table)
}

pub(super) fn monkeypatch_lua_environment(lua: &Lua) -> LuaResult<()> {
    // Monkeypatch builtin `type` function.
    let globals = lua.globals();
    globals.raw_set(
        "type",
        lua.create_function(|_lua, v| Ok(lua_type_name(&v)))?,
    )?;

    // Monkeypatch math lib.
    let math: LuaTable = globals.get("math")?;
    let round_fn = lua.create_function(|_lua, x: LuaNumber| Ok(x.round()))?;
    math.raw_set("round", round_fn)?;
    // TODO: support other numbers
    let eq_fn = lua
        .create_function(|_lua, (a, b): (LuaNumber, LuaNumber)| Ok(hypermath::approx_eq(&a, &b)));
    math.raw_set("eq", eq_fn?)?;
    let neq_fn = lua
        .create_function(|_lua, (a, b): (LuaNumber, LuaNumber)| Ok(!hypermath::approx_eq(&a, &b)));
    math.raw_set("neq", neq_fn?)?;

    Ok(())
}

pub(super) fn init_lua_environment(loader: &LuaLoader, env: &LuaTable) -> LuaResult<()> {
    let lua = &loader.lua;

    // Constants
    env.raw_set("_PUZZLE_ENGINE", crate::PUZZLE_ENGINE_VERSION_STRING)?;
    env.raw_set("AXES", lua_axes_table(lua)?)?;

    // Imports
    let loader2 = loader.clone();
    let index_metamethod = lua.create_function(
        move |_lua, (_lib_table, index_string): (LuaTable, String)| loader2.load_file(index_string),
    )?;
    let lib_table = crate::lua::create_sealed_table_with_index_metamethod(lua, index_metamethod)?;
    env.raw_set("lib", lib_table)?;

    // Database
    env.raw_set("puzzles", LuaPuzzleDb)?;
    env.raw_set("puzzle_generators", LuaPuzzleGeneratorDb)?;
    env.raw_set("color_systems", LuaColorSystemDb)?;

    // `blade` constructors
    let vec_fn =
        lua.create_function(|lua, LuaVectorFromMultiValue(v)| LuaBlade::from_vector(lua, v));
    env.raw_set("vec", vec_fn?)?;
    let point_fn =
        lua.create_function(|lua, LuaPointFromMultiValue(v)| LuaBlade::from_point(lua, v))?;
    env.raw_set("point", point_fn)?;
    let blade_fn = lua.create_function(|_lua, b: LuaBlade| Ok(b))?;
    env.raw_set("blade", blade_fn)?;
    let plane_fn = lua.create_function(|lua, LuaHyperplaneFromMultivalue(h)| {
        LuaBlade::from_hyperplane(lua, &h)
    })?;
    env.raw_set("plane", plane_fn)?;

    // `symmetry` constructors
    let cd_fn = LuaSymmetry::construct_from_cd;
    env.raw_set("cd", lua.create_function(cd_fn)?)?;
    let symmetry_fn = LuaSymmetry::construct_from_generators;
    env.raw_set("symmetry", lua.create_function(symmetry_fn)?)?;

    // `transform` constructors
    let ident_fn = LuaTransform::construct_identity_lua;
    env.raw_set("ident", lua.create_function(ident_fn)?)?;
    let refl_fn = LuaTransform::construct_reflection_lua;
    env.raw_set("refl", lua.create_function(refl_fn)?)?;
    let rot_fn = LuaTransform::construct_rotation_lua;
    env.raw_set("rot", lua.create_function(rot_fn)?)?;

    // `region` constants
    env.raw_set("REGION_ALL", LuaRegion::Everything)?;
    env.raw_set("REGION_NONE", LuaRegion::Nothing)?;

    // `name` constructors
    fn unpack_name_sets(lua: &Lua, args: LuaMultiValue) -> LuaResult<Vec<NameSet>> {
        args.into_iter()
            .map(|arg| lua.unpack::<LuaNameSet>(arg))
            .map_ok(|LuaNameSet(s)| s)
            .try_collect()
    }
    let names_seq_fn = lua.create_function(move |lua, args| {
        Ok(LuaNameSet(NameSet::new_seq(unpack_name_sets(lua, args)?)))
    })?;
    let names_set_fn = lua.create_function(move |lua, args| {
        Ok(LuaNameSet(NameSet::new_set(unpack_name_sets(lua, args)?)))
    })?;
    let names_any_fn = lua.create_function(move |lua, args| {
        Ok(LuaNameSet(NameSet::any(unpack_name_sets(lua, args)?)))
    })?;
    let names_charset_fn =
        lua.create_function(move |_lua, s: String| Ok(LuaNameSet(NameSet::new_set(s.chars()))))?;
    let names_table = lua.create_table_from([
        ("seq", names_seq_fn),
        ("set", names_set_fn),
        ("any", names_any_fn),
        ("charset", names_charset_fn),
    ])?;
    seal_table(lua, &names_table)?;
    env.raw_set("names", names_table)?;

    // Tag utilities
    env.raw_set(
        "merge_tags",
        lua.create_function(|lua, tables: LuaMultiValue| {
            tables
                .into_iter()
                .map(|t| super::tags::unpack_tags_table(lua, <Option<LuaTable>>::from_lua(t, lua)?))
                .reduce(|a, b| Ok(crate::lua::tags::merge_tag_sets(a?, b?)))
                .unwrap_or(Ok(hyperpuzzle_core::TagSet::new()))
                .map(|tag_set| super::tags::tags_table_to_lua(lua, &tag_set))
        })?,
    )?;

    Ok(())
}

pub(super) fn set_logger(lua: &Lua, logger: &Logger) {
    if let Err(e) = try_set_logger(lua, logger) {
        log::error!("error setting Lua logger: {e}");
    }
}
pub(super) fn try_set_logger(lua: &Lua, logger: &Logger) -> LuaResult<()> {
    if !crate::CAPTURE_LUA_OUTPUT {
        return Ok(());
    }

    let l = logger.clone();
    lua.globals().raw_set(
        "print",
        lua.create_function(move |lua, args: LuaMultiValue| {
            let args: Vec<String> = args.iter().map(|arg| arg.to_string()).try_collect()?;
            l.log(LogLine {
                level: log::Level::Info,
                file: lua_current_filename(lua),
                msg: args.into_iter().join("\t"),
                traceback: None, // we could get a traceback but that would be expensive
            });
            Ok(())
        })?,
    )?;

    let l = logger.clone();
    let partial = Arc::new(Mutex::new(String::new()));
    lua.set_warning_function(move |lua, msg, to_continue| {
        let mut full_msg = partial.lock();
        *full_msg += msg;
        if !to_continue {
            l.log(LogLine {
                level: log::Level::Warn,
                file: lua_current_filename(lua),
                msg: std::mem::take(&mut full_msg),
                traceback: Some(lua_stack_trace(lua)),
            });
        }
        Ok(())
    });

    Ok(())
}

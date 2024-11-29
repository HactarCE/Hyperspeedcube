use mlua::prelude::*;

use super::*;

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

pub(super) fn monkeypatch_lua_environment(lua: &Lua, logger: &LuaLogger) -> LuaResult<()> {
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

    if crate::CAPTURE_LUA_OUTPUT {
        let logger = logger.clone();
        globals.raw_set("print", logger.lua_info_fn(&lua)?)?;
        lua.set_warning_function(move |lua, msg, _to_continue| {
            logger.warn(lua, msg);
            Ok(())
        });
    }

    Ok(())
}

pub(super) fn init_lua_environment(lua: &Lua, env: &LuaTable, loader: LuaLoader) -> LuaResult<()> {
    // Constants
    env.raw_set("_PUZZLE_ENGINE", crate::PUZZLE_ENGINE_VERSION_STRING)?;
    env.raw_set("AXES", lua_axes_table(&lua)?)?;

    // Imports
    let index_metamethod = lua.create_function(
        move |_lua, (_lib_table, index_string): (LuaTable, String)| loader.load_file(index_string),
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
    let ident_fn = LuaTransform::construct_identity;
    env.raw_set("ident", lua.create_function(ident_fn)?)?;
    let refl_fn = LuaTransform::construct_reflection;
    env.raw_set("refl", lua.create_function(refl_fn)?)?;
    let rot_fn = LuaTransform::construct_rotation;
    env.raw_set("rot", lua.create_function(rot_fn)?)?;

    // `region` constants
    env.raw_set("REGION_ALL", LuaRegion::Everything)?;
    env.raw_set("REGION_NONE", LuaRegion::Nothing)?;

    Ok(())
}

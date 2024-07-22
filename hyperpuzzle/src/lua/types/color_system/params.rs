use itertools::Itertools;

use super::*;
use crate::builder::ColorSystemBuilder;
use crate::lua::lua_warn_fn;
use crate::PerColor;

/// Constructs a color system from a Lua specification.
pub fn from_lua_table<'lua>(
    lua: &'lua Lua,
    id: Option<String>,
    table: LuaTable<'lua>,
) -> LuaResult<ColorSystemBuilder> {
    if !table.contains_key("colors")? {
        let mut colors = ColorSystemBuilder::new();
        add_colors_from_table(lua, &mut colors, table, true)?;
        return Ok(colors);
    }

    let name: Option<String>;
    let colors: LuaTable<'_>;
    let schemes: Option<LuaTable<'_>>;
    let default: Option<String>;

    unpack_table!(lua.unpack(table {
        name,
        colors,
        schemes,
        default,
    }));
    let colors_table = colors;

    let mut colors = ColorSystemBuilder::new();

    colors.id = id.map(crate::validate_id).transpose().into_lua_err()?;
    colors.name = name;

    // Set default color scheme.
    if let Some(name) = default {
        colors.default_scheme = Some(name);
    }

    // Add colors.
    add_colors_from_table(lua, &mut colors, colors_table, schemes.is_none())?;

    // Add color schemes.
    if let Some(color_schemes_table) = schemes {
        for scheme in color_schemes_table.sequence_values::<LuaTable<'_>>() {
            let (name, mapping_table) =
                scheme?.sequence_values().collect_tuple().ok_or_else(|| {
                    LuaError::external(
                        "expected color scheme to be \
                         a sequence containing a name \
                         and a table of color mappings",
                    )
                })?;
            add_color_scheme_from_table(
                &mut colors,
                lua.unpack(name?)?,
                lua.unpack(mapping_table?)?,
            )?;
        }
    }

    // Reset the "is modified" flag.
    colors.is_modified = false;

    Ok(colors)
}

fn add_colors_from_table<'lua>(
    lua: &'lua Lua,
    colors: &mut ColorSystemBuilder,
    colors_table: LuaTable<'lua>,
    allow_default: bool,
) -> LuaResult<()> {
    for color in colors_table.sequence_values() {
        let t = color?;

        let name: Option<String>;
        let display: Option<String>;
        let default: Option<String>;
        unpack_table!(lua.unpack(t {
            name,
            display,
            default,
        }));

        if default.is_some() && !allow_default {
            lua.warning(
                "per-color `default` key should not be used with color schemes",
                false,
            );
        }

        let id = colors.add().into_lua_err()?;
        colors.names.set_name(id, name, lua_warn_fn(lua));
        colors.names.set_display(id, display);
        if let Some(s) = default {
            colors.set_default_color(id, Some(s.parse().into_lua_err()?));
        }
    }

    Ok(())
}

fn add_color_scheme_from_table(
    colors: &mut ColorSystemBuilder,
    name: String,
    mapping_table: LuaTable<'_>,
) -> LuaResult<()> {
    let name_to_id = &colors.names.names_to_ids();

    let mut mapping = PerColor::new();
    mapping.resize(colors.len()).into_lua_err()?;
    for pair in mapping_table.pairs::<String, String>() {
        let (k, v) = pair?;
        let id = *name_to_id
            .get(&k)
            .ok_or_else(|| LuaError::external(format!("no color with name {k:?}")))?;
        mapping[id] = Some(v.parse().into_lua_err()?);
    }
    colors.schemes.insert(name, mapping);

    Ok(())
}

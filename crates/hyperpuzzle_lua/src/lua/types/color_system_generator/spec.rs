use std::sync::Arc;

use hyperpuzzle_core::{
    BuildTask, ColorSystem, ColorSystemGenerator, GeneratorParam, Redirectable,
};
use itertools::Itertools;

use super::*;
use crate::lua::lua_warn_fn;

/// Specification for a color system generator.
#[derive(Debug)]
pub struct LuaColorSystemGeneratorSpec {
    /// Internal ID.
    pub id: String,
    /// Human-friendly name.
    pub name: String,

    /// Color system generation parameters.
    pub params: Vec<GeneratorParam>,
    /// Lua function to generate a color system.
    generator: LuaFunction,
}

impl FromLua for LuaColorSystemGeneratorSpec {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let table: LuaTable = lua.unpack(value)?;

        let id: String;
        let name: Option<String>;
        let params: Vec<LuaValue>;
        let r#gen: LuaFunction;
        unpack_table!(lua.unpack(table {
            id,
            name,
            params,
            r#gen,
        }));

        let id = crate::validate_id(id).into_lua_err()?;

        crate::lua::protect_with_local_env(lua, &r#gen)?;

        let name = name.unwrap_or_else(|| {
            lua.warning(
                format!("missing `name` for color system generator `{id}`"),
                false,
            );
            id.clone()
        });

        Ok(LuaColorSystemGeneratorSpec {
            id: id.clone(),
            name,

            params: params
                .into_iter()
                .map(|p| param_from_lua(lua, p))
                .try_collect()?,
            generator: r#gen,
        })
    }
}

impl LuaColorSystemGeneratorSpec {
    /// Converts to a [`ColorSystemGenerator`].
    pub fn into_color_system_generator(self, lua: &Lua) -> ColorSystemGenerator {
        let lua = lua.clone();
        ColorSystemGenerator {
            id: self.id.clone(),
            name: self.name.clone(),
            params: self.params.clone(),
            generate: Box::new(move |ctx, param_values| {
                crate::lua::env::set_logger(&lua, &ctx.logger);
                ctx.progress.lock().task = BuildTask::GeneratingSpec;
                let color_system = self.generate_color_system(&lua, param_values)?;
                Ok(color_system)
            }),
        }
    }

    /// Runs user Lua code to generate a color system.
    #[allow(clippy::get_first)]
    pub fn generate_color_system(
        &self,
        lua: &Lua,
        generator_param_values: Vec<impl ToString>,
    ) -> LuaResult<Redirectable<Arc<ColorSystem>>> {
        let generator_param_values = generator_param_values
            .into_iter()
            .map(|v| v.to_string())
            .collect_vec();

        let id = hyperpuzzle_core::generated_id(&self.id, &generator_param_values);

        let expected = self.params.len();
        let got = generator_param_values.len();
        if expected != got {
            let generator_id = &self.id;
            return Err(LuaError::external(format!(
                "generator {generator_id} expects {expected} params; got {got}",
            )));
        }

        let params: Vec<LuaValue> = std::iter::zip(&self.params, generator_param_values)
            .map(|(p, s)| match p.value_from_str(&s) {
                Ok(v) => param_value_into_lua(lua, &v),
                Err(e) => Err(LuaError::external(e)),
            })
            .try_collect()?;
        let gen_params_table = lua.create_sequence_from(params.clone())?;

        let user_gen_fn_output = self
            .generator
            .call::<LuaMultiValue>(gen_params_table)
            .context("error generating puzzle definition")?
            .into_iter()
            .collect_vec();

        let generated_spec = match user_gen_fn_output.get(0) {
            Some(LuaValue::String(s)) => {
                let redirect_id = s.to_string_lossy();
                return Ok(Redirectable::Redirect(
                    if let Some(val) = user_gen_fn_output.get(1) {
                        let redirect_params: Vec<String> =
                            LuaSequence::<LuaValue>::from_lua(val.clone(), lua)?
                                .0
                                .iter()
                                .map(|v| v.to_string())
                                .try_collect()?;
                        hyperpuzzle_core::generated_id(&redirect_id, redirect_params)
                    } else {
                        redirect_id
                    },
                ));
            }
            Some(LuaValue::Table(tab)) => tab,
            _ => {
                return Err(LuaError::external(
                    "return value of `gen` function must string \
                     (ID redirect) or table (color system specification)",
                ));
            }
        };

        // Inherit defaults from generator.
        let color_system_spec_table = crate::lua::deep_copy_table(lua, generated_spec.clone())?;
        {
            // Set color system ID.
            if color_system_spec_table.contains_key("id")? {
                lua.warning("overwriting `id` outputted by puzzle generator", false);
            }
            color_system_spec_table.raw_set("id", id)?;
        }

        // Generate the color system spec.
        let color_system_builder = crate::lua::types::color_system::from_generated_lua_table(
            lua,
            color_system_spec_table,
        )?;

        let (color_system, _color_map) = color_system_builder
            .build(None, None, lua_warn_fn(lua))
            .into_lua_err()?;

        Ok(Redirectable::Direct(Arc::new(color_system)))
    }
}

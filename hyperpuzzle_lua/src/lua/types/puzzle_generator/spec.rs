use std::collections::HashMap;
use std::sync::Arc;

use hyperpuzzle_core::{
    BuildTask, GeneratorParam, PuzzleListMetadata, PuzzleSpec, PuzzleSpecGenerator, Redirectable,
    TagSet, TagValue,
};
use itertools::Itertools;

use super::*;

/// Specification for a puzzle generator.
#[derive(Debug)]
pub struct LuaPuzzleGeneratorSpec {
    /// Metadata for the puzzle.
    pub meta: PuzzleListMetadata,

    /// Default color system.
    pub colors: Option<String>,

    /// Puzzle generation parameters.
    pub params: Vec<GeneratorParam>,
    /// Examples and special cases for generated puzzles.
    pub examples: HashMap<String, Arc<PuzzleSpec>>,
    /// Lua function to generate a puzzle definition.
    gen: LuaFunction,
}

impl FromLua for LuaPuzzleGeneratorSpec {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let table: LuaTable = lua.unpack(value)?;

        let id: String;
        let version: LuaVersion;
        let colors: Option<String>;
        let tags: Option<LuaTable>;
        let params: Vec<LuaValue>;
        let examples: Option<Vec<PuzzleGeneratorOverrides>>;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let gen: LuaFunction;
        unpack_table!(lua.unpack(table {
            id,
            version,

            colors,
            tags,

            params,
            examples,
            gen,

            name,
            aliases,
        }));

        let id = crate::validate_id(id).into_lua_err()?;
        let mut tags = crate::lua::tags::unpack_tags_table(lua, tags)?;

        for tag in tags.0.keys().filter(|tag| tag.starts_with("type")) {
            lua.warning(format!("generator {id} should not have tag {tag:?}"), false);
        }

        // Add `#generator` tag.
        tags.insert_named("type/generator", TagValue::True)
            .map_err(LuaError::external)?;

        // Add `#file` tag.
        if let Some(filename) = crate::lua::lua_current_filename(lua) {
            tags.insert_named("file", TagValue::Str(filename))
                .map_err(LuaError::external)?;
        }

        crate::lua::tags::inherit_parent_tags(&mut tags);

        crate::lua::protect_with_local_env(lua, &gen)?;

        let name = name.unwrap_or_else(|| {
            lua.warning(format!("missing `name` for puzzle generator `{id}`"), false);
            id.clone()
        });

        let mut ret = LuaPuzzleGeneratorSpec {
            meta: PuzzleListMetadata {
                id: id.clone(),
                version: version.0,
                name,
                aliases: aliases.unwrap_or_default(),
                tags,
            },

            colors,

            params: params
                .into_iter()
                .map(|p| param_from_lua(lua, p))
                .try_collect()?,
            examples: HashMap::new(),
            gen,
        };

        for example in examples.unwrap_or_default() {
            let generator_param_values = example.params.iter().map(|val| val.to_string()).collect();
            let id = crate::generated_puzzle_id(&id, &generator_param_values);
            match ret.generate_puzzle_spec(lua, generator_param_values, Some(example)) {
                Ok(Redirectable::Direct(puzzle_spec)) => {
                    ret.examples
                        .insert(puzzle_spec.meta.id.clone(), puzzle_spec);
                }
                Ok(Redirectable::Redirect(other)) => {
                    lua.warning(
                        format!(
                            "ignoring example puzzle {id:?} \
                             because it redirects to {other:?}",
                        ),
                        false,
                    );
                }
                Err(e) => {
                    lua.warning(format!("error in example puzzle {id:?}: {e}"), false);
                }
            }
        }

        Ok(ret)
    }
}

impl LuaPuzzleGeneratorSpec {
    /// Converts to a [`PuzzleSpecGenerator`].
    pub fn into_puzzle_spec_generator(self, lua: &Lua) -> PuzzleSpecGenerator {
        let lua = lua.clone();
        PuzzleSpecGenerator {
            meta: self.meta.clone(),
            params: self.params.clone(),
            examples: self.examples.clone(),
            generate: Box::new(move |ctx, param_values| {
                crate::lua::env::set_logger(&lua, &ctx.logger);
                ctx.progress.lock().task = BuildTask::GeneratingSpec;
                let puzzle_spec = self.generate_puzzle_spec(&lua, param_values, None)?;
                Ok(puzzle_spec)
            }),
        }
    }

    /// Runs user Lua code to generate a puzzle _definition_.
    ///
    /// Even if `overrides` is `None`, overrides for known example puzzles will
    /// be applied.
    #[allow(clippy::get_first)]
    pub fn generate_puzzle_spec(
        &self,
        lua: &Lua,
        generator_param_values: Vec<impl ToString>,
        overrides: Option<PuzzleGeneratorOverrides>,
    ) -> LuaResult<Redirectable<Arc<PuzzleSpec>>> {
        let generator_param_values = generator_param_values
            .into_iter()
            .map(|v| v.to_string())
            .collect_vec();

        let id = crate::generated_puzzle_id(&self.meta.id, &generator_param_values);

        if let Some(puzzle_spec) = self.examples.get(&id) {
            return Ok(Redirectable::Direct(Arc::clone(puzzle_spec)));
        }

        let expected = self.params.len();
        let got = generator_param_values.len();
        if expected != got {
            let generator_id = &self.meta.id;
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
            .gen
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
                        crate::generated_puzzle_id(&redirect_id, redirect_params)
                    } else {
                        redirect_id
                    },
                ));
            }
            Some(LuaValue::Table(tab)) => tab,
            _ => {
                return Err(LuaError::external(
                    "return value of `gen` function must string \
                     (ID redirect) or table (puzzle specification)",
                ));
            }
        };

        // Inherit defaults from generator.
        let puzzle_spec_table = crate::lua::deep_copy_table(lua, generated_spec.clone())?;
        {
            // Set puzzle ID.
            if puzzle_spec_table.contains_key("id")? {
                lua.warning("overwriting `id` outputted by puzzle generator", false);
            }
            puzzle_spec_table.raw_set("id", id)?;

            // Set version.
            if puzzle_spec_table.contains_key("version")? {
                lua.warning("overwriting `version` outputted by puzzle generator", false);
            }
            puzzle_spec_table.raw_set("version", LuaVersion(self.meta.version))?;

            // Set color system, if it isn't already set.
            if !puzzle_spec_table.contains_key("colors")? {
                puzzle_spec_table.raw_set("colors", self.colors.as_deref())?;
            }
        }

        // Generate the puzzle spec.
        let mut puzzle_spec = LuaPuzzleSpec::from_generated_lua_table(lua, puzzle_spec_table)?;

        let meta = &mut puzzle_spec.meta;

        // Add data from the matching example.
        if let Some(overrides) = overrides {
            let PuzzleGeneratorOverrides {
                params: _,
                name,
                aliases,
                tags,
            } = overrides;

            // Set name from the example, and move the auto-generated name to an
            // alias.
            if let Some(new_name) = name.clone() {
                let old_name = std::mem::replace(&mut meta.name, new_name);
                if old_name != meta.id {
                    meta.aliases.push(old_name);
                }
            }

            // Add aliases from the example.
            meta.aliases.extend(aliases);

            // Add tags from the example, and these tags should have the highest
            // priority.
            let tags_from_example = tags;
            let tags_from_puzzle_spec = std::mem::take(&mut meta.tags);
            meta.tags = crate::lua::tags::merge_tag_sets(tags_from_example, tags_from_puzzle_spec);
        }

        // Add tags from generator.
        let tags_from_generator = self.meta.tags.clone();
        crate::lua::tags::merge_tag_sets_into(&mut meta.tags, tags_from_generator);

        // Remove `#generator` tag.
        meta.tags.0.remove("type/generator");

        // Add `#generated` tag.
        meta.tags
            .insert_named("generated", TagValue::True)
            .map_err(LuaError::external)?;

        crate::lua::tags::inherit_parent_tags(&mut meta.tags);

        Ok(Redirectable::Direct(Arc::new(
            puzzle_spec.into_puzzle_spec(lua),
        )))
    }
}

/// Overrides and additions for certain fields of a [`PuzzleSpec`].
///
/// This is all the data associated with an example puzzle.
#[derive(Debug)]
pub struct PuzzleGeneratorOverrides {
    /// Parameters for the generator.
    pub params: Vec<String>,
    /// Name override.
    pub name: Option<String>,
    /// Additional aliases.
    pub aliases: Vec<String>,
    /// Extra tags.
    pub tags: TagSet,
}
impl FromLua for PuzzleGeneratorOverrides {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let table: LuaTable = lua.unpack(value)?;

        let params: Vec<LuaValue>;
        let name: Option<String>;
        let aliases: LuaVecString;
        let tags: Option<LuaTable>;
        unpack_table!(lua.unpack(table {
            params,
            name,
            aliases,
            tags,
        }));

        let tags = crate::lua::tags::unpack_tags_table(lua, tags)?;

        Ok(PuzzleGeneratorOverrides {
            params: params.into_iter().map(|v| v.to_string()).try_collect()?,
            name,
            aliases: aliases.0,
            tags,
        })
    }
}

use std::{collections::HashMap, fmt, sync::Arc};

use itertools::Itertools;

mod db;

use super::*;
use crate::TagValue;
pub use db::LuaPuzzleGeneratorDb;

/// Specification for a puzzle generator.
#[derive(Debug)]
pub struct PuzzleGeneratorSpec {
    /// String ID of the puzzle generator.
    pub id: String,
    /// Version of the puzzle geneartor.
    pub version: Version,

    /// Default color system.
    pub colors: Option<String>,
    /// Default tags.
    pub tags: HashMap<String, TagValue>,

    /// Puzzle generation parameters.
    pub params: Vec<GeneratorParam>,
    /// Examples and special cases for generated puzzles.
    pub examples: HashMap<String, Arc<PuzzleSpec>>,
    /// Lua function to generate a puzzle definition.
    user_gen_fn: LuaRegistryKey,

    /// User-friendly name for the puzzle generator. (default = same as ID)
    pub name: Option<String>,
    /// Alternative user-friendly names for the puzzle generator.
    pub aliases: Vec<String>,
}

impl<'lua> FromLua<'lua> for PuzzleGeneratorSpec {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let id: String;
        let version: Version;
        let colors: Option<String>;
        let tags: Option<LuaTable<'lua>>;
        let params: Vec<GeneratorParam>;
        let examples: Option<Vec<PuzzleGeneratorExample>>;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let gen: LuaFunction<'lua>;
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

        for tag in tags.keys().filter(|tag| tag.starts_with("type")) {
            lua.warning(format!("generator {id} should not have tag {tag:?}"), false);
        }

        // Add `#generator` tag.
        tags.insert("type/generator".to_owned(), TagValue::True);

        crate::lua::tags::inherit_parent_tags(&mut tags);

        let mut ret = PuzzleGeneratorSpec {
            id: id.clone(),
            version,

            colors,
            tags,

            params,
            examples: HashMap::new(),
            user_gen_fn: lua.create_registry_value(gen)?,

            name,
            aliases: aliases.unwrap_or_default(),
        };

        for example in examples.unwrap_or_default() {
            let generator_param_values = example.params.iter().map(|val| val.to_string()).collect();
            let id = crate::generated_puzzle_id(&id, &generator_param_values);
            match ret.generate_puzzle_spec(lua, generator_param_values, Some(example)) {
                Ok(PuzzleGeneratorOutput::Puzzle(puzzle_spec)) => {
                    ret.examples.insert(puzzle_spec.id.clone(), puzzle_spec);
                }
                Ok(PuzzleGeneratorOutput::Redirect(other)) => {
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

impl PuzzleGeneratorSpec {
    /// Returns the name or the ID of the puzzle.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Runs user Lua code to generate a puzzle _definition_.
    pub fn generate_puzzle_spec<'lua>(
        &self,
        lua: &'lua Lua,
        generator_param_values: Vec<String>,
        matching_example: Option<PuzzleGeneratorExample>,
    ) -> LuaResult<PuzzleGeneratorOutput> {
        let id = crate::generated_puzzle_id(&self.id, &generator_param_values);

        if let Some(puzzle_spec) = self.examples.get(&id) {
            return Ok(PuzzleGeneratorOutput::Puzzle(Arc::clone(&puzzle_spec)));
        }

        let expected = self.params.len();
        let got = generator_param_values.len();
        if expected != got {
            let generator_id = &self.id;
            return Err(LuaError::external(format!(
                "generator {generator_id} expects {expected} params; got {got}",
            )));
        }

        let params: Vec<GeneratorParamValue> = std::iter::zip(&self.params, generator_param_values)
            .map(|(p, s)| p.value_from_str(&s))
            .try_collect()?;
        let gen_params_table = lua.create_sequence_from(params.clone())?;

        let user_gen_fn_output = lua
            .registry_value::<LuaFunction<'_>>(&self.user_gen_fn)?
            .call::<_, LuaMultiValue<'_>>(gen_params_table)
            .context("error generating puzzle definition")?
            .into_vec();

        let generated_spec = match user_gen_fn_output.get(0) {
            Some(LuaValue::String(s)) => {
                let redirect_id = s.to_string_lossy();
                return Ok(PuzzleGeneratorOutput::Redirect(
                    if let Some(val) = user_gen_fn_output.get(1) {
                        let redirect_params =
                            LuaSequence::<GeneratorParamValue>::from_lua(val.clone(), lua)?.0;
                        crate::generated_puzzle_id(&redirect_id, redirect_params)
                    } else {
                        redirect_id.into_owned()
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
            // Set dummy ID (can't use the real ID because it isn't a valid ID).
            if puzzle_spec_table.contains_key("id")? {
                lua.warning("overwriting `id` outputted by puzzle generator", false);
            }
            puzzle_spec_table.raw_set("id", "_")?;

            // Set version.
            if puzzle_spec_table.contains_key("version")? {
                lua.warning("overwriting `version` outputted by puzzle generator", false);
            }
            puzzle_spec_table.raw_set("version", self.version)?;

            // Set color system, if it isn't already set.
            if !puzzle_spec_table.contains_key("colors")? {
                puzzle_spec_table.raw_set("colors", self.colors.as_deref())?;
            }
        }

        // Generate the puzzle spec.
        let mut puzzle_spec = PuzzleSpec::from_lua(puzzle_spec_table.into_lua(lua)?, lua)?;

        // Now we can manually overwrite the ID.
        puzzle_spec.id = id;

        // Add data from the matching example.
        if let Some(example) = matching_example {
            let PuzzleGeneratorExample {
                params: _,
                name,
                aliases,
                tags,
            } = example;

            // Set name from the example, and move the auto-generated name to an
            // alias.
            if let Some(new_name) = name.clone() {
                let old_name = puzzle_spec.name.replace(new_name);
                puzzle_spec.aliases.extend(old_name);
            }

            // Add aliases from the example.
            puzzle_spec.aliases.extend(aliases);

            // Add tags from the example, and these tags should have the highest
            // priority.
            let tags_from_example = tags;
            let tags_from_puzzle_spec = std::mem::take(&mut puzzle_spec.tags);
            puzzle_spec.tags =
                crate::lua::tags::merge_tag_sets(tags_from_example, tags_from_puzzle_spec);
        }

        // Add tags from generator.
        let tags_from_generator = self.tags.iter().map(|(k, v)| (k.clone(), v.clone()));
        puzzle_spec.tags = crate::lua::tags::merge_tag_sets(puzzle_spec.tags, tags_from_generator);

        // Remove `#generator` tag.
        puzzle_spec.tags.remove("type/generator");

        // Add `#generated` tag.
        puzzle_spec
            .tags
            .insert("generated".to_owned(), TagValue::True);

        crate::lua::tags::inherit_parent_tags(&mut puzzle_spec.tags);

        Ok(PuzzleGeneratorOutput::Puzzle(Arc::new(puzzle_spec)))
    }
}

/// Parameter for a generated puzzle.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorParam {
    /// Human-friendly name.
    pub name: String,
    /// Parameter type.
    pub ty: GeneratorParamType,
    /// Default value.
    pub default: GeneratorParamValue,
}
impl GeneratorParam {
    /// Converts a Lua value to a value for this parameter and returns an error
    /// if it is invalid.
    pub fn value_from_lua<'lua>(
        &self,
        lua: &'lua Lua,
        mut value: LuaValue<'lua>,
    ) -> LuaResult<GeneratorParamValue> {
        if value.is_nil() {
            value = self.default.clone().into_lua(lua)?;
        }
        match self.ty {
            GeneratorParamType::Int { min, max } => {
                let name = &self.name;
                let i = i64::from_lua(value, lua)?;
                if i > max {
                    return Err(LuaError::external(format!(
                        "value {i:?} for parameter {name:?} is greater than {max}"
                    )));
                }
                if i < min {
                    return Err(LuaError::external(format!(
                        "value {i:?} for parameter {name:?} is less than {min}"
                    )));
                }
                Ok(GeneratorParamValue::Int(i))
            }
        }
    }

    /// Converts a string to a value for this parameter and returns an error if
    /// it is invalid.
    pub fn value_from_str<'lua>(&self, s: &str) -> LuaResult<GeneratorParamValue> {
        if s.is_empty() {
            return Ok(self.default.clone());
        }
        match self.ty {
            GeneratorParamType::Int { .. } => Ok(GeneratorParamValue::Int(
                s.parse().map_err(LuaError::external)?,
            )),
        }
    }
}
impl<'lua> FromLua<'lua> for GeneratorParam {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let name: String;
        let r#type: String;
        let default: GeneratorParamValue;
        let min: Option<i64>;
        let max: Option<i64>;
        unpack_table!(lua.unpack(table {
            name,
            r#type,
            default,

            min,
            max,
        }));

        let ty = match r#type.as_str() {
            "int" => {
                let min = min.ok_or_else(|| LuaError::external("`int` type requires `min`"))?;
                let max = max.ok_or_else(|| LuaError::external("`int` type requires `max`"))?;
                GeneratorParamType::Int { min, max }
            }
            s => return Err(LuaError::external(format!("unknown parameter type {s:?}"))),
        };

        Ok(GeneratorParam { name, ty, default })
    }
}

/// Type of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorParamType {
    /// Integer.
    Int {
        /// Minimum value (inclusive).
        min: i64,
        /// Maximum value (inclusive).
        max: i64,
    },
}

/// Value of a parameter for a puzzle generator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeneratorParamValue {
    /// Integer.
    Int(i64),
}
impl<'lua> FromLua<'lua> for GeneratorParamValue {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        None.or_else(|| lua.unpack(value).map(Self::Int).ok())
            .ok_or_else(|| LuaError::external("invalid generator parameter"))
    }
}
impl<'lua> IntoLua<'lua> for GeneratorParamValue {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            GeneratorParamValue::Int(i) => i.into_lua(lua),
        }
    }
}
impl fmt::Display for GeneratorParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeneratorParamValue::Int(i) => write!(f, "{i}"),
        }
    }
}

/// Example of a generated puzzle, which can defined overrides for certain
/// fields of the [`PuzzleSpec`].
#[derive(Debug)]
pub struct PuzzleGeneratorExample {
    /// Parameters for the generator.
    pub params: Vec<GeneratorParamValue>,
    /// Name override.
    pub name: Option<String>,
    /// Additional aliases.
    pub aliases: Vec<String>,
    /// Extra tags.
    pub tags: HashMap<String, TagValue>,
}
impl<'lua> FromLua<'lua> for PuzzleGeneratorExample {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let params: Vec<GeneratorParamValue>;
        let name: Option<String>;
        let aliases: LuaVecString;
        let tags: Option<LuaTable<'lua>>;
        unpack_table!(lua.unpack(table {
            params,
            name,
            aliases,
            tags,
        }));

        let tags = crate::lua::tags::unpack_tags_table(lua, tags)?;

        Ok(PuzzleGeneratorExample {
            params,
            name,
            aliases: aliases.0,
            tags,
        })
    }
}

/// Output of a puzzle generator.
pub enum PuzzleGeneratorOutput {
    /// Puzzle parameters.
    Puzzle(Arc<PuzzleSpec>),
    /// Redirect to a different puzzle ID.
    Redirect(String),
}

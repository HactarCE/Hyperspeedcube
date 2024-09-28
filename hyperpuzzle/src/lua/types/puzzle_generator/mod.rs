use std::{collections::HashMap, fmt, sync::Arc};

use itertools::Itertools;

use super::*;

mod db;

pub use db::LuaPuzzleGeneratorDb;

/// Maximum recursion depth for the puzzle metadata table.
const MAX_METADATA_TABLE_RECURSION_DEPTH: usize = 5;

/// Set of parameters that define a puzzle.
#[derive(Debug)]
pub struct PuzzleGenerator {
    /// String ID of the puzzle generator.
    pub id: String,
    /// Version of the puzzle geneartor.
    pub version: Version,

    /// Puzzle generation parameters.
    pub params: Vec<GeneratorParam>,
    /// Examples and special cases for generated puzzles.
    pub examples: HashMap<String, Arc<PuzzleParams>>,
    /// Lua function to generate a puzzle definition.
    user_gen_fn: LuaRegistryKey,

    /// User-friendly name for the puzzle generator. (default = same as ID)
    pub name: Option<String>,
    /// Alternative user-friendly names for the puzzle generator.
    pub aliases: Vec<String>,
    /// Lua table containing metadata about the puzzle.
    pub meta: Option<LuaRegistryKey>,
}

impl<'lua> FromLua<'lua> for PuzzleGenerator {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let id: String;
        let version: Version;
        let params: Vec<GeneratorParam>;
        let examples: Option<Vec<PuzzleGeneratorExample>>;
        let name: Option<String>;
        let aliases: Option<Vec<String>>;
        let meta: Option<LuaTable<'lua>>;
        let gen: LuaFunction<'lua>;
        unpack_table!(lua.unpack(table {
            id,
            version,

            params,
            examples,
            gen,

            name,
            aliases,
            meta,
        }));

        let id = crate::validate_id(id).into_lua_err()?;

        let mut ret = PuzzleGenerator {
            id: id.clone(),
            version,

            params,
            examples: HashMap::new(),
            user_gen_fn: lua.create_registry_value(gen)?,

            name,
            aliases: aliases.unwrap_or_default(),
            meta: crate::lua::create_opt_registry_value(lua, meta)?,
        };

        for example in examples.unwrap_or_default() {
            let generator_param_values = example.params.iter().map(|val| val.to_string()).collect();
            let id = crate::generated_puzzle_id(&id, &generator_param_values);
            match ret.generate_puzzle_params(lua, generator_param_values, example.meta) {
                Ok(PuzzleGeneratorOutput::Puzzle(puzzle_params)) => {
                    ret.examples.insert(puzzle_params.id.clone(), puzzle_params);
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

impl PuzzleGenerator {
    /// Returns the name or the ID of the puzzle.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Runs user Lua code to generate a puzzle _definition_.
    pub fn generate_puzzle_params<'lua>(
        &self,
        lua: &'lua Lua,
        generator_param_values: Vec<String>,
        extra_metadata: Option<LuaTable<'lua>>,
    ) -> LuaResult<PuzzleGeneratorOutput> {
        let id = crate::generated_puzzle_id(&self.id, &generator_param_values);

        if let Some(puzzle_params) = self.examples.get(&id) {
            return Ok(PuzzleGeneratorOutput::Puzzle(Arc::clone(&puzzle_params)));
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
            .call(gen_params_table)
            .context("error generating puzzle definition")?;

        let puzzle_params = match user_gen_fn_output {
            LuaValue::String(s) => {
                return Ok(PuzzleGeneratorOutput::Redirect(
                    s.to_string_lossy().into_owned(),
                ))
            }
            LuaValue::Table(tab) => tab,
            _ => return Err(LuaError::external(
                "return value of `gen` function must string (ID redirect) or table (puzzle params)",
            )),
        };

        // Add metadata from a matching example, if there is one.
        if let Some(meta) = extra_metadata {
            let t = lua.create_table_from([("meta", meta)])?;
            augment_table(lua, &puzzle_params, &t, MAX_METADATA_TABLE_RECURSION_DEPTH)?;
        }

        // Add keys from generator.
        let t = lua.create_table_from([
            ("id", id.into_lua(lua)?),
            ("version", self.version.into_lua(lua)?),
            ("__generated__", true.into_lua(lua)?),
        ])?;
        augment_table(lua, &puzzle_params, &t, 1)?;

        Ok(PuzzleGeneratorOutput::Puzzle(Arc::new(
            PuzzleParams::from_lua(puzzle_params.into_lua(lua)?, lua)?,
        )))
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorParamType {
    Int { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeneratorParamValue {
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

#[derive(Debug, Default)]
pub struct PuzzleGeneratorExample<'lua> {
    pub params: Vec<GeneratorParamValue>,
    pub meta: Option<LuaTable<'lua>>,
}
impl<'lua> FromLua<'lua> for PuzzleGeneratorExample<'lua> {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table: LuaTable<'lua> = lua.unpack(value)?;

        let params: Vec<GeneratorParamValue>;
        let meta: Option<LuaTable<'_>>;
        unpack_table!(lua.unpack(table { params, meta }));

        Ok(PuzzleGeneratorExample { params, meta })
    }
}

pub enum PuzzleGeneratorOutput {
    /// Puzzle parameters.
    Puzzle(Arc<PuzzleParams>),
    /// Redirect to a different puzzle ID.
    Redirect(String),
}

fn augment_table<'lua>(
    lua: &'lua Lua,
    destination: &LuaTable<'lua>,
    source: &LuaTable<'lua>,
    max_depth: usize,
) -> LuaResult<()> {
    let Some(new_depth) = max_depth.checked_sub(1) else {
        return Err(LuaError::external("recursion limit exceeded"));
    };

    for pair in source.clone().pairs::<LuaValue<'_>, LuaValue<'_>>() {
        let (k, v) = pair?;
        if let (Ok(dst), LuaValue::Table(src)) =
            (destination.raw_get::<_, LuaTable<'_>>(k.clone()), &v)
        {
            // Both values are tables; recurse!
            augment_table(lua, &dst, src, new_depth)?;
        } else {
            if destination.contains_key(k.clone())? {
                lua.warning(format!("overwriting key {k:?}"), false);
            }
            destination.raw_set(k, v)?;
        }
    }

    Ok(())
}

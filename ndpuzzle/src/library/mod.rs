use std::collections::HashMap;
use std::sync::Arc;

#[macro_use]
mod lua;

use anyhow::{Context, Result};
use rlua::prelude::*;
use tinyset::Set64;

use crate::puzzle::PuzzleType;
pub use lua::types::{LuaFileLoadError, LuaLogLine};
use lua::types::{LuaShapeSet, LuaSpace};

/// Library of loaded puzzles, shapes, etc.
#[derive(Debug)]
pub struct PuzzleLibrary {
    lua: Lua,
    files: HashMap<String, String>,
}
impl PuzzleLibrary {
    pub fn new() -> Self {
        Self {
            lua: lua::new_lua(),
            files: HashMap::new(),
        }
    }

    pub fn load_file(
        &mut self,
        filename: String,
        contents: String,
    ) -> Result<(), LuaFileLoadError> {
        let result = lua::load_sandboxed(&self.lua, &filename, &contents);
        println!("loaded file!");
        if result.is_ok() {
            self.files.insert(filename, contents);
            println!("good");
        }
        result
    }

    pub fn drain_log(&mut self) -> Vec<LuaLogLine> {
        self.lua.context(lua::drain_logs)
    }

    pub fn puzzle_names(&self) -> Result<Vec<String>> {
        self.lua.context(|lua| {
            Ok(lua
                .globals()
                .get::<_, LuaTable>("library")?
                .get::<_, LuaTable>("objects")?
                .pairs::<String, LuaTable>()
                .filter_map(LuaResult::ok)
                .filter(|(_, v)| {
                    v.get::<_, String>("type")
                        .is_ok_and(|type_str| type_str == "puzzle")
                })
                .map(|(k, _)| k)
                .collect())
        })
    }

    pub fn build_puzzle(
        &self,
        puzzle_name: String,
    ) -> Result<(LuaResult<Arc<PuzzleType>>, Vec<LuaLogLine>)> {
        // Figure out which file we need to load.
        let filename = match self.lua.context(|lua| {
            lua.globals()
                .get::<_, LuaTable>("library")?
                .get::<_, LuaTable>("objects")?
                .get::<_, LuaTable>(puzzle_name.as_str())?
                .get::<_, String>("filename")
        }) {
            Ok(filename) => filename,
            Err(_) => {
                return Ok((
                    Err(LuaError::external(format!(
                        "object puzzle/{puzzle_name:?} not found"
                    ))),
                    vec![],
                ));
            }
        };

        let lua = lua::new_lua();

        if let Err(e) = lua::load_sandboxed(
            &lua,
            &filename,
            self.files
                .get(&filename)
                .context("loading file from cache")?,
        ) {
            return Ok((
                Err(match e {
                    LuaFileLoadError::MissingDependencies(deps) => {
                        LuaError::external(format!("missing dependencies: {}", deps.join(", ")))
                    }
                    LuaFileLoadError::UserError(e) => e,
                    LuaFileLoadError::InternalError(e) => Err(e)?,
                }),
                lua.context(lua::drain_logs),
            ));
        };

        lua.context(|lua| {
            let puzzle_spec = lua
                .globals()
                .get::<_, LuaTable>("library")?
                .get::<_, LuaTable>("objects")?
                .get::<_, LuaTable>(puzzle_name.as_str())?;
            let ndim = match puzzle_spec.get::<_, u8>("ndim") {
                Ok(ndim) if 2 <= ndim && ndim <= 8 => ndim,
                _ => {
                    return Ok((
                        Err(LuaError::external(
                            "puzzle requires an `ndim` property between 2 and 8",
                        )),
                        lua::drain_logs(lua),
                    ));
                }
            };

            let build_fn = match puzzle_spec.get::<_, LuaFunction>("build") {
                Ok(f) => f,
                _ => {
                    return Ok((
                        Err(LuaError::external(
                            "puzzle requires a `build` property that is a function",
                        )),
                        lua::drain_logs(lua),
                    ));
                }
            };

            let space = LuaSpace::new(ndim);
            let initial = LuaShapeSet(Set64::from_iter([space.0.lock().roots()[0].into()]));

            lua.globals().set("SPACE", space.clone())?;
            lua.globals().set("NDIM", ndim)?;

            Ok((
                build_fn
                    .call::<LuaShapeSet, LuaShapeSet>(initial)
                    .and_then(|root_shapes| {
                        let pieces = root_shapes.0.into_iter().collect();
                        PuzzleType::create_puzzle_type_from_shapes(
                            puzzle_name,
                            &space.0.lock(),
                            pieces,
                        )
                        .map_err(LuaError::external)
                    }),
                lua::drain_logs(lua),
            ))
        })
    }
}

/// Metadata about a puzzle, including its name, description, relation to other
/// puzzles, and data needed to construct it.
pub struct PuzzleTypeInfo {
    name: String,
    tags: Vec<String>,
}

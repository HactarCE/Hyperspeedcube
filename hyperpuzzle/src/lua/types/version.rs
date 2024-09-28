use std::fmt;

use super::*;

/// Semantic-ish version for a puzzle or puzzle generator.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch version number.
    pub patch: u32,
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major,
            minor,
            patch,
        } = self;
        write!(f, "{major}.{minor}.{patch}")
    }
}
/// Parses a basic semver string, where minor and patch versions are optional.
impl<'lua> FromLua<'lua> for Version {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let Ok(version_string) = String::from_lua(value, lua) else {
            lua.warning(format!("expected version string"), false);
            return Ok(Self::default());
        };

        fn parse_component(s: &str) -> Result<u32, String> {
            s.parse()
                .map_err(|e| format!("invalid major version because {e}"))
        }

        // IIFE to mimic try_block
        let result = (|| {
            let mut segments = version_string.split('.');
            let version = Self {
                major: parse_component(segments.next().ok_or("missing major version")?)?,
                minor: parse_component(segments.next().unwrap_or("0"))?,
                patch: parse_component(segments.next().unwrap_or("0"))?,
            };
            if segments.next().is_some() {
                return Err(
                    "too many segments; only the form `major.minor.patch` is accepted".to_owned(),
                );
            }
            Ok(version)
        })();

        match result {
            Ok(version) => Ok(version),
            Err(e) => {
                lua.warning(format!("error parsing version string: {e}"), false);
                Ok(Self::default())
            }
        }
    }
}
impl<'lua> IntoLua<'lua> for Version {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.to_string().into_lua(lua)
    }
}

use hyperpuzzle_core::Version;

use super::*;

/// Conversion wrapper for a basic semver string, where minor and patch versions
/// are optional.
pub struct LuaVersion(pub Version);
impl FromLua for LuaVersion {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let Ok(version_string) = String::from_lua(value, lua) else {
            lua.warning("expected version string", false);
            return Ok(Self(Version::default()));
        };

        fn parse_component(s: &str) -> Result<u32, String> {
            s.parse()
                .map_err(|e| format!("invalid major version because {e}"))
        }

        // IIFE to mimic try_block
        let result = (|| {
            let mut segments = version_string.split('.');
            let version = Version {
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
            Ok(version) => Ok(Self(version)),
            Err(e) => {
                lua.warning(format!("error parsing version string: {e}"), false);
                Ok(Self(Version::default()))
            }
        }
    }
}
impl IntoLua for LuaVersion {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        self.0.to_string().into_lua(lua)
    }
}

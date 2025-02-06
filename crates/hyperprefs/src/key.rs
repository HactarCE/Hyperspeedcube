use std::fmt;
use std::str::FromStr;

use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer, Serialize};
use winit::keyboard::{Key, NativeKey, NativeKeyCode, PhysicalKey};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyKey {
    Virtual(Key),
    Physical(PhysicalKey),
}

impl fmt::Display for AnyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AnyKey::Virtual(key) => key_names::key_name(key.clone()),
            AnyKey::Physical(physical_key) => key_names::physical_key_name(*physical_key),
        };
        write!(f, "{s}")
    }
}

impl Serialize for AnyKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&match self {
            AnyKey::Virtual(key) => match key {
                Key::Named(named_key) => format!("key_{}", serialize_to_string(named_key)?),
                Key::Character(s) => format!("char_{s}"),

                Key::Unidentified(native_key) => match native_key {
                    NativeKey::Unidentified => format!("unidentified"),
                    NativeKey::Android(sc) => format!("android_{sc}"),
                    NativeKey::MacOS(sc) => format!("macos_{sc}"),
                    NativeKey::Windows(sc) => format!("windows_{sc}"),
                    NativeKey::Xkb(sc) => format!("xkb_{sc}"),
                    NativeKey::Web(s) => format!("web_{s}"),
                },

                Key::Dead(None) => format!("dead"),
                Key::Dead(Some(c)) => format!("dead_{c}"),
            },

            AnyKey::Physical(physical_key) => match physical_key {
                PhysicalKey::Code(key_code) => format!("code_{}", serialize_to_string(key_code)?),

                PhysicalKey::Unidentified(native_key) => match native_key {
                    NativeKeyCode::Unidentified => format!("native_unidentified"),
                    NativeKeyCode::Android(sc) => format!("native_android_{sc}"),
                    NativeKeyCode::MacOS(sc) => format!("native_macos_{sc}"),
                    NativeKeyCode::Windows(sc) => format!("native_windows_{sc}"),
                    NativeKeyCode::Xkb(sc) => format!("native_xkb_{sc}"),
                },
            },
        })
    }
}

impl<'de> Deserialize<'de> for AnyKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        Ok(match s.split_once('_').unwrap_or((&s, "")) {
            ("key", named_key) => AnyKey::Virtual(Key::Named(deserialize_from_string(named_key)?)),
            ("char", s) => AnyKey::Virtual(Key::Character(s.into())),

            ("unidentified", _) => AnyKey::Virtual(Key::Unidentified(NativeKey::Unidentified)),
            ("android", sc) => AnyKey::Virtual(Key::Unidentified(NativeKey::Android(parse(sc)?))),
            ("macos", sc) => AnyKey::Virtual(Key::Unidentified(NativeKey::MacOS(parse(sc)?))),
            ("windows", sc) => AnyKey::Virtual(Key::Unidentified(NativeKey::Windows(parse(sc)?))),
            ("xkb", sc) => AnyKey::Virtual(Key::Unidentified(NativeKey::Xkb(parse(sc)?))),
            ("web", s) => AnyKey::Virtual(Key::Unidentified(NativeKey::Web(s.into()))),

            ("dead", s) => AnyKey::Virtual(Key::Dead(s.chars().next())),

            ("code", key_code) => {
                AnyKey::Physical(PhysicalKey::Code(deserialize_from_string(key_code)?))
            }
            ("native", rest) => AnyKey::Physical(PhysicalKey::Unidentified(
                match rest.split_once('_').unwrap_or((rest, "")) {
                    ("unidentified", _) => NativeKeyCode::Unidentified,
                    ("android", sc) => NativeKeyCode::Android(parse(sc)?),
                    ("macos", sc) => NativeKeyCode::MacOS(parse(sc)?),
                    ("windows", sc) => NativeKeyCode::Windows(parse(sc)?),
                    ("xkb", sc) => NativeKeyCode::Xkb(parse(sc)?),
                    (prefix, _) => {
                        let msg = format!("unknown native key prefix {prefix:?}");
                        return Err(D::Error::custom(msg));
                    }
                },
            )),

            (prefix, _) => {
                let msg = format!("unknown key prefix {prefix:?}");
                return Err(D::Error::custom(msg));
            }
        })
    }
}

fn serialize_to_string<T: Serialize, E: serde::ser::Error>(value: &T) -> Result<String, E> {
    Ok(serde_json::to_value(value)
        .map_err(E::custom)?
        .as_str()
        .ok_or_else(|| E::custom("expected string"))?
        .to_owned())
}

fn deserialize_from_string<T: DeserializeOwned, E: serde::de::Error>(s: &str) -> Result<T, E> {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).map_err(E::custom)
}

fn parse<T: FromStr, E: serde::de::Error>(s: &str) -> Result<T, E>
where
    T::Err: fmt::Display,
{
    s.parse().map_err(E::custom)
}

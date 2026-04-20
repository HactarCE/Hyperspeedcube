//! ID string for an object in a catalog.

use std::{fmt, str::FromStr};

use chumsky::prelude::*;
use serde::{Deserialize, Serialize, de};

/// Error produced when parsing a catalog ID.
pub type CatalogIdParseError = Rich<'static, char, SimpleSpan>;

/// ID string for an object in a catalog.
///
/// ## Examples
///
/// ```
/// # use hyperpuzzle_core::CatalogId;
/// assert_eq!(
///     CatalogId::from_str("megaminx_crystal").unwrap(),
///     CatalogId::new("megaminx_crystal", []).unwrap(),
/// );
///
/// assert_eq!(
///     CatalogId::from_str("product(ft_ngon(7,3),line(3))").unwrap(),
///     CatalogId::new(
///         "product",
///         [
///             CatalogId::new("ft_ngon", [7.into(), 3.into()]).unwrap(),
///             CatalogId::new("line", [3.into()]).unwrap(),
///         ]
///     )
///     .unwrap(),
/// );
/// ```
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CatalogId {
    /// Base string.
    pub base: Box<str>,
    /// Argument values, if the base string specifies a generator.
    pub args: Vec<CatalogArgValue>,
}

impl fmt::Debug for CatalogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string(), f)
    }
}

impl fmt::Display for CatalogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { base, args } = self;
        write!(f, "{base}")?;
        if !args.is_empty() {
            write!(f, "(")?;
            let mut is_first = true;
            for arg in args {
                if !std::mem::take(&mut is_first) {
                    write!(f, ",")?;
                }
                write!(f, "{arg}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl FromStr for CatalogId {
    type Err = CatalogIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        recursive::<_, _, extra::Err<Rich<'_, char, SimpleSpan>>, _, _>(|id_or_parameter| {
            let base = any()
                .filter(|&c| is_id_base_char(c))
                .repeated()
                .at_least(1)
                .to_slice()
                .map(Box::from);
            base.then(
                id_or_parameter
                    .map(CatalogArgValue)
                    .separated_by(just(','))
                    .collect()
                    .delimited_by(just('('), just(')'))
                    .or_not()
                    .map(Option::<Vec<_>>::unwrap_or_default),
            )
            .map(|(base, args)| CatalogId { base, args })
            .boxed()
        })
        .parse(s)
        .into_result()
        .map_err(|errors| {
            errors
                .into_iter()
                .next()
                .expect("parse failed with no errors")
                .into_owned()
        })
    }
}

impl Serialize for CatalogId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CatalogId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl CatalogId {
    /// Constructs a new catalog ID. Returns `None` if the ID is invalid.
    ///
    /// Prefer [`CatalogId::from_str()`] when parsing an ID from an external
    /// source because it performs validation.
    pub fn new(
        base: impl Into<Box<str>>,
        args: impl IntoIterator<Item = CatalogArgValue>,
    ) -> Option<Self> {
        let base = base.into();
        if base.is_empty() || base.chars().any(|c| !is_id_base_char(c)) {
            return None;
        }
        let args = args.into_iter().collect();
        Some(Self { base, args })
    }

    /// Returns a catalog ID for an unnamed object.
    pub fn unnamed() -> Self {
        Self {
            base: "unnamed".into(),
            args: vec![],
        }
    }
}

fn is_id_base_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit() || c == '_' || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_id_roundtrip() {
        for s in ["product(ft_ngon(7,3),line(3))", "megaminx_crystal"] {
            assert_eq!(s, CatalogId::from_str(s).unwrap().to_string());
        }
    }
}

/// Argument to a generator parameter.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CatalogArgValue(CatalogId); // stored as `CatalogId` only to avoid quadratic reparsing ofs nested parens

impl fmt::Debug for CatalogArgValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for CatalogArgValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for CatalogArgValue {
    type Err = CatalogIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl From<CatalogId> for CatalogArgValue {
    fn from(value: CatalogId) -> Self {
        Self(value)
    }
}

impl From<i64> for CatalogArgValue {
    fn from(value: i64) -> Self {
        Self(CatalogId {
            base: value.to_string().into(),
            args: vec![],
        })
    }
}

impl CatalogArgValue {
    /// Interprets the argument as a boolean, or returns `None` if it is not a
    /// valid boolean.
    pub fn to_bool(&self) -> Option<bool> {
        if self.0.args.is_empty() {
            match &*self.0.base {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Interprets the argument as a signed integer, or returns `None` if it is
    /// not a valid signed integer.
    pub fn to_int(&self) -> Option<i64> {
        if self.0.args.is_empty() {
            self.0.base.parse().ok()
        } else {
            None
        }
    }

    /// Interprets the argument as a catalog ID.
    pub fn to_id(&self) -> CatalogId {
        self.0.clone()
    }
}

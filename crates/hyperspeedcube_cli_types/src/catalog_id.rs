//! ID string for an object in a catalog.

use std::str::FromStr;
use std::{fmt, ops::Deref};

use chumsky::prelude::*;
use serde::{Deserialize, Serialize, de};

/// Alphanumeric (with underscores) string, used in [`CatalogId`] and
/// [`CatalogIdValue`].
///
/// This type dereferences to [`str`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CatalogIdent(Box<str>);

impl fmt::Debug for CatalogIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for CatalogIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for CatalogIdent {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CatalogIdError::Empty);
        }
        if let Some(c) = s.chars().find(|&c| !is_id_ident_char(c)) {
            return Err(CatalogIdError::BadChar(c));
        }
        Ok(Self(Box::from(s)))
    }
}

impl Deref for CatalogIdent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl PartialOrd for CatalogIdent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CatalogIdent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        numeric_sort::cmp(self, other)
    }
}

impl From<bool> for CatalogIdent {
    fn from(value: bool) -> Self {
        Self(value.to_string().into_boxed_str())
    }
}

impl From<i64> for CatalogIdent {
    fn from(value: i64) -> Self {
        Self(value.to_string().into_boxed_str())
    }
}

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
    pub base: CatalogIdent,
    /// Argument values, if the base string specifies a generator.
    pub args: Vec<CatalogIdValue>,
    /// Optional subset.
    pub subset: Option<CatalogIdent>,
}

impl fmt::Debug for CatalogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_string(), f)
    }
}

impl fmt::Display for CatalogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { base, args, subset } = self;
        write!(f, "{base}")?;
        if !args.is_empty() {
            write!(f, "(")?;
            write_comma_sep_list(f, args)?;
            write!(f, ")")?;
        }
        if let Some(s) = subset {
            write!(f, ".{s}")?;
        }
        Ok(())
    }
}

impl FromStr for CatalogId {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CatalogIdValue::from_str(s)?.try_into()
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
    /// Constructs a new catalog ID.
    pub fn new(
        base: CatalogIdent,
        args: impl IntoIterator<Item = CatalogIdValue>,
        subset: Option<CatalogIdent>,
    ) -> Self {
        let args = args.into_iter().collect();
        Self { base, args, subset }
    }

    /// Returns a catalog ID for an unnamed object.
    pub fn unnamed() -> Self {
        Self {
            base: "unnamed".parse().expect("invalid ID"),
            args: vec![],
            subset: None,
        }
    }

    /// If `self` is a pattern that matches `other`, returns the values in
    /// `other` that fill the wildcards in `self`. Returns `None` if the IDs do
    /// not match.
    pub fn match_wildcards(&self, other: &CatalogId) -> Option<Vec<CatalogIdValue>> {
        let mut buf = vec![];
        self.match_wildcards_into(other, &mut buf).then_some(buf)
    }

    #[must_use]
    fn match_wildcards_into(&self, other: &Self, output_buffer: &mut Vec<CatalogIdValue>) -> bool {
        self.base == other.base
            && self.args.len() == other.args.len()
            && self.subset == other.subset
            && std::iter::zip(&self.args, &other.args)
                .all(|(pattern, value)| pattern.match_wildcards_into(value, output_buffer))
    }
}

/// Abstract syntax tree node for a [`CatalogId`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CatalogIdValue {
    /// Identifier, which typically represents a primitive parameter value (such
    /// as an integer) or a catalog object.
    Ident(CatalogIdent),
    /// Generator invocation.
    Generator(CatalogId),
    /// List of parameter values to a generator.
    List(Vec<CatalogIdValue>),
    /// Wildcard, which is represented by `*`.
    Wildcard,
    /// Error value, which is represented by `!`.
    #[default]
    Error,
}

impl fmt::Display for CatalogIdValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogIdValue::Ident(id) => fmt::Display::fmt(id, f),
            CatalogIdValue::Generator(id) => fmt::Display::fmt(id, f),
            CatalogIdValue::List(elems) => {
                write!(f, "[")?;
                write_comma_sep_list(f, elems)?;
                write!(f, "]")?;
                Ok(())
            }
            CatalogIdValue::Wildcard => write!(f, "*"),
            CatalogIdValue::Error => write!(f, "!"),
        }
    }
}

impl FromStr for CatalogIdValue {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        recursive::<_, _, extra::Err<Rich<'_, char, SimpleSpan>>, _, _>(|ast_node| {
            let ident = any()
                .filter(|&c| is_id_ident_char(c))
                .repeated()
                .at_least(1)
                .to_slice()
                .map(Box::from)
                .map(CatalogIdent);
            let id = ident
                .clone()
                .then(
                    ast_node
                        .clone()
                        .separated_by(just(','))
                        .collect()
                        .delimited_by(just('('), just(')'))
                        .or_not(),
                )
                .then(just('.').ignore_then(ident).or_not())
                .map(|((base, args), subset)| {
                    if args.is_none() && subset.is_none() {
                        Self::Ident(base)
                    } else {
                        Self::Generator(CatalogId {
                            base,
                            args: args.unwrap_or_default(),
                            subset,
                        })
                    }
                });

            let list = ast_node
                .separated_by(just(','))
                .collect()
                .delimited_by(just('['), just(']'))
                .map(Self::List);

            choice((
                id,
                list,
                just('*').to(Self::Wildcard),
                just('!').to(Self::Error),
            ))
            .boxed()
        })
        .parse(s)
        .into_result()
        .map_err(|errors| {
            CatalogIdError::ParseError(
                errors
                    .into_iter()
                    .next()
                    .expect("parse failed with no errors")
                    .into_owned(),
            )
        })
    }
}

impl Serialize for CatalogIdValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CatalogIdValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl CatalogIdValue {
    fn expected_got(&self, expected: &'static str) -> CatalogIdError {
        let got = match self {
            CatalogIdValue::Ident(_) => "identifier",
            CatalogIdValue::Generator(_) => "generator",
            CatalogIdValue::List(_) => "list",
            CatalogIdValue::Wildcard => "wildcard '*'",
            CatalogIdValue::Error => "error '!'",
        };
        CatalogIdError::ExpectedGot { expected, got }
    }

    /// Parses the value into a catalog ID.
    pub fn into_id(self) -> Result<CatalogId, CatalogIdError> {
        match self {
            CatalogIdValue::Ident(base) => Ok(CatalogId {
                base,
                args: vec![],
                subset: None,
            }),
            CatalogIdValue::Generator(id) => Ok(id),
            _ => Err(self.expected_got("id")),
        }
    }
    /// Parses the value into a list.
    pub fn into_list(self) -> Result<Vec<CatalogIdValue>, CatalogIdError> {
        match self {
            CatalogIdValue::List(list) => Ok(list),
            _ => Err(self.expected_got("list")),
        }
    }
    /// Parses the value into a boolean.
    pub fn to_bool(&self) -> Result<bool, CatalogIdError> {
        match self {
            CatalogIdValue::Ident(base) => Ok(base.parse()?),
            _ => Err(self.expected_got("boolean")),
        }
    }
    /// Parses the value into an integer.
    pub fn to_int(&self) -> Result<i64, CatalogIdError> {
        match self {
            CatalogIdValue::Ident(base) => Ok(base.parse()?),
            _ => Err(self.expected_got("integer")),
        }
    }

    fn match_wildcards_into(&self, other: &Self, output_buffer: &mut Vec<Self>) -> bool {
        match (self, other) {
            (CatalogIdValue::Ident(_), _) => self == other,

            (CatalogIdValue::Generator(a), CatalogIdValue::Generator(b)) => {
                a.match_wildcards_into(b, output_buffer)
            }
            (CatalogIdValue::Generator(_), _) => false,

            (CatalogIdValue::List(a), CatalogIdValue::List(b)) => {
                a.len() == b.len()
                    && std::iter::zip(a, b)
                        .all(|(pattern, value)| pattern.match_wildcards_into(value, output_buffer))
            }
            (CatalogIdValue::List(_), _) => false,

            (CatalogIdValue::Wildcard, _) => {
                output_buffer.push(other.clone());
                true
            }

            (CatalogIdValue::Error, _) => false,
        }
    }
}

macro_rules! impl_catalog_id_value_convert {
    ($method:ident -> $type:ty $(, $into_expr:expr)?) => {
        impl TryFrom<CatalogIdValue> for $type {
            type Error = CatalogIdError;

            fn try_from(value: CatalogIdValue) -> Result<Self, Self::Error> {
                value.$method()
            }
        }

        $(
            impl From<$type> for CatalogIdValue {
                fn from(value: $type) -> Self {
                    $into_expr(value)
                }
            }
        )?
    };
}

impl_catalog_id_value_convert!(into_id -> CatalogId);
impl_catalog_id_value_convert!(into_list -> Vec<CatalogIdValue>, Self::List);
impl_catalog_id_value_convert!(to_bool -> bool, |b: bool| Self::Ident(b.into()));
impl_catalog_id_value_convert!(to_int -> i64, |i: i64| Self::Ident(i.into()));

impl From<CatalogId> for CatalogIdValue {
    fn from(id: CatalogId) -> Self {
        if id.args.is_empty() {
            CatalogIdValue::Ident(id.base)
        } else {
            CatalogIdValue::Generator(id)
        }
    }
}

/// Error produced when parsing a catalog ID.
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum CatalogIdError {
    #[error("catalog ID parse error: {0}")]
    ParseError(Rich<'static, char, SimpleSpan>),
    #[error("expected {expected}; got {got}")]
    ExpectedGot {
        expected: &'static str,
        got: &'static str,
    },
    #[error("catalog ID cannot contain {0:?}")]
    BadChar(char),
    #[error("catalog ID cannot be empty")]
    Empty,
    #[error("integer parse error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("boolean parse error: {0}")]
    ParseBoolError(#[from] std::str::ParseBoolError),
}

fn write_comma_sep_list(f: &mut fmt::Formatter<'_>, elems: &[impl fmt::Display]) -> fmt::Result {
    let mut is_first = true;
    for elem in elems {
        if !std::mem::take(&mut is_first) {
            write!(f, ",")?;
        }
        fmt::Display::fmt(elem, f)?;
    }
    Ok(())
}

fn is_id_ident_char(c: char) -> bool {
    // allow `-` for negative numbers
    c.is_alphabetic() || c.is_ascii_digit() || c == '_' || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_id_roundtrip() {
        for s in [
            "product(ngon_ft(7,3).refl,line(3))",
            "megaminx_crystal",
            "curvy_copter.rot",
            "cube_ft(3).refl",
        ] {
            assert_eq!(s, CatalogId::from_str(s).unwrap().to_string());
        }
    }
}

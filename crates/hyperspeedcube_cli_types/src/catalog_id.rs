//! ID string for an object in a catalog.

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use chumsky::prelude::*;
use serde::{Deserialize, Serialize, de};

/// String that is nonempty and consist only of lowercase ASCII alphanumeric
/// characters, hyphens, and underscores; i.e., it must match the regex
/// `[a-z_-]+`. This is used in [`CatalogId`] and [`CatalogIdValue`] as part of
/// generator name or for numeric arguments to generators.
///
/// This type dereferences to [`str`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CatalogWord(Box<str>);

impl fmt::Debug for CatalogWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for CatalogWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for CatalogWord {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CatalogIdError::Empty);
        }
        if let Some(c) = s.chars().find(|&c| !is_catalog_word_char(c)) {
            return Err(CatalogIdError::BadChar(c));
        }
        Ok(Self(Box::from(s)))
    }
}

impl Deref for CatalogWord {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for CatalogWord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CatalogWord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        numeric_sort::cmp(self, other)
    }
}

impl From<bool> for CatalogWord {
    fn from(value: bool) -> Self {
        Self(value.to_string().into_boxed_str())
    }
}

impl From<i64> for CatalogWord {
    fn from(value: i64) -> Self {
        Self(value.to_string().into_boxed_str())
    }
}

impl CatalogWord {
    /// Constructs a [`VersionedCatalogWord`].
    #[must_use]
    pub fn with_version(self, version: Option<u32>) -> VersionedCatalogWord {
        VersionedCatalogWord {
            word: self,
            version,
        }
    }
}

/// [`CatalogWord`] with an optional version number separated by `@`. The
/// version number must be a valid `u32`. It may be zero.
///
/// Examples of valid versioned catalog words:
///
/// - `cube_ft`
/// - `-1`
/// - `cube_ft@1`
/// - `120cell_ft_shallow@0`
/// - `--1_4-@16`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VersionedCatalogWord {
    pub word: CatalogWord,
    pub version: Option<u32>,
}

impl fmt::Display for VersionedCatalogWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { word, version } = self;
        write!(f, "{word}")?;
        if let Some(v) = version {
            write!(f, "@{v}")?;
        }
        Ok(())
    }
}

impl<T: Into<CatalogWord>> From<T> for VersionedCatalogWord {
    fn from(value: T) -> Self {
        value.into().with_version(None)
    }
}

impl FromStr for VersionedCatalogWord {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('@') {
            Some((left, right)) => {
                let word = left.parse()?;
                let version = Some(
                    right
                        .parse()
                        .map_err(|_| CatalogIdError::BadVersion(right.to_string()))?,
                );
                Ok(Self { word, version })
            }
            None => Ok(Self::from(s.parse::<CatalogWord>()?)),
        }
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
    pub base: VersionedCatalogWord,
    /// Argument values, if the base string specifies a generator.
    pub args: Option<Vec<CatalogIdValue>>,
    /// Optional subset.
    pub subset: Option<CatalogWord>,
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
        if let Some(args) = args {
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

impl<T: Into<VersionedCatalogWord>> From<T> for CatalogId {
    fn from(value: T) -> Self {
        Self::new(value, [], None)
    }
}

impl FromStr for CatalogId {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CatalogIdValue::from_str(s)?.into_id()
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
        base: impl Into<VersionedCatalogWord>,
        args: impl IntoIterator<Item = CatalogIdValue>,
        subset: Option<CatalogWord>,
    ) -> Self {
        let base = base.into();
        let args: Vec<_> = args.into_iter().collect();
        let args = (!args.is_empty()).then_some(args);
        Self { base, args, subset }
    }

    /// Returns a catalog ID for an unnamed object.
    pub fn unnamed() -> Self {
        Self {
            base: "unnamed".parse().expect("invalid ID"),
            args: None,
            subset: None,
        }
    }

    /// Returns the arguments, or an empty slice if there are none.
    pub fn args(&self) -> &[CatalogIdValue] {
        self.args.as_deref().unwrap_or(&[])
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
            && self.args().len() == other.args().len()
            && self.subset == other.subset
            && std::iter::zip(self.args(), other.args())
                .all(|(pattern, value)| pattern.match_wildcards_into(value, output_buffer))
    }
}

/// Untyped abstract syntax tree node for a [`CatalogId`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CatalogIdValue {
    /// Generator invocation or primitive parameter value.
    Id(CatalogId),
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
            CatalogIdValue::Id(id) => fmt::Display::fmt(id, f),
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

impl From<CatalogId> for CatalogIdValue {
    fn from(value: CatalogId) -> Self {
        Self::Id(value)
    }
}

impl From<VersionedCatalogWord> for CatalogIdValue {
    fn from(value: VersionedCatalogWord) -> Self {
        Self::Id(value.into())
    }
}

impl From<CatalogWord> for CatalogIdValue {
    fn from(value: CatalogWord) -> Self {
        Self::Id(value.into())
    }
}

impl FromStr for CatalogIdValue {
    type Err = CatalogIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        recursive::<_, _, extra::Err<Rich<'_, char, SimpleSpan>>, _, _>(|ast_node| {
            let word = any()
                .filter(|&c| is_catalog_word_char(c))
                .repeated()
                .at_least(1)
                .to_slice()
                .map(Box::from)
                .map(CatalogWord)
                .padded();
            let int_u32 = text::digits(10)
                .to_slice()
                .try_map(|s, e| u32::from_str(s).map_err(|err| Rich::custom(e, err)));
            let id = word
                .then(just('@').padded().ignore_then(int_u32).or_not())
                .map(|(word, version)| VersionedCatalogWord { word, version })
                .then(
                    ast_node
                        .clone()
                        .separated_by(just(',').padded())
                        .collect()
                        .delimited_by(just('(').padded(), just(')').padded())
                        .or_not(),
                )
                .then(just('.').padded().ignore_then(word).or_not())
                .map(|((base, args), subset)| Self::Id(CatalogId { base, args, subset }));

            let list = ast_node
                .separated_by(just(',').padded())
                .collect()
                .delimited_by(just('[').padded(), just(']').padded())
                .map(Self::List);

            choice((
                id,
                list,
                just('*').to(Self::Wildcard),
                just('!').to(Self::Error),
            ))
            .padded()
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
            CatalogIdValue::Id(_) => "id",
            CatalogIdValue::List(_) => "list",
            CatalogIdValue::Wildcard => "wildcard '*'",
            CatalogIdValue::Error => "error '!'",
        };
        CatalogIdError::ExpectedGot { expected, got }
    }

    /// Parses the value into a catalog ID.
    pub fn into_id(self) -> Result<CatalogId, CatalogIdError> {
        match self {
            CatalogIdValue::Id(id) => Ok(id),
            _ => Err(self.expected_got("id")),
        }
    }
    /// Parses the value into a catalog word.
    pub fn into_word(self) -> Result<CatalogWord, CatalogIdError> {
        self.to_word_with_expected("word")
    }
    fn to_word_with_expected(&self, expected: &'static str) -> Result<CatalogWord, CatalogIdError> {
        match self {
            CatalogIdValue::Id(id)
                if id.args.is_none() && id.subset.is_none() && id.base.version.is_none() =>
            {
                Ok(id.base.word.clone())
            }
            _ => Err(self.expected_got(expected)),
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
        Ok(self.to_word_with_expected("boolean")?.parse()?)
    }
    /// Parses the value into an integer.
    pub fn to_int(&self) -> Result<i64, CatalogIdError> {
        Ok(self.to_word_with_expected("integer")?.parse()?)
    }

    fn match_wildcards_into(&self, other: &Self, output_buffer: &mut Vec<Self>) -> bool {
        match (self, other) {
            (CatalogIdValue::Id(a), CatalogIdValue::Id(b)) => {
                a.match_wildcards_into(b, output_buffer)
            }
            (CatalogIdValue::Id(_), _) => false,

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
impl_catalog_id_value_convert!(to_bool -> bool, |b: bool| Self::Id(b.into()));
impl_catalog_id_value_convert!(to_int -> i64, |i: i64| Self::Id(i.into()));

/// Error produced when parsing a catalog ID.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
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
    #[error("bad version: {0:?}")]
    BadVersion(String),
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

fn is_catalog_word_char(c: char) -> bool {
    // allow `-` for negative numbers
    matches!(c, 'a'..='z' | '0'..='9' | '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_id_roundtrip() {
        for s in [
            "product@3([ngon_ft@1(7,3).refl,line@2(3)])",
            "megaminx_crystal",
            "curvy_copter@5.rot",
            "cube_ft(3).refl",
        ] {
            assert_eq!(s, CatalogId::from_str(s).unwrap().to_string());
        }

        assert_eq!(
            Ok("product@3([ngon_ft@1(7,3).refl,line@2(3)])".to_string()),
            CatalogId::from_str(
                "  product  @  3  (  [  ngon_ft  @  1  (  7  ,  3  )  .  refl  ,  line  @  2  (  3  )  ]  )  ",
            )
            .map(|id| id.to_string()),
        );
    }
}

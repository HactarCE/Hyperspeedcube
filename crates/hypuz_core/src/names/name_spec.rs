//! Name specifications, used for puzzle elements such as axes, twists, and
//! colors.
//!
//! - Only Latin and Greek letters are literal
//! - `_` are separators for optionally-permutable strings.
//! - `{ ... }` are used to denote permutable sets
//! - `|` is used to separate top-level choices
//! - All other ASCII punctuation is reserved for future use

use std::borrow::Cow;
use std::collections::{HashMap, hash_map};
use std::str::FromStr;

use itertools::Itertools;

use super::BadName;

/// String of 32 underscores.
const SEPARATOR_STR: &str = "________________________________";

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameSpecError {
    #[error("char {0:?} is not allowed")]
    BadChar(char),
    #[error("too many `{{`")]
    TooManyLBrace,
    #[error("too many `}}`")]
    TooManyRBrace,
    #[error("missing matching `{{`")]
    MissingMatchingLBrace,
    #[error("missing matching `}}`")]
    MissingMatchingRBrace,
    #[error("nested permutations must use different levels of separators")]
    NestedPermutationAtSameSeparatorLevel,
    #[error("cannot permute nonequivalent segments")]
    CannotPermuteNonequivalentSegments,
    #[error("empty segment")]
    EmptySegment,
    #[error("cannot permute zero elements")]
    CannotPermuteZeroElements,
    #[error("cannot permute one element")]
    CannotPermuteOneElement,
}

/// Returns the preferred name for `name_spec`.
pub fn preferred_name_from_name_spec(name_spec: &str) -> String {
    name_spec
        .chars()
        .filter(|&c| c != '{' && c != '}')
        .take_while(|&c| c != '|')
        .collect()
}

/// Returns whether a name spec is valid.
pub fn is_name_spec_valid(name_spec: &str) -> bool {
    parse_name_spec_into_patterns(name_spec).is_ok()
}

/// Returns whether a name spec matches a name.
///
/// Consider building a [`NameSpecMap`] if calling this multiple times on the
/// same name spec.
pub fn name_spec_matches_name(name_spec: &str, name: &str) -> bool {
    parse_name_spec_into_patterns(name_spec).is_ok_and(|patterns| {
        patterns.into_iter().any(|(pat, canonicalized_name_spec)| {
            pat.canonicalize(name)
                .is_some_and(|canonicalized_name| canonicalized_name == canonicalized_name_spec)
        })
    })
}

/// Parsed name specification, such as `I{UFR}`.
#[derive(Debug, Clone)]
pub struct NameSpec {
    /// Preferred name, such as `IUFR`.
    pub preferred: String,
    /// Original name specification, such as `I{UFR}`.
    pub spec: String,
    /// Lexicographically-first name; useful for canonical ordering.
    pub canonical: String,
}
impl FromStr for NameSpec {
    type Err = BadName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}
impl NameSpec {
    /// Constructs a name specification from a string such as `I{UFR}`.
    pub fn new(spec: String) -> Result<Self, BadName> {
        let preferred = preferred_name_from_name_spec(&spec);
        let canonical = parse_name_spec_into_patterns(&spec)?
            .into_iter()
            .map(|(_pattern, canonical)| canonical)
            .min()
            .unwrap_or_else(|| preferred.clone());
        Ok(Self {
            preferred,
            spec,
            canonical,
        })
    }

    /// Returns a key to use for sorting name specs canonically.
    ///
    /// TODO: review uses of this and consider canonicalizing the namespec in
    ///       all cases
    pub fn sort_key(spec: String) -> impl Ord {
        match Self::new(spec) {
            Ok(name) => (None, Some(name.canonical)),
            Err(error) => (Some(error), None),
        }
    }
}

/// Map from name spec to value.
#[derive(Debug, Clone)]
pub struct NameSpecMap<V>(HashMap<NamePattern, HashMap<String, V>>);
impl<V> Default for NameSpecMap<V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}
impl<V: Clone> NameSpecMap<V> {
    /// Constructs a new empty name map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts `name_spec` into the map associated to `value`.
    ///
    /// If there is an equivalent pattern with an equivalent name, then an error
    /// is returned and the map is not modified.
    ///
    /// If successful, returns the canonicalized name.
    pub fn insert(&mut self, name_spec: &str, value: &V) -> Result<String, BadName> {
        let mut min_canonical = None;
        let mut saved_patterns = vec![];
        for (pattern, canonicalized_name) in parse_name_spec_into_patterns(name_spec)? {
            let pat = pattern;
            let canon = canonicalized_name;
            match self.0.entry(pat.clone()).or_default().entry(canon.clone()) {
                hash_map::Entry::Occupied(_) => {
                    // Remove partial progress.
                    for (pat, canon) in saved_patterns {
                        self.0.entry(pat).or_default().remove(&canon);
                    }

                    return Err(BadName::AlreadyTaken { name: canon });
                }
                hash_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(value.clone());
                    saved_patterns.push((pat, canon.clone()));
                }
            }

            if min_canonical.as_ref().is_none_or(|it| *it > canon) {
                min_canonical = Some(canon);
            }
        }
        min_canonical.ok_or(BadName::Empty)
    }

    /// Returns the value associated with a name.
    pub fn get(&self, name: &str) -> Option<&V> {
        self.0
            .iter()
            .find_map(|(pattern, ids)| ids.get(&*pattern.canonicalize(name)?))
    }

    /// Returns the ID of the element with a name based on the preferred form of
    /// `name_pattern`.
    ///
    /// This does not guarantee that the whole pattern is unique.
    pub fn get_from_pattern(&self, name_pattern: &str) -> Option<&V> {
        self.get(&preferred_name_from_name_spec(name_pattern))
    }

    /// Removes each pattern in `name_pattern`.
    pub fn remove(&mut self, name_pattern: &str) -> Result<(), BadName> {
        for (pattern, canonicalized) in parse_name_spec_into_patterns(name_pattern)? {
            if let Some(hashmap) = self.0.get_mut(&pattern) {
                hashmap.remove(&canonicalized);
            }
        }
        Ok(())
    }
}

/// Pattern for a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NamePattern(SeqNode);
impl NamePattern {
    /// Canonicalizes a string according to the pattern, or returns `None` if
    /// the string does not match the pattern.
    fn canonicalize<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.0.canonicalize(s)
    }
}

/// Parses a name specification (such as `I_{U_{FR}}|Q`) into a list of
/// [`NamePattern`]s, each with an associated canonicalized name.
///
/// Returns `None` in the case of a parse error.
fn parse_name_spec_into_patterns(name_spec: &str) -> Result<Vec<(NamePattern, String)>, BadName> {
    name_spec
        .split('|')
        .map(|name_spec_choice| {
            let tokens = lex_to_tokens(name_spec_choice)?;
            let seq = parse_tokens(&tokens)?;

            let canonicalized = seq
                .canonicalize(
                    &name_spec_choice
                        .chars()
                        .filter(|&c| c != '{' && c != '}')
                        .collect::<String>(),
                )
                .ok_or(BadName::InternalError)?
                .into_owned();

            Ok((NamePattern(seq), canonicalized))
        })
        .try_collect()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Token {
    LBrace,
    RBrace,
    Separator(usize),
    Literal(char),
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for Token {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        prop_oneof![
            Just(Token::LBrace),
            Just(Token::RBrace),
            (0..5_usize).prop_map(Token::Separator),
            prop::char::range('A', 'Z').prop_map(Token::Literal),
        ]
        .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}

fn lex_to_tokens(s: &str) -> Result<Vec<Token>, NameSpecError> {
    let mut tokens = vec![];
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        tokens.push(match c {
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '_' => {
                let mut separator_power = 1;
                while chars.peek() == Some(&'_') {
                    chars.next();
                    separator_power += 1;
                }
                Token::Separator(separator_power)
            }
            _ if hypuz_notation::charsets::is_family_char(c) => Token::Literal(c),
            _ => return Err(NameSpecError::BadChar(c)),
        });
    }
    Ok(tokens)
}

fn parse_tokens(tokens: &[Token]) -> Result<SeqNode, NameSpecError> {
    if tokens.is_empty() {
        return Err(NameSpecError::EmptySegment);
    }
    if !tokens
        .iter()
        .any(|t| matches!(t, Token::LBrace | Token::RBrace))
    {
        return Ok(SeqNode::Literal);
    }

    let separator_power = tokens
        .iter()
        .map(|&t| match t {
            Token::Separator(n) => n,
            _ => 0,
        })
        .max()
        .unwrap_or(0);
    let mut items = vec![];
    let mut permutation_ranges = vec![];
    let mut permutation_start = None;
    // TODO: ew, collecting
    let segments = if separator_power == 0 {
        tokens
            .iter()
            .map(|t| std::array::from_ref(t).as_slice())
            .collect_vec()
    } else {
        tokens
            .split(|&t| t == Token::Separator(separator_power))
            .collect_vec()
    };
    for mut segment in segments {
        let mut net_depth = 0_i32;
        for &t in segment {
            net_depth += (t == Token::LBrace) as i32;
            net_depth -= (t == Token::RBrace) as i32;
        }
        match net_depth {
            1.. => {
                if net_depth == 1
                    && let Some((Token::LBrace, rest)) = segment.split_first()
                {
                    if permutation_start.is_some() {
                        return Err(NameSpecError::NestedPermutationAtSameSeparatorLevel);
                    }
                    permutation_start = Some(items.len());
                    segment = rest;
                } else {
                    return Err(NameSpecError::TooManyLBrace);
                }
            }
            ..=-1 => {
                if net_depth == -1
                    && let Some((Token::RBrace, rest)) = segment.split_last()
                {
                    let Some(start) = permutation_start.take() else {
                        return Err(NameSpecError::MissingMatchingLBrace);
                    };
                    let end = items.len() + (!rest.is_empty()) as usize;
                    if start == end {
                        return Err(NameSpecError::CannotPermuteZeroElements);
                    } else if start + 1 == end {
                        return Err(NameSpecError::CannotPermuteOneElement);
                    }
                    permutation_ranges.push(start..end);
                    segment = rest;
                } else {
                    return Err(NameSpecError::TooManyRBrace);
                }
            }
            0 if permutation_start.is_none() => {
                permutation_ranges.push(items.len()..items.len() + 1);
            }
            0 => (),
        }
        if segment.is_empty() {
            if separator_power > 0 {
                return Err(NameSpecError::EmptySegment);
            }
        } else {
            items.push(parse_tokens(segment)?);
        }
    }
    if permutation_start.is_some() {
        return Err(NameSpecError::MissingMatchingRBrace);
    }

    if items.is_empty() {
        return Err(NameSpecError::EmptySegment);
    }

    let mut permutations = vec![];
    for range in permutation_ranges {
        if range.is_empty() {
            return Err(NameSpecError::EmptySegment);
        }
        if !items[range.clone()].iter().all_equal() {
            return Err(NameSpecError::CannotPermuteNonequivalentSegments);
        }
        permutations.push(PermutationNode {
            inner: items[range.start].clone(),
            count: range.count(),
        });
    }
    Ok(SeqNode::Sequence {
        separator_power,
        permutations,
    })
}

/// AST node containing a literal or a sequence of permutations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SeqNode {
    /// Literal string of any length.
    Literal,
    /// Sequence of literals and permutations.
    Sequence {
        /// Number of `_` between elements. For a permutation of individual
        /// characters, this is zero.
        separator_power: usize,
        /// Sequences of literals and permutations.
        ///
        /// Permutation nodes are joined using `separator` and each permutation
        /// also uses `separator`.
        permutations: Vec<PermutationNode>,
    },
}

/// AST node containing a permutation of N elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PermutationNode {
    /// Structure of an element of the permutation.
    inner: SeqNode,
    /// Number of elements in the permutation.
    count: usize,
}

impl SeqNode {
    /// Canonicalizes a string according to the pattern, or returns `None` if
    /// the string does not match the pattern.
    fn canonicalize<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        match self {
            SeqNode::Literal => Some(s.into()),

            SeqNode::Sequence {
                separator_power,
                permutations,
            } => {
                let elements = split_str_by_separator(s, *separator_power);
                let expected_element_count = permutations.iter().map(|c| c.count).sum::<usize>();
                if expected_element_count != elements.len() {
                    return None;
                }

                let mut i = 0;
                let mut canonicalized_elements = vec![];
                for component in permutations {
                    // Canonicalize by sorting lexicographically
                    let permutable_elements = &elements[i..i + component.count];
                    for elem in permutable_elements
                        .iter()
                        .map(|elem| component.inner.canonicalize(elem))
                        .sorted()
                    {
                        canonicalized_elements.push(elem?);
                    }
                    i += component.count;
                }

                Some(
                    canonicalized_elements
                        .join(&separator_str(*separator_power))
                        .into(),
                )
            }
        }
    }
}

fn split_str_by_separator(s: &str, separator_power: usize) -> Vec<&str> {
    if separator_power == 0 {
        s.char_indices()
            .map(|(i, c)| &s[i..i + c.len_utf8()])
            .collect()
    } else {
        s.split(&*separator_str(separator_power)).collect()
    }
}

fn separator_str(separator_power: usize) -> Cow<'static, str> {
    if separator_power > SEPARATOR_STR.len() {
        Cow::Owned(std::iter::repeat_n("_", separator_power).collect())
    } else {
        Cow::Borrowed(&SEPARATOR_STR[..separator_power])
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[track_caller]
    fn assert_test_case(input: &str, expected_canonicalized: &str) {
        let name_specs = parse_name_spec_into_patterns(input).unwrap();
        assert_eq!(name_specs.len(), 1);
        let (_name_pattern, canonicalized) = &name_specs[0];
        assert_eq!(canonicalized, expected_canonicalized, "oh no");
    }

    #[test]
    fn test_name_parsing_and_canonicalization() {
        assert_test_case("meow", "meow");
        assert_test_case("{mmmeow}", "emmmow");
        assert_test_case("{mm_me_ow}", "me_mm_ow");
        assert_test_case("{mm_e_ow}", "e_mm_ow"); // mismatch length ok
        assert_test_case("{mm__me__ow}", "me__mm__ow");
        assert_test_case("{mm___me___ow}", "me___mm___ow");
        assert_test_case("z__e_b__r__a", "z__e_b__r__a");
        assert_test_case("{z_e_b_r_a}", "a_b_e_r_z");
        assert_test_case("{z_e}_{b_r}_a", "e_z_b_r_a");
        assert_test_case("{{ze}_{br}}_a", "br_ez_a");
        assert_test_case(
            "{{{twa}_{tri}}__{{ima}_{nhh}}__{{suc}_{ces}}___{{its}_{hrd}}__{{too}_{ver}}__{{sta}_{tem}}}",
            "atw_irt__aim_hhn__ces_csu___dhr_ist__erv_oot__ast_emt",
        );
        assert_test_case(
            "twa_tri__ima_nhh__suc_ces___its_hrd__too_ver__sta_tem",
            "twa_tri__ima_nhh__suc_ces___its_hrd__too_ver__sta_tem",
        );
        assert_test_case("{Al_pha__Gamm_a__Be_ta}", "Al_pha__Be_ta__Gamm_a");

        // empty not allowed
        parse_name_spec_into_patterns("").unwrap_err();
        parse_name_spec_into_patterns("{").unwrap_err();
        parse_name_spec_into_patterns("}").unwrap_err();
        parse_name_spec_into_patterns("{}").unwrap_err();
        parse_name_spec_into_patterns("}{").unwrap_err();
        parse_name_spec_into_patterns("{a}").unwrap_err();
        parse_name_spec_into_patterns("{_}").unwrap_err();
        parse_name_spec_into_patterns("{a_}").unwrap_err();
        parse_name_spec_into_patterns("{_a_b}").unwrap_err();
        // mismatch structure not allowed
        parse_name_spec_into_patterns("{{z__e}__{b__r}__a}").unwrap_err();
        parse_name_spec_into_patterns("{{z_e}__{br}}__a").unwrap_err();
        parse_name_spec_into_patterns("{{z_e}__{a_b_c}}__a").unwrap_err();
    }

    proptest! {
        #[test]
        fn proptest_name_lexing(s: String) {
            let _ = lex_to_tokens(&s); // don't panic!
        }

        #[test]
        fn proptest_name_parsing(tokens: Vec<Token>) {
            let _ = parse_tokens(Vec::as_slice(&tokens)); // don't panic!
        }
    }
}

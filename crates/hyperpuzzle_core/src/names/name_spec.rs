use std::borrow::Cow;
use std::collections::{HashMap, hash_map};
use std::str::FromStr;

use itertools::Itertools;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::take_while_m_n;
use nom::character::complete::{anychar, char};
use nom::combinator::{all_consuming, complete, fail, value, verify};
use nom::error::Error;
use nom::multi::{many1, separated_list1};
use nom::sequence::{delimited, separated_pair};

use super::BadName;

/// Separator characters, in order from loosest-binding to tighest-binding.
///
/// `None` represents individual characters, which use no separator.
const SEPARATORS: &[Option<char>] = &[Some('_'), Some('-'), Some('.'), None];

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

/// Returns whether ar name spec matches a name.
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
///
/// TODO: document name specifications
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

/// Parses a name specification (such as `I{UFR}|Q`) into a list of
/// [`NamePattern`]s, each with an associated canonicalized name.
///
/// Returns `None` in the case of a parse error.
fn parse_name_spec_into_patterns(name_spec: &str) -> Result<Vec<(NamePattern, String)>, BadName> {
    name_spec
        .split('|')
        .map(|segment| {
            let (_remaining_input, seq) = all_consuming(separated_element_sequence(SEPARATORS))
                .parse_complete(segment)
                .map_err(|_| BadName::InvalidName {
                    name: segment.to_string(),
                })?;

            let canonicalized = seq
                .canonicalize(
                    &segment
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

/// Parser for a sequence of elements separated by a separator character.
fn separated_element_sequence<'a>(
    separators: &'a [Option<char>],
) -> impl 'a + Parser<&'a str, Output = SeqNode, Error = Error<&'a str>> {
    |s: &'a str| match separators.split_first() {
        None => fail().parse(s),
        Some((separator, remaining_separators)) => (|s: &'a str| match separator {
            Some(c) => complete(separated_list1(
                nom::character::char(*c),
                separated_permutation(*c, remaining_separators),
            ))
            .parse(s),
            None => many1(chars_permutation()).parse(s),
        })
        .map(|permutable_components| {
            if let Ok(PermutationNode { inner, count: 1 }) =
                permutable_components.iter().exactly_one()
            {
                inner.clone()
            } else {
                SeqNode::Sequence {
                    separator: *separator,
                    permutations: permutable_components,
                }
            }
        })
        .parse(s),
    }
}

/// Parser for a permutation of at least one element.
fn separated_permutation(
    separator: char,
    remaining_separators: &[Option<char>],
) -> impl '_ + Parser<&str, Output = PermutationNode, Error = Error<&str>> {
    alt((
        // Permutation of at least 2 elements. Example: `{Al.pha-Be.ta-Gam.ma}`
        delimited(
            char('{'),
            separated_pair(
                separated_element_sequence(remaining_separators),
                char(separator),
                separated_list1(
                    char(separator),
                    separated_element_sequence(remaining_separators),
                ),
            ),
            char('}'),
        )
        .map_opt(|(first, mut rest)| {
            rest.insert(0, first);
            let count = rest.len();
            rest.into_iter()
                .map(SeqNode::simplify)
                .all_equal_value()
                .map(|inner| PermutationNode { inner, count })
                .ok()
        }),
        // Single element. Example: `Al.pha-Be.ta-Gam.ma`
        separated_element_sequence(remaining_separators)
            .map(SeqNode::simplify)
            .map(|inner| PermutationNode { inner, count: 1 }),
    ))
}

/// Parser for a permutation of at least one character.
fn chars_permutation<'input>()
-> impl Parser<&'input str, Output = PermutationNode, Error = Error<&'input str>> {
    alt((
        // Permutation of at least 2 characters. Example: `{ABC}`
        delimited(
            char::<_, Error<&str>>('{'),
            take_while_m_n(2, usize::MAX, |c: char| !c.is_ascii_punctuation()).map(|s: &str| {
                PermutationNode {
                    inner: SeqNode::Literal,
                    count: s.len(),
                }
            }),
            char('}'),
        ),
        // Single character. Example: `A` (may be apostrophe)
        value(
            PermutationNode {
                inner: SeqNode::Literal,
                count: 1,
            },
            verify(anychar, |&c: &char| !c.is_ascii_punctuation() || c == '\''),
        ),
    ))
}

/// AST node containing a literal or a sequence of permutations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SeqNode {
    /// Literal string of any length.
    Literal,
    /// Sequence of literals and permutations.
    Sequence {
        /// Separator between elements.
        separator: Option<char>,
        /// Sequences of literals and permutations.
        ///
        /// Permutation nodes are joined using `separator` and each permutation
        /// also uses `separator`.
        permutations: Vec<PermutationNode>,
    },
}
impl SeqNode {
    /// Returns whether the sequence can be simplified to a literal string.
    fn is_trivial(&self) -> bool {
        match self {
            SeqNode::Literal => true,
            SeqNode::Sequence {
                permutations: permutable_components,
                ..
            } => permutable_components.iter().all(|c| {
                matches!(
                    c,
                    PermutationNode {
                        inner: SeqNode::Literal,
                        count: 1
                    }
                )
            }),
        }
    }
    /// Simplifies the sequence to a literal string if possible.
    fn simplify(self) -> Self {
        if self.is_trivial() {
            SeqNode::Literal
        } else {
            self
        }
    }
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
                separator,
                permutations,
            } => {
                let elements = split_str_by_opt_char(s, *separator);
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

                Some(join_with_sep(canonicalized_elements, *separator).into())
            }
        }
    }
}

fn split_str_by_opt_char(s: &str, separator: Option<char>) -> Vec<&str> {
    match separator {
        Some(sep) => s.split(sep).collect(),
        None => s
            .char_indices()
            .map(|(i, c)| &s[i..i + c.len_utf8()])
            .collect(),
    }
}
fn join_with_sep<'a>(
    strings: impl IntoIterator<Item = Cow<'a, str>>,
    separator: Option<char>,
) -> String {
    match separator {
        Some(sep) => strings.into_iter().join(&sep.to_string()),
        None => strings.into_iter().join(""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_test_case(input: &str, expected_canonicalized: &str) {
        let name_specs = parse_name_spec_into_patterns(input).unwrap();
        assert_eq!(name_specs.len(), 1);
        let (_name_pattern, canonicalized) = &name_specs[0];
        assert_eq!(canonicalized, expected_canonicalized);
    }

    #[test]
    fn test_name_parsing_and_canonicalization() {
        assert_test_case("meow", "meow");
        assert_test_case("{mmmeow}", "emmmow");
        assert_test_case("{mm.me.ow}", "me.mm.ow");
        assert_test_case("{mm.e.ow}", "e.mm.ow"); // mismatch length ok
        assert_test_case("{mm-me-ow}", "me-mm-ow");
        assert_test_case("{mm_me_ow}", "me_mm_ow");
        assert_test_case("z_e-b_r_a", "z_e-b_r_a");
        assert_test_case("{z_e_b_r_a}", "a_b_e_r_z");
        assert_test_case("{z_e}_{b_r}_a", "e_z_b_r_a");
        assert_test_case("{{ze}_{br}}_a", "br_ez_a");
        assert_test_case(
            "{{{twa}.{tri}}-{{ima}.{nhh}}-{{suc}.{ces}}_{{its}.{hrd}}-{{too}.{ver}}-{{sta}.{tem}}}",
            "atw.irt-aim.hhn-ces.csu_dhr.ist-erv.oot-ast.emt",
        );
        assert_test_case(
            "twa.tri-ima.nhh-suc.ces_its.hrd-too.ver-sta.tem",
            "twa.tri-ima.nhh-suc.ces_its.hrd-too.ver-sta.tem",
        );
        assert_test_case("{Al.pha-Be.ta-Gam.ma}", "Al.pha-Be.ta-Gam.ma");

        // empty not allowed
        parse_name_spec_into_patterns("").unwrap_err();
        parse_name_spec_into_patterns("{}").unwrap_err();
        parse_name_spec_into_patterns("{a}").unwrap_err();
        parse_name_spec_into_patterns("{.}").unwrap_err();
        parse_name_spec_into_patterns("{a.}").unwrap_err();
        parse_name_spec_into_patterns("{.a.b}").unwrap_err();
        // mismatch structure not allowed
        parse_name_spec_into_patterns("{{z_e}_{b_r}_a}").unwrap_err();
        parse_name_spec_into_patterns("{{z.e}_{br}}_a").unwrap_err();
        parse_name_spec_into_patterns("{{z.e}_{a.b.c}}_a").unwrap_err();
    }
}

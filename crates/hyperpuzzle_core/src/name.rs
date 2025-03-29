use std::borrow::Cow;

use itertools::Itertools;
use nom::{
    Parser,
    branch::alt,
    bytes::complete::take_while_m_n,
    character::complete::{anychar, char},
    combinator::{all_consuming, complete, fail, value, verify},
    error::Error,
    multi::{many1, separated_list1},
    sequence::{delimited, separated_pair},
};

/// Separator characters, in order from loosest-binding to tighest-binding.
///
/// `None` represents individual characters, which use no separator.
const SEPARATORS: &[Option<char>] = &[Some('_'), Some('-'), Some('.'), None];

/// Pattern for a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePattern(SeqNode);
impl NamePattern {
    /// Canonicalizes a string according to the pattern, or returns `None` if
    /// the string does not match the pattern.
    pub fn canonicalize<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
        self.0.canonicalize(s)
    }
}

/// Parses a name specification (such as `I{UFR}`) into a [`NamePattern`] and a
/// preferred name.
pub fn parse_name_spec(s: &str) -> Option<(NamePattern, String)> {
    let name: String = s.chars().filter(|&c| c != '{' && c != '}').collect();

    // Do not allow empty name
    if name.is_empty() {
        return None;
    }

    // Optimize for the simplest case
    if name == s {
        return Some((NamePattern(SeqNode::Literal), name));
    }

    let (_remaining_input, seq) = all_consuming(separated_element_sequence(SEPARATORS))
        .parse_complete(s)
        .ok()?;

    Some((NamePattern(seq), name))
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
        // Single character. Example: `A`
        value(
            PermutationNode {
                inner: SeqNode::Literal,
                count: 1,
            },
            verify(anychar, |c: &char| !c.is_ascii_punctuation()),
        ),
    ))
}

/// AST node containing a literal or a sequence of permutations.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
struct PermutationNode {
    /// Structure of an element of the permutation.
    inner: SeqNode,
    /// Number of elements in the permutation.
    count: usize,
}

impl SeqNode {
    /// Canonicalizes a string according to the pattern, or returns `None` if
    /// the string does not match the pattern.
    pub fn canonicalize<'a>(&self, s: &'a str) -> Option<Cow<'a, str>> {
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
        let (name_pattern, name) = parse_name_spec(input).unwrap();
        assert_eq!(
            name_pattern.canonicalize(&name).unwrap(),
            expected_canonicalized,
        );
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
        assert_eq!(None, parse_name_spec(""));
        assert_eq!(None, parse_name_spec("{}"));
        assert_eq!(None, parse_name_spec("{a}"));
        assert_eq!(None, parse_name_spec("{.}"));
        assert_eq!(None, parse_name_spec("{a.}"));
        assert_eq!(None, parse_name_spec("{.a.b}"));
        // mismatch structure not allowed
        assert_eq!(None, parse_name_spec("{{z_e}_{b_r}_a}"));
        assert_eq!(None, parse_name_spec("{{z.e}_{br}}_a"));
        assert_eq!(None, parse_name_spec("{{z.e}_{a.b.c}}_a"));
    }
}

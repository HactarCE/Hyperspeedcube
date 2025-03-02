//! Functions for parsing and formatting general puzzle twist notation.

use std::collections::HashMap;

use hyperpuzzle_core::{LayerMask, LayeredTwist, PerTwist, Twist, TwistInfo};
use itertools::Itertools;
use regex::Regex;
use smallvec::{SmallVec, smallvec};

/// Formats a sequence of twists as a string.
pub fn format_twists(
    all_twists: &PerTwist<TwistInfo>,
    twists: impl IntoIterator<Item = LayeredTwist>,
) -> String {
    twists
        .into_iter()
        .map(|LayeredTwist { layers, transform }| layers.to_string() + &all_twists[transform].name)
        .join(" ")
}

/// Parses a sequence of twists, allowing non-nested parenthetical groupings.
pub fn parse_grouped_twists<'a>(
    twists_by_name: &'a HashMap<String, Twist>,
    s: &'a str,
) -> Vec<SmallVec<[Result<LayeredTwist, TwistParseError<'a>>; 1]>> {
    // TODO: handle more than 2 nested parens, and also maybe commutator notation
    let mut ret = vec![];
    let mut start = 0;
    while start < s.len() {
        let end = start + s[start..].find('(').unwrap_or(s.len() - start);
        ret.extend(parse_twists(twists_by_name, &s[start..end]).map(|result| smallvec![result]));
        start = end.saturating_add(1).min(s.len());
        let end = start + s[start..].find(')').unwrap_or(s.len() - start);
        let group: SmallVec<_> = parse_twists(twists_by_name, &s[start..end]).collect();
        if !group.is_empty() {
            ret.push(group);
        }
        start = end.saturating_add(1).min(s.len());
    }
    ret
}
/// Parses a sequence of twists with no parentheses.
pub fn parse_twists<'a>(
    twists_by_name: &'a HashMap<String, Twist>,
    s: &'a str,
) -> impl 'a + Iterator<Item = Result<LayeredTwist, TwistParseError<'a>>> {
    s.split_whitespace()
        .map(|word| parse_twist(twists_by_name, word))
}

/// Parses a single twist.
fn parse_twist<'a>(
    twists_by_name: &HashMap<String, Twist>,
    s: &'a str,
) -> Result<LayeredTwist, TwistParseError<'a>> {
    let (layers, rest) = strip_layer_mask_prefix(s)?;
    let layers = layers.unwrap_or(LayerMask::default());
    let transform = *twists_by_name
        .get(rest)
        .ok_or(TwistParseError::BadTwist(rest))?;
    Ok(LayeredTwist { layers, transform })
}

/// Returns a layer mask from the beginning of `string` and the remainder of
/// `string` after the layer mask.
fn strip_layer_mask_prefix(string: &str) -> Result<(Option<LayerMask>, &str), TwistParseError<'_>> {
    const LAYER_PREFIX_PATTERN: &str = r"^(\{[\d\s,-]*\}|\d+)(.*)$";
    // match the whole string            ^                       $
    // capture                            (                 )
    //   match a pair of `{}`              \{         \}
    //     any number of                     [      ]*
    //       digits,                          \d
    //       whitespace,                        \s
    //       commas,                              ,
    //       and hyphens                           -
    //   or                                             |
    //     a sequence of one or more digits              \d+
    // then capture the rest                                 (.*)

    lazy_static! {
        static ref LAYER_PREFIX_REGEX: Regex = Regex::new(LAYER_PREFIX_PATTERN).expect("bad regex");
    }

    Ok(match LAYER_PREFIX_REGEX.captures(string) {
        Some(captures) => {
            // need `.get()` for lifetime reasons
            let layers_str = captures.get(1).expect("missing regex group").as_str();
            let rest_str = captures.get(2).expect("missing regex group").as_str();
            let layers = layers_str
                .parse::<LayerMask>()
                .map_err(|_| TwistParseError::BadLayerMask(layers_str))?;
            (Some(layers), rest_str)
        }
        None => (None, string),
    })
}

/// Error encountered while parsing a twist.
#[allow(missing_docs)]
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum TwistParseError<'a> {
    #[error("bad layer mask: {0:?}")]
    BadLayerMask(&'a str),
    #[error("bad twist: {0:?}")]
    BadTwist(&'a str),
}

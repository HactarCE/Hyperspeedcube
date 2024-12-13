use std::collections::HashMap;

use hyperpuzzle::{LayerMask, LayeredTwist, PerTwist, Twist, TwistInfo};
use itertools::Itertools;
use regex::Regex;

pub fn format_twists(
    all_twists: &PerTwist<TwistInfo>,
    twists: impl IntoIterator<Item = LayeredTwist>,
) -> String {
    twists
        .into_iter()
        .map(|LayeredTwist { layers, transform }| layers.to_string() + &all_twists[transform].name)
        .join(" ")
}

pub fn parse_twists<'a>(
    twists_by_name: &'a HashMap<String, Twist>,
    s: &'a str,
) -> impl 'a + Iterator<Item = Result<LayeredTwist, TwistParseError<'a>>> {
    s.split_whitespace()
        .map(|word| parse_twist(twists_by_name, word))
}

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

fn strip_layer_mask_prefix<'a>(
    string: &'a str,
) -> Result<(Option<LayerMask>, &'a str), TwistParseError<'a>> {
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
        static ref LAYER_PREFIX_REGEX: Regex = Regex::new(LAYER_PREFIX_PATTERN).unwrap();
    }

    Ok(match LAYER_PREFIX_REGEX.captures(string) {
        Some(captures) => {
            // need `.get()` for lifetime reasons
            let layers_str = captures.get(1).unwrap().as_str();
            let rest_str = captures.get(2).unwrap().as_str();
            let layers = layers_str
                .parse::<LayerMask>()
                .map_err(|_| TwistParseError::BadLayerMask(layers_str))?;
            (Some(layers), rest_str)
        }
        None => (None, string),
    })
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum TwistParseError<'a> {
    #[error("bad layer mask: {0:?}")]
    BadLayerMask(&'a str),
    #[error("bad twist: {0:?}")]
    BadTwist(&'a str),
}

#![allow(missing_docs)]

// TODO: delete this file

use itertools::Itertools;
use regex::Regex;
use std::fmt;

use super::*;

#[derive(Debug, Clone)]
pub struct NotationScheme {
    pub axis_names: Vec<String>,
    pub direction_names: Vec<TwistDirectionName>,
    pub block_suffix: Option<String>,
    pub aliases: Vec<(String, Alias)>,
    // TODO: flag to allow chaining directions (e.g., "Rxyx'y")
}

#[derive(Debug, Copy, Clone)]
pub enum Alias {
    AxisLayers(TwistAxis, LayerMask),
    EntireTwist(Twist),
}
impl Alias {
    fn matches(self, twist: Twist) -> bool {
        match self {
            Alias::AxisLayers(axis, layers) => axis == twist.axis && layers == twist.layers,
            Alias::EntireTwist(t) => t == twist,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TwistDirectionName {
    Same(String),
    PerAxis(Vec<String>),
}
impl TwistDirectionName {
    fn for_axis(&self, axis: TwistAxis) -> &str {
        match self {
            TwistDirectionName::Same(name) => name,
            TwistDirectionName::PerAxis(names) => &names[axis.0 as usize],
        }
    }
}

impl NotationScheme {
    pub fn twist_to_string(&self, twist: Twist) -> String {
        struct NotatedTwist<'a> {
            scheme: &'a NotationScheme,
            twist: Twist,
        }
        impl fmt::Display for NotatedTwist<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.scheme.format_twist(f, self.twist)
            }
        }

        let t = NotatedTwist {
            scheme: self,
            twist,
        };

        format!("{}", t)
    }

    pub fn format_twist(&self, f: &mut fmt::Formatter<'_>, twist: Twist) -> fmt::Result {
        // First, try searching for a relevant alias.
        for (alias_str, alias) in &self.aliases {
            if alias.matches(twist) {
                write!(f, "{alias_str}")?;
                match alias {
                    Alias::AxisLayers(..) => {
                        return self.format_direction(f, twist.axis, twist.direction)
                    }
                    Alias::EntireTwist(..) => return Ok(()),
                }
            }
        }

        // If that doesn't work, format the twist normally.
        self.format_layers(f, twist.layers)?;
        self.format_axis(f, twist.axis)?;
        if let Some(block_suffix) = &self.block_suffix {
            if twist.layers.is_contiguous_from_outermost() && twist.layers.count() > 1 {
                write!(f, "{block_suffix}")?;
            }
        }
        self.format_direction(f, twist.axis, twist.direction)?;

        Ok(())
    }
    fn format_layers(&self, f: &mut fmt::Formatter<'_>, layers: LayerMask) -> fmt::Result {
        if layers.is_default() {
            Ok(()) // Layer mask is not necessary.
        } else if self.block_suffix.is_some() && layers.is_contiguous_from_outermost() {
            if layers.count() <= 2 {
                Ok(()) // Layer mask is not necessary.
            } else {
                write!(f, "{}", layers.count())
            }
        } else {
            write!(f, "{}", layers)
        }
    }
    fn format_axis(&self, f: &mut fmt::Formatter<'_>, axis: TwistAxis) -> fmt::Result {
        match self.axis_names.get(axis.0 as usize) {
            Some(s) => write!(f, "{s}"),
            None => write!(f, "{}", axis.0),
        }
    }
    fn format_direction(
        &self,
        f: &mut fmt::Formatter<'_>,
        axis: TwistAxis,
        dir: TwistDirection,
    ) -> fmt::Result {
        match self.direction_names.get(dir.0 as usize) {
            Some(d) => write!(f, "{}", d.for_axis(axis)),
            None => write!(f, "{}", dir.0),
        }
    }

    pub fn parse_twist(&self, s: &str) -> Result<Twist, String> {
        const GENERIC_ERR_MSG: &str = "error parsing twist";

        // Check for aliases.
        let matching_alias = strip_any_prefix(
            s,
            self.aliases
                .iter()
                .map(|(alias_str, alias)| (alias, alias_str)),
        );
        if let Some((&alias, remaining)) = matching_alias {
            match alias {
                Alias::AxisLayers(axis, layers) => {
                    let direction = self.parse_twist_direction(axis, remaining)?;
                    Ok(Twist {
                        axis,
                        direction,
                        layers,
                    })
                }
                Alias::EntireTwist(twist) => {
                    if remaining.is_empty() {
                        Ok(twist)
                    } else {
                        Err(GENERIC_ERR_MSG.to_string())
                    }
                }
            }
        } else {
            // Parse layer mask if present.
            let (prefix_layers, remaining) = self.strip_layer_mask_prefix(s)?;
            let mut layers = prefix_layers.unwrap_or_default();
            // Parse twist axis.
            let (axis, mut remaining) =
                strip_any_prefix(remaining, self.axis_names.iter().enumerate())
                    .ok_or_else(|| GENERIC_ERR_MSG.to_string())?;
            let axis = TwistAxis(axis as u8);
            if let Some(block_suffix) = &self.block_suffix {
                if let Some(after_block_suffix) = remaining.strip_prefix(block_suffix) {
                    remaining = after_block_suffix;
                    let leading_zeros = prefix_layers.unwrap_or(LayerMask(3)).0.leading_zeros();
                    layers = LayerMask(u32::MAX >> leading_zeros);
                }
            }
            // Parse twist direction.
            let direction = self.parse_twist_direction(axis, remaining)?;

            Ok(Twist {
                axis,
                direction,
                layers,
            })
        }
    }

    fn parse_twist_direction(
        &self,
        axis: TwistAxis,
        string: &str,
    ) -> Result<TwistDirection, String> {
        match self
            .direction_names
            .iter()
            .find_position(|name| name.for_axis(axis) == string)
        {
            Some((i, _)) => Ok(TwistDirection(i as _)),
            None => Err("invalid twist direction".to_string()),
        }
    }

    fn strip_layer_mask_prefix<'a>(
        &self,
        string: &'a str,
    ) -> Result<(Option<LayerMask>, &'a str), String> {
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
            Some(captures) => (
                Some(captures[1].parse::<LayerMask>()?),
                captures.get(2).unwrap().as_str(), // need `.get()` for lifetime reasons
            ),
            None => (None, string),
        })
    }
}

fn strip_any_prefix<'a, 'b, T>(
    s: &'a str,
    possible_prefixes: impl IntoIterator<Item = (T, impl 'b + AsRef<str>)>,
) -> Option<(T, &'a str)> {
    possible_prefixes
        .into_iter()
        .find_map(|(value, prefix)| Some((value, s.strip_prefix(prefix.as_ref())?)))
}

//! Common utility functions that didn't fit anywhere else.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use itertools::Itertools;

/// Returns an iterator over the strings `A`, `B`, `C`, ..., `Z`, `AA`, `AB`,
/// ..., `ZY`, `ZZ`, `AAA`, `AAB`, etc.
pub(crate) fn iter_uppercase_letter_names() -> impl Iterator<Item = String> {
    (1..).flat_map(|len| {
        (0..26_usize.pow(len)).map(move |i| {
            (0..len)
                .rev()
                .map(|j| ('A' as u8 + ((i / 26_usize.pow(j)) % 26) as u8) as char)
                .collect()
        })
    })
}

/// Titlecases a string, replacing underscore `_` with space.
pub fn titlecase(s: &str) -> String {
    s.split(&[' ', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            if let Some((char_boundary, _)) = word.char_indices().skip(1).next() {
                let (left, right) = word.split_at(char_boundary);
                left.to_uppercase() + right
            } else {
                word.to_uppercase()
            }
        })
        .join(" ")
}

/// Lazily resolves a set of dependencies.
pub(crate) fn lazy_resolve<K: fmt::Debug + Clone + Eq + Hash, V: Clone>(
    key_value_dependencies: Vec<(K, (V, Option<K>))>,
    compose: fn(V, &V) -> V,
    warn_fn: impl Fn(String),
) -> HashMap<K, V> {
    // Some values are given directly.
    let mut known = Vec::<(K, V)>::new();
    // Some must be computed based on other values.
    let mut unknown = HashMap::<K, Vec<(K, V)>>::new();

    for (k, (v, other_key)) in key_value_dependencies {
        match other_key {
            Some(k2) => unknown.entry(k2).or_default().push((k, v)),
            None => known.push((k, v)),
        }
    }

    let mut known: HashMap<K, V> = known.iter().cloned().collect();

    // Resolve lazy evaluation.
    let mut queue = known.iter().map(|(k, _v)| k.clone()).collect_vec();
    while let Some(next_known) = queue.pop() {
        if let Some(unprocessed) = unknown.remove(&next_known) {
            for (k, v) in unprocessed {
                let value = compose(v, &known[&next_known]);
                known.insert(k.clone(), value);
                queue.push(k);
            }
        }
    }
    if let Some(unprocessed_key) = unknown.keys().next() {
        if unknown.contains_key(unprocessed_key) {
            warn_fn(format!("circular dependency on key {unprocessed_key:?}"));
        } else {
            warn_fn(format!("unknown key {unprocessed_key:?}"));
        }
    }

    known
}

/// Escapes a string to be used as a key in a Lua table literal.
pub fn escape_lua_table_key(s: &str) -> Cow<'_, str> {
    const RESERVED_WORDS: &[&str] = &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if",
        "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    ];
    if s.is_empty()
        || s.starts_with(|c: char| c.is_ascii_digit())
        || s.chars().any(|c| !c.is_alphanumeric() && c != '_')
        || RESERVED_WORDS.contains(&s)
    {
        format!("[{}]", lua_string_literal(s)).into()
    } else {
        s.into()
    }
}

/// Escapes a string to be used as a Lua string literal.
pub fn lua_string_literal(s: &str) -> String {
    // I'm not sure whether Rust string escaping works for Lua, but it's not the
    // worst thing if this generates invalid Lua code.
    let s = format!("{s:?}");

    // Prefer single quotes as a matter of style
    if s.contains("'") {
        s
    } else {
        format!("'{}'", &s[1..s.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_letter_names() {
        let mut it = iter_uppercase_letter_names();
        assert_eq!(it.next().unwrap(), "A");
        assert_eq!(it.next().unwrap(), "B");
        let mut it = it.skip(23);
        assert_eq!(it.next().unwrap(), "Z");
        assert_eq!(it.next().unwrap(), "AA");
        assert_eq!(it.next().unwrap(), "AB");
        let mut it = it.skip(23);
        assert_eq!(it.next().unwrap(), "AZ");
        assert_eq!(it.next().unwrap(), "BA");
        assert_eq!(it.next().unwrap(), "BB");
        assert_eq!(it.next().unwrap(), "BC");
        let mut it = it.skip(645);
        assert_eq!(it.next().unwrap(), "ZY");
        assert_eq!(it.next().unwrap(), "ZZ");
        assert_eq!(it.next().unwrap(), "AAA");
        assert_eq!(it.next().unwrap(), "AAB");
    }

    #[test]
    fn test_titlecase() {
        assert_eq!(titlecase("  this_was a__triumph_"), "This Was A Triumph");
    }
}

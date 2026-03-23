//! Common utility functions that didn't fit anywhere else.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::io::Write;

use itertools::Itertools;
use rand::SeedableRng;
use sha2::Digest;

/// Returns a canonical RNG from a seed value.
pub fn rng_from_seed(seed: &str) -> std::io::Result<chacha20::ChaCha12Rng> {
    let mut sha256 = sha2::Sha256::new();
    sha256.write_all(&seed.len().to_le_bytes())?; // native endianness on x86 and Apple Silicon
    sha256.write_all(seed.as_bytes())?;
    let digest = sha256.finalize();
    Ok(chacha20::ChaCha12Rng::from_seed(
        <[u8; 32]>::try_from(&digest[..32]).expect("sha256 digest must be 32 bytes"),
    ))
}

/// Returns a random nonempty layer mask, or `None` if `max_layer == 0`.
pub fn random_layer_mask(
    rng: &mut dyn rand::Rng,
    layer_count: u16,
) -> Option<hypuz_notation::LayerMask> {
    if layer_count == 0 {
        None
    } else {
        let mut random_bits = std::iter::from_fn(|| Some(rng.next_u32()))
            .flat_map(|bits: u32| (0..u32::BITS).map(move |i| bits & (1 << i) != 0));
        std::iter::from_fn(|| hypuz_notation::LayerRange::all(layer_count))
            .map(|all_layers| {
                all_layers
                    .into_iter()
                    .filter(|_| random_bits.next().expect("end of random bits"))
                    .collect()
            })
            .find(|mask: &hypuz_notation::LayerMask| !mask.is_empty())
    }
}

// TODO: remove name functions. they should live in hypuz_notation

/// Returns an iterator over the strings `A`, `B`, `C`, ..., `Z`, `AA`, `AB`,
/// ..., `ZY`, `ZZ`, `AAA`, `AAB`, etc.
pub fn iter_uppercase_letter_names() -> impl Iterator<Item = String> {
    (1..).flat_map(|len| {
        (0..26_usize.pow(len)).map(move |i| {
            (0..len)
                .rev()
                .map(|j| (b'A' + ((i / 26_usize.pow(j)) % 26) as u8) as char)
                .collect()
        })
    })
}

/// Titlecases a string, replacing underscore `_` with space.
pub fn titlecase(s: &str) -> String {
    s.split(&[' ', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            if let Some((char_boundary, _)) = word.char_indices().nth(1) {
                let (left, right) = word.split_at(char_boundary);
                left.to_uppercase() + right
            } else {
                word.to_uppercase()
            }
        })
        .join(" ")
}

/// Lazily resolves a set of dependencies.
///
/// TODO: use TypedIndex instead of strings
pub fn lazy_resolve<K: fmt::Debug + Clone + Eq + Hash, V: Clone>(
    key_value_dependencies: impl IntoIterator<Item = (K, (V, Option<K>))>,
    compose: fn(V, &V) -> V,
    warn_fn: impl FnOnce(String),
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
    let mut queue = known.keys().cloned().collect_vec();
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

/// Constructs a vantage name with the standard format `r1:r2,r3:r4,r5:r6`
/// (using as many reference pairs as necessary to disambiguate).
///
/// The first reference in each pair is typically fixed.
pub fn vantage_name<'a>(axis_pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    axis_pairs
        .into_iter()
        .map(|(ax1, ax2)| format!("{ax1}:{ax2}"))
        .join(",")
}
/// Parses a vantage name with the standard format `r1:r2,r3:r4,r5:r6` (using as
/// many reference pairs as necessary to disambiguate). Returns `None` if the
/// name is invalid and unparseable.
///
/// The first reference in each pair is typically fixed.
pub fn parse_vantage_name(name: &str) -> Option<Vec<(&str, &str)>> {
    name.split(',').map(|s| s.split_once(':')).collect()
}

#[cfg(test)]
mod tests {
    use rand::RngExt;

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

    #[test]
    fn test_stable_deterministic_rng() {
        let mut rng = chacha20::ChaCha12Rng::from_seed((0..32).collect_array().unwrap());
        let a = rng.random::<[u64; 4]>();
        assert_eq!(
            a,
            [
                6829280927315210738,
                12268062495221155140,
                13566740668459520841,
                3898457950037656553
            ]
        );
    }
}

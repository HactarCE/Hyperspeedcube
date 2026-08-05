use std::collections::{BTreeMap, HashMap, HashSet, hash_map};
use std::ops::Index;
use std::sync::Arc;

use hyperpuzzle_core::{Axis, IndexOutOfRange, Names};
use hypuz_notation::charsets::CharSet;
use hypuz_notation::{Str, family::SequentialLowercaseName};
use hypuz_util::ti::{IndexOverflow, TiVec, TypedIndex};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use crate::NamedPoint;

/// Trie of strings, none of which is a prefix of any other.
///
/// This does not utilize path compression because the names it contains are
/// typically very short and have common prefixes.
#[derive(Debug, Clone)]
pub enum PrefixFreeTrie<T> {
    /// Leaf node.
    Leaf(T),
    /// Empty root or nonempty branch.
    Branch(BTreeMap<char, PrefixFreeTrie<T>>),
}

impl<T> Default for PrefixFreeTrie<T> {
    fn default() -> Self {
        Self::Branch(BTreeMap::new())
    }
}

impl<T> PrefixFreeTrie<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn arbitrary_element(&self, mut prefix: Str) -> Option<(Str, &T)> {
        match self {
            PrefixFreeTrie::Leaf(v) => Some((prefix, v)),
            PrefixFreeTrie::Branch(map) => {
                let (c, next) = map.iter().next()?;
                prefix.push(*c);
                next.arbitrary_element(prefix)
            }
        }
    }

    /// Returns an arbitrary element that starts with the given character, or
    /// `None` if there is none.
    ///
    /// If there is a single leaf corresponding to the empty string, it is
    /// returned.
    pub fn arbitrary_element_with_prefix(&self, c: char) -> Option<(Str, &T)> {
        match self {
            PrefixFreeTrie::Leaf(v) => Some((c.into(), v)),
            PrefixFreeTrie::Branch(map) => map.get(&c)?.arbitrary_element(c.into()),
        }
    }

    /// Returns whether there is any element that starts with the given
    /// character.
    ///
    /// Returns true if there is a single leaf corresponding to the empty
    /// string.
    pub fn contains_prefix(&self, c: char) -> bool {
        match self {
            PrefixFreeTrie::Leaf(_) => true,
            PrefixFreeTrie::Branch(map) => map.contains_key(&c),
        }
    }

    fn arbitrary_key_infallible(&self, prefix: Str) -> Str {
        match self.arbitrary_element(prefix) {
            Some((key, _)) => key,
            None => "<unknown>".into(),
        }
    }

    pub fn insert(&mut self, key: &str, value: T) -> Result<(), StrTrieError> {
        self.insert_subtree(key, PrefixFreeTrie::Leaf(value))
    }

    pub fn insert_subtree(
        &mut self,
        key: &str,
        new_subtree: PrefixFreeTrie<T>,
    ) -> Result<(), StrTrieError> {
        self.insert_subtree_from_index(key, 0, new_subtree)
    }

    fn insert_subtree_from_index(
        &mut self,
        key: &str,
        index: usize,
        new_subtree: PrefixFreeTrie<T>,
    ) -> Result<(), StrTrieError> {
        match (key[index..].chars().next(), &mut *self) {
            (None, PrefixFreeTrie::Leaf(_)) => Err(StrTrieError::SameName(key.into())),
            (None, PrefixFreeTrie::Branch(_)) => match self.arbitrary_element(key.into()) {
                None => {
                    *self = new_subtree;
                    Ok(())
                }
                Some((k, _)) => Err(StrTrieError::NotPrefixFree {
                    prefix: key.into(),
                    word: k,
                }),
            },
            (Some(c), PrefixFreeTrie::Leaf(_)) => {
                if key[index + c.len_utf8()..].is_empty() {
                    Err(StrTrieError::NotPrefixFree {
                        prefix: key[..index].into(),
                        word: key.into(),
                    })
                } else {
                    Err(StrTrieError::NotPrefixFree {
                        prefix: key[..index + c.len_utf8()].into(),
                        word: key.into(),
                    })
                }
            }
            (Some(c), PrefixFreeTrie::Branch(map)) => map
                .entry(c)
                .or_default()
                .insert_subtree_from_index(key, index + c.len_utf8(), new_subtree),
        }
    }

    pub fn merge(mut self, other: Self) -> Result<Self, StrTrieError>
    where
        T: Clone,
    {
        // this could be better optimized but it doesn't really matter
        for (k, v) in other.to_vec() {
            self.insert(&k, v.clone())?;
        }
        Ok(self)
    }

    pub fn get(&self, s: &str) -> Option<&T> {
        let (c, rest) = self.split_off(s)?;
        rest.is_empty().then_some(c)
    }

    pub fn split_off<'a, 'b>(&'a self, s: &'b str) -> Option<(&'a T, &'b str)> {
        match self {
            PrefixFreeTrie::Leaf(v) => Some((v, s)),
            PrefixFreeTrie::Branch(map) => {
                let c = s.chars().next()?;
                map.get(&c)?.split_off(&s[c.len_utf8()..])
            }
        }
    }

    pub fn to_vec(&self) -> Vec<(Str, &T)> {
        let mut out = vec![];
        self.flatten_into(&mut out, &mut Str::new());
        out
    }
    fn flatten_into<'a>(&'a self, out: &mut Vec<(Str, &'a T)>, prefix: &mut Str) {
        match self {
            PrefixFreeTrie::Leaf(v) => out.push((prefix.clone(), v)),
            PrefixFreeTrie::Branch(map) => {
                for (c, v) in map {
                    prefix.push(*c);
                    v.flatten_into(out, prefix);
                    prefix.pop();
                }
            }
        }
    }

    pub fn map<U>(self, f: impl Copy + Fn(T) -> U) -> PrefixFreeTrie<U> {
        match self {
            PrefixFreeTrie::Leaf(v) => PrefixFreeTrie::Leaf(f(v)),
            PrefixFreeTrie::Branch(map) => {
                PrefixFreeTrie::Branch(map.into_iter().map(|(k, v)| (k, v.map(f))).collect())
            }
        }
    }

    pub fn try_map_ref<U, E>(
        &self,
        f: impl Copy + Fn(&T) -> Result<U, E>,
    ) -> Result<PrefixFreeTrie<U>, E> {
        match self {
            PrefixFreeTrie::Leaf(v) => Ok(PrefixFreeTrie::Leaf(f(v)?)),
            PrefixFreeTrie::Branch(map) => Ok(PrefixFreeTrie::Branch(
                map.iter()
                    .map(|(k, v)| Ok((*k, v.try_map_ref(f)?)))
                    .try_collect()?,
            )),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum StrTrieError {
    #[error("duplicate entry {0:?}")]
    SameName(Str),
    #[error("{prefix:?} is a prefix of {word:?}")]
    NotPrefixFree { prefix: Str, word: Str },
}

/// Bidirectional map that stores a single prefix-free name for each index.
#[derive(Debug, Default, Clone)]
pub struct PrefixFreeBiMap<I> {
    id_to_str: TiVec<I, Str>,
    str_to_id: PrefixFreeTrie<I>,
}

impl<I: TypedIndex> Index<I> for PrefixFreeBiMap<I> {
    type Output = Str;

    fn index(&self, index: I) -> &Self::Output {
        &self.id_to_str[index]
    }
}

impl<I: TypedIndex> PrefixFreeBiMap<I> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, name: Str) -> eyre::Result<I> {
        let index = self.id_to_str.next_idx()?;
        self.str_to_id.insert(&name, index)?;
        Ok(self.id_to_str.push(name)?)
    }

    /// Returns the number of names in the map.
    pub fn len(&self) -> usize {
        self.id_to_str.len()
    }

    pub fn get_name(&self, index: I) -> Result<&Str, IndexOutOfRange> {
        self.id_to_str.get(index)
    }
    pub fn get_index(&self, name: &str) -> Option<I> {
        self.str_to_id.get(name).copied()
    }

    pub fn split_off<'s>(&self, s: &'s str) -> Option<(I, &'s str)> {
        self.str_to_id.split_off(s).map(|(&i, rest)| (i, rest))
    }

    /// Returns a disjoint union of names. If there are multiple addends, they
    /// are differentiated by prepending a [`SequentialLowercaseName`] to every
    /// name. If there is only one addend, the names are left unmodified.
    pub fn disjoint_union(addends: &[&Self]) -> Result<Self, IndexOverflow> {
        match addends {
            [] => return Ok(Self::new()),
            [a] => return Ok((*a).clone()),
            _ => (),
        }

        let mut ret = Self::new();
        let mut offset = 0;
        for (index, addend) in addends.iter().enumerate() {
            let prefix = SequentialLowercaseName(index as u32).to_string();
            ret.id_to_str.extend(
                addend
                    .id_to_str
                    .iter_values()
                    .map(|s| format!("{prefix}{s}").into()),
            );
            let new_subtree = ret.str_to_id.try_map_ref(|i| i.offset_index(offset))?;
            ret.str_to_id.insert_subtree(&prefix, new_subtree);
            offset += addend.len();
        }
        Ok(ret)
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum NameError {
    #[error("{0}")]
    IndexOverflow(#[from] IndexOverflow),
    #[error("named point name cannot be empty")]
    EmptyNamedPoint,
    #[error("axis name cannot be empty")]
    EmptyAxis,
    #[error(
        "conflicting permutation patterns for axes {axis1:?} and {axis2:?}; \
         try using a distinct prefix for one of them"
    )]
    ConflictingAxisNamePatterns { axis1: Str, axis2: Str },
    #[error(
        "named point name {name:?} contains {char:?}; only \
         uppercase Latin and uppercase Greek are allowed"
    )]
    BadNamedPointChar { char: char, name: Str },
    #[error(
        "named point name {name:?} ends with {char:?}; \
         must end with uppercase Latin"
    )]
    BadNamedPointEndChar { char: char, name: Str },
    #[error(
        "axis prefix contains char {0:?}, which
         is also used in at least one named point"
    )]
    AxisPrefixCharConflictsWithNamedPoint(char),
    #[error("{0:?} does not start with a named point")]
    DoesNotStartWithNamedPoint(Str),
    #[error("no axis with prefix {prefix:?} followed by {len} named points")]
    UnknownAxisNamePattern { prefix: Str, len: usize },
    #[error("no axis with name {axis:?} (canonicalized to {canonicalized:?})")]
    UnknownAxis { axis: Str, canonicalized: Str },
    #[error("no named point with name {named_point:?}")]
    UnknownNamedPoint { named_point: Str },
}

/// Names for the named points and axes of a product puzzle twist system.
///
/// When there are multiple factors, each is assigned a unique
/// [`SequentialLowercaseName`].
#[derive(Debug, Default, Clone)]
pub struct ProductTwistSystemNames {
    pub factors: Vec<FactorTwistSystemNames>,
    /// Axis ID offset for each term.
    factor_axis_offsets: Vec<usize>,
}

impl ProductTwistSystemNames {
    pub fn product(factors: Vec<FactorTwistSystemNames>) -> Self {
        let mut factor_axis_offsets = vec![];
        let mut i = 0;
        for term in &factors {
            factor_axis_offsets.push(i);
            i += term.axis_count();
        }
        Self {
            factors,
            factor_axis_offsets,
        }
    }

    pub fn build_named_point_names(
        &self,
    ) -> Result<Names<NamedPoint>, hyperpuzzle_core::NameError> {
        Names::new_simple(prefixed_if_needed(
            self.factors
                .iter()
                .map(|term| &term.named_point_names.id_to_str),
        ))
    }

    pub fn build_axis_names(&self) -> Result<Names<Axis>, hyperpuzzle_core::NameError> {
        let needs_prefix = self.factors.len() != 1;

        Names::new(
            prefixed_if_needed(
                self.factors
                    .iter()
                    .map(|term| &term.canonical_axis_names.id_to_name),
            ),
            {
                let this = self.clone();
                move |s| {
                    let (SequentialLowercaseName(i), rest) = if needs_prefix {
                        hypuz_notation::family::strip_sequential_lowercase_prefix(s)?
                    } else {
                        (SequentialLowercaseName(0), s)
                    };
                    let &offset = this.factor_axis_offsets.get(i as usize)?;
                    let term = this.factors.get(i as usize)?;
                    term.axis_from_name(rest).ok()?.offset_index(offset).ok()
                }
            },
        )
    }
}

fn prefixed_if_needed<'a, I: TypedIndex>(
    iter: impl ExactSizeIterator + Iterator<Item = &'a TiVec<I, Str>>,
) -> TiVec<I, Str> {
    let needs_prefix = iter.len() != 1;
    iter.enumerate()
        .flat_map(|(i, names)| {
            let prefix = SequentialLowercaseName(i as u32);
            names.iter_values().map(move |s| {
                if needs_prefix {
                    format!("{prefix}{s}").into()
                } else {
                    s.clone()
                }
            })
        })
        .collect()
}

#[derive(Debug, Default, Clone)]
pub struct FactorTwistSystemNames {
    /// Characters used in name point names.
    named_point_chars: HashSet<char>,
    /// Named points, which are used to name axes.
    named_point_names: Arc<PrefixFreeBiMap<NamedPoint>>,
    /// Axis name patterns.
    ///
    /// An axis name pattern says which indices of named points in the name must
    /// be permuted to canonicalize the axis name.
    ///
    /// The axis prefix must only use characters that do not start a named
    /// point. In practice, named points typically use uppercase letters and
    /// axis prefixes typically use lowercase Latin letters.
    axis_name_patterns: HashMap<AxisNamePatternKey, AxisNamePatternValue>,
    /// Map between axes and canonical names.
    canonical_axis_names: NameBiMap<Axis>,
}

impl FactorTwistSystemNames {
    pub fn new(named_point_names: Arc<PrefixFreeBiMap<NamedPoint>>) -> Result<Self, NameError> {
        // Validate named point names
        for (_, name) in &named_point_names.id_to_str {
            if let Some(char) = name.chars().find(|&c| {
                !matches!(
                    hypuz_notation::charsets::classify(c),
                    Some(CharSet::UppercaseGreek | CharSet::UppercaseLatin)
                )
            }) {
                return Err(NameError::BadNamedPointChar {
                    char,
                    name: name.clone(),
                });
            }

            let Some(last_char) = name.chars().last() else {
                return Err(NameError::EmptyNamedPoint);
            };
            if !hypuz_notation::charsets::is_latin_letter(last_char) {
                return Err(NameError::BadNamedPointEndChar {
                    char: last_char,
                    name: name.clone(),
                });
            }
        }

        if named_point_names.str_to_id.get("").is_some() {}
        let named_point_chars = named_point_names
            .id_to_str
            .iter_values()
            .flat_map(|s| s.chars())
            .collect();
        Ok(Self {
            named_point_chars,
            named_point_names,
            axis_name_patterns: HashMap::new(),
            canonical_axis_names: NameBiMap::new(),
        })
    }

    pub fn named_point_count(&self) -> usize {
        self.named_point_names.len()
    }
    pub fn axis_count(&self) -> usize {
        self.canonical_axis_names.len()
    }

    /// Adds an axis.
    ///
    /// - `prefix` must not use any characters that start a named point name.
    /// - `named_point_sets` is a list of sets of named points. The named points
    ///   within a set can be freely permuted. The name is canonicalized by
    ///   sorting the named points within each set.
    ///
    /// A name is formed by concatenating the prefix and all the named points
    /// with no separators.
    ///
    /// Returns an error if the canonical name conflicts with
    pub fn add_axis(
        &mut self,
        prefix: &str,
        mut named_point_sets: Vec<Vec<NamedPoint>>,
    ) -> Result<Axis, NameError> {
        // Keep axis prefixes maximally distinguishable from named points for
        // parsing reasons.
        if let Some(c) = prefix.chars().find(|c| self.named_point_chars.contains(c)) {
            return Err(NameError::AxisPrefixCharConflictsWithNamedPoint(c));
        }

        // Canonicalize axis name
        for set in &mut named_point_sets {
            set.sort();
        }
        let new_canonical_axis_name = format!(
            "{prefix}{}",
            named_point_sets
                .iter()
                .flatten()
                .map(|&p| &self.named_point_names[p])
                .join(""),
        )
        .into();

        let set_sizes = named_point_sets.iter().map(|set| set.len()).collect_vec();
        let axis_name_pattern_key = AxisNamePatternKey {
            prefix: prefix.into(),
            len: set_sizes.iter().sum::<usize>(),
        };
        match self.axis_name_patterns.entry(axis_name_pattern_key) {
            hash_map::Entry::Occupied(e) => {
                if e.get().set_sizes != set_sizes {
                    return Err(NameError::ConflictingAxisNamePatterns {
                        axis1: e.get().example.clone(),
                        axis2: new_canonical_axis_name,
                    });
                }
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(AxisNamePatternValue {
                    set_sizes,
                    example: new_canonical_axis_name.clone(),
                });
            }
        }

        Ok(self.canonical_axis_names.push(new_canonical_axis_name)?)
    }

    pub fn named_point_from_name(&self, named_point_name: &str) -> Result<NamedPoint, NameError> {
        self.named_point_names
            .get_index(named_point_name)
            .ok_or(NameError::UnknownNamedPoint {
                named_point: named_point_name.into(),
            })
    }

    pub fn axis_from_name<'a>(&self, axis_name: &str) -> Result<Axis, NameError> {
        if axis_name.is_empty() {
            return Err(NameError::EmptyAxis);
        }

        let prefix_len = axis_name
            .char_indices()
            .find(|(_, c)| self.named_point_names.str_to_id.contains_prefix(*c))
            .map(|(i, _)| i)
            .unwrap_or(axis_name.len());

        let (prefix, mut named_points_str) = axis_name.split_at(prefix_len);

        let mut named_points: SmallVec<[NamedPoint; 8]> = smallvec![];
        while !named_points_str.is_empty() {
            let (&named_point, rest) = self
                .named_point_names
                .str_to_id
                .split_off(named_points_str)
                .ok_or_else(|| NameError::DoesNotStartWithNamedPoint(named_points_str.into()))?;
            assert!(
                rest.len() < named_points_str.len(),
                "infinite loop while parsing axis name",
            );
            named_points.push(named_point);
            named_points_str = rest;
        }

        let name_pattern = self
            .axis_name_patterns
            .get(&AxisNamePatternKey {
                prefix: prefix.into(),
                len: named_points.len(),
            })
            .ok_or_else(|| NameError::UnknownAxisNamePattern {
                prefix: prefix.into(),
                len: named_points.len(),
            })?;

        name_pattern.canonicalize(&mut named_points);

        let mut canonicalized_axis_name: Str = prefix.into();
        for p in named_points {
            canonicalized_axis_name += &*self.named_point_names[p];
        }

        self.canonical_axis_names
            .name_to_id(&canonicalized_axis_name)
            .ok_or_else(|| NameError::UnknownAxis {
                axis: axis_name.into(),
                canonicalized: canonicalized_axis_name,
            })
    }
}

/// Key for an axis name pattern.
///
/// This can be discerned simply by parsing the axis name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AxisNamePatternKey {
    /// Prefix before the list of named points.
    ///
    /// The prefix must not conflict with the name of any named point.
    prefix: Str,
    /// Number of named points in the axis name.
    len: usize,
}

/// Description of an axis name pattern, along with [`AxisNamePatternKey`].
///
/// After parsing an axis name, this tells how to canonicalize it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AxisNamePatternValue {
    /// Sizes of sets into which to partition the list of named points.
    ///
    /// For example, if `set_sizes` is `vec![1, 3, 2]` for an axis named
    /// `XBCARQ` (where each letter is a distinct named point), then the axis
    /// name would be partitioned into the sets `X`, `BCA`, `RQ`. Sorting each
    /// set individually yields `X`, `ABC`, `QR`. Concatenating them all yields
    /// `XABCQR`, which is the canonical name for the axis.
    set_sizes: Vec<usize>,
    /// Example axis using this pattern, for generating error messages.
    example: Str,
}
impl AxisNamePatternValue {
    pub fn canonicalize<T: Clone + Ord>(&self, elems: &mut [T]) {
        let mut i = 0;
        for set_size in &self.set_sizes {
            let j = i + set_size;
            elems[i..j].sort();
            i = j;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_trie() {
        let mut trie = PrefixFreeTrie::new();
        assert_eq!(trie.split_off(""), None);
        assert_eq!(trie.split_off("a"), None);
        assert_eq!(trie.split_off("ab"), None);
        assert_eq!(trie.insert("", 0), Ok(()));
        assert_eq!(trie.split_off("abc"), Some((&0, "abc")));
        let mut trie = PrefixFreeTrie::new();
        assert_eq!(trie.insert("a", 1), Ok(()));
        assert_eq!(trie.split_off(""), None);
        assert_eq!(trie.split_off("a"), Some((&1, "")));
        assert_eq!(trie.split_off("ab"), Some((&1, "b")));
        assert_eq!(trie.insert("a", 2), Err(StrTrieError::SameName("a".into())));
        assert_eq!(
            trie.insert("ab", 2),
            Err(StrTrieError::NotPrefixFree {
                prefix: "a".into(),
                word: "ab".into(),
            }),
        );
        assert_eq!(trie.insert("ba", 3), Ok(()));
        assert_eq!(
            trie.insert("b", 4),
            Err(StrTrieError::NotPrefixFree {
                prefix: "b".into(),
                word: "ba".into(),
            }),
        );
        assert_eq!(trie.insert("bcd", 5), Ok(()));
        assert_eq!(trie.insert("bce", 6), Ok(()));

        assert_eq!(
            trie.insert("b", 4),
            Err(StrTrieError::NotPrefixFree {
                prefix: "b".into(),
                word: "ba".into(),
            }),
        );

        assert_eq!(trie.split_off("abcde"), Some((&1, "bcde")));
        assert_eq!(trie.split_off("bcde"), Some((&5, "e")));
        assert_eq!(trie.split_off("bced"), Some((&6, "d")));
        assert_eq!(
            trie.to_vec(),
            vec![
                ("a".into(), &1),
                ("ba".into(), &3),
                ("bcd".into(), &5),
                ("bce".into(), &6),
            ],
        );
    }
}

//
//
//
//
//
//

#[derive(Debug, Clone)]
pub struct NameBiMap<I> {
    id_to_name: TiVec<I, Str>,
    name_to_id: HashMap<Str, I>,
}

impl<I> Default for NameBiMap<I> {
    fn default() -> Self {
        Self {
            id_to_name: Default::default(),
            name_to_id: Default::default(),
        }
    }
}

impl<I: TypedIndex> NameBiMap<I> {
    pub fn new() -> Self {
        Self {
            id_to_name: TiVec::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn concat(a: &Self, b: &Self) -> Self {
        let lift_a = |i: I| i;
        let lift_b = |i: I| I::try_from_index(i.to_index() + a.len()).expect("overflow");
        Self {
            id_to_name: crate::chain_cloned(a.id_to_name.iter_values(), b.id_to_name.iter_values()),
            name_to_id: std::iter::chain(
                a.name_to_id
                    .iter()
                    .map(|(a_name, &a_index)| (a_name.clone(), lift_a(a_index))),
                b.name_to_id
                    .iter()
                    .map(|(b_name, &b_index)| (b_name.clone(), lift_b(b_index))),
            )
            .collect(),
        }
    }

    pub fn push(&mut self, name: Str) -> Result<I, IndexOverflow> {
        let id = self.id_to_name.push(name.clone())?;
        self.name_to_id.insert(name, id);
        Ok(id)
    }

    pub fn len(&self) -> usize {
        self.id_to_name.len()
    }

    pub fn id_to_name(&self) -> &TiVec<I, Str> {
        &self.id_to_name
    }

    pub fn name_to_id(&self, name: &str) -> Option<I> {
        self.name_to_id.get(name).copied()
    }
}

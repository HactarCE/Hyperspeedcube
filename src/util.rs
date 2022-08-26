use cgmath::Point3;
use itertools::Itertools;
use std::ops::{Add, Mul};

pub const INVALID_STR: &str = "<invalid>";

pub struct CyclicPairsIter<I: Iterator> {
    first: Option<I::Item>,
    prev: Option<I::Item>,
    rest: I,
}
impl<I> Iterator for CyclicPairsIter<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.rest.next() {
            Some(curr) => (self.prev.replace(curr.clone())?, curr),
            None => (self.prev.take()?, self.first.take()?),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.rest.size_hint();
        (lo.saturating_add(1), hi.and_then(|x| x.checked_add(1)))
    }
}

pub trait IterCyclicPairsExt: Iterator + Sized {
    fn cyclic_pairs(self) -> CyclicPairsIter<Self>;
}
impl<I> IterCyclicPairsExt for I
where
    I: Iterator,
    I::Item: Clone,
{
    fn cyclic_pairs(mut self) -> CyclicPairsIter<Self> {
        let first = self.next();
        let prev = first.clone();
        CyclicPairsIter {
            first,
            prev,
            rest: self,
        }
    }
}

pub fn min_and_max_bound(verts: &[Point3<f32>]) -> (Point3<f32>, Point3<f32>) {
    let mut min_bound = verts[0];
    let mut max_bound = verts[0];

    for v in &verts[1..] {
        if v.x < min_bound.x {
            min_bound.x = v.x;
        }
        if v.y < min_bound.y {
            min_bound.y = v.y;
        }
        if v.z < min_bound.z {
            min_bound.z = v.z;
        }

        if v.x > max_bound.x {
            max_bound.x = v.x;
        }
        if v.y > max_bound.y {
            max_bound.y = v.y;
        }
        if v.z > max_bound.z {
            max_bound.z = v.z;
        }
    }

    (min_bound, max_bound)
}

pub fn wrap_words<S: AsRef<str>>(words: impl Iterator<Item = S>) -> String {
    const WORD_WRAP_WIDTH: usize = 70;
    let mut ret = String::new();
    let mut column = 0;
    for word in words {
        let word = word.as_ref();
        if column == 0 {
            column += word.len();
            ret += word;
        } else {
            column += word.len() + 1;
            if column <= WORD_WRAP_WIDTH {
                ret += " ";
            } else {
                column = word.len();
                ret += "\n";
            }
            ret += word;
        }
    }
    ret
}

pub fn mix<T>(a: T, b: T, t: f32) -> <T::Output as Add>::Output
where
    T: Mul<f32>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

pub fn b16_encode_bools(bits: impl IntoIterator<Item = bool>) -> String {
    bits.into_iter()
        .chunks(4)
        .into_iter()
        .map(|mut chunk| {
            let nibble = (0..4)
                .map(|i| (chunk.next().unwrap_or(false) as u32) << i)
                .sum();
            char::from_digit(nibble, 16).unwrap_or('?')
        })
        .collect()
}
pub fn b16_fetch_bit<'a>(s: &str, bit_idx: usize) -> bool {
    s.as_bytes()
        .get(bit_idx / 4)
        .map(|&ch| b16_decode_char(ch as char)[bit_idx & 3])
        .unwrap_or(false)
}
fn b16_decode_char(ch: char) -> [bool; 4] {
    let nibble = ch.to_digit(16).unwrap_or(0);
    [
        nibble & 1 != 0,
        nibble & 2 != 0,
        nibble & 4 != 0,
        nibble & 8 != 0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b16_encode_decode() {
        let s = "f4add8920abe83143362";
        assert_eq!(s, b16_encode_bools((0..78).map(|i| b16_fetch_bit(s, i))));
    }
}

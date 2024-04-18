use cgmath::Point3;

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

/// Converts an `egui::Color32` to a `[u8; 3]`, ignoring alpha.
pub(crate) fn color_to_u8x3(color: impl Into<egui::Color32>) -> [u8; 3] {
    let [r, g, b, _a] = color.into().to_array();
    [r, g, b]
}

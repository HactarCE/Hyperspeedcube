use cgmath::{Point3, SquareMatrix};

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

/// Returns the perspective-correct barycentric coordinates for the point `p` in
/// triangle `tri`, and a boolean indicating whether the triangle's backface is
/// visible (as opposed to its frontface). The Z coordinate is ignored; only X,
/// Y, and W are used.
///
/// Returns `None` if the point is not in the triangle.
///
/// This function uses the math described at
/// <https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/visibility-problem-depth-buffer-depth-interpolation.html>
pub fn triangle_hover_barycentric_coordinates(
    hover_pos: cgmath::Point2<f32>,
    tri: [cgmath::Vector4<f32>; 3],
) -> Option<([f32; 3], bool)> {
    // If any vertex is culled, skip the whole triangle.
    if tri.iter().any(|p| p.w == 0.0) {
        return None;
    }

    let mut tri_2d = tri.map(|p| cgmath::point2(p.x / p.w, p.y / p.w));

    // Ensure the triangle is counterclockwise.
    let mut total_area = triangle_area_2x(tri_2d);
    let rev = total_area < 0.0;
    if rev {
        tri_2d.reverse();
        total_area = -total_area;
    }

    // Compute the barycentric coordinates in screen space.
    let [a, b, c] = tri_2d;
    let h = hover_pos;
    let recip_total_area = total_area.recip();
    let qa = triangle_area_2x([h, b, c]) * recip_total_area;
    let qb = triangle_area_2x([a, h, c]) * recip_total_area;
    let qc = triangle_area_2x([a, b, h]) * recip_total_area;
    // If the point is inside the triangle ...
    let [ra, rb, _rc] = (qa > 0.0 && qb > 0.0 && qc > 0.0).then(|| {
        let [a, b, c] = tri;
        // ... then compute the perspective-correct W value
        let w = qa * a.w + qb * b.w + qc * c.w;
        // ... and use that to compute perspective-correct barycentric
        //     coordinates.
        let mut out = [qa * w / a.w, qb * w / b.w, qc * w / c.w];
        if rev {
            out.reverse();
        }
        out
    })?;

    // Ensure that the barycentric coordinates add to *exactly* one.
    Some(([ra, rb, 1.0 - ra - rb], rev))
}
/// Returns 2 times the area of a triangle.
pub fn triangle_area_2x([a, b, c]: [cgmath::Point2<f32>; 3]) -> f32 {
    cgmath::Matrix2::from_cols(b - a, b - c).determinant()
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

/// Converts an [`egui::Color32`] to a `[u8; 3]`, ignoring alpha.
pub(crate) fn color_to_u8x3(color: impl Into<egui::Color32>) -> [u8; 3] {
    let [r, g, b, _a] = color.into().to_array();
    [r, g, b]
}

/// Serializes a color to a hex string like `#ff00ff`.
pub(crate) fn color_to_hex_string(rgb: [u8; 3]) -> String {
    format!("#{}", hex::encode(rgb))
}

/// Deserializes a color from a hex string like `#ff00ff` or `#f0f`.
pub(crate) fn color_from_hex_str(s: &str) -> Result<[u8; 3], hex::FromHexError> {
    let mut rgb = [0_u8; 3];
    let s = s.strip_prefix('#').unwrap_or(s).trim();
    match s.len() {
        3 => {
            let s = &s.chars().flat_map(|c| [c, c]).collect::<String>();
            hex::decode_to_slice(&s, &mut rgb)?;
        }
        _ => hex::decode_to_slice(s, &mut rgb)?,
    }
    Ok(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclic_pairs_iter() {
        assert_eq!(
            [1, 2, 3, 4].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 4), (4, 1)],
        );
        assert_eq!(
            [1, 2, 3].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 1)],
        );
        assert_eq!(
            [1, 2].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 1)],
        );
        assert_eq!(
            [1].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 1)],
        );
    }
}

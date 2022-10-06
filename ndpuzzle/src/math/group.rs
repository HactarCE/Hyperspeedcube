use std::ops::Mul;

use approx::abs_diff_eq;

pub trait Group {
    fn compose(&self, other: &Self) -> Self;
    fn generate<T>(generators: &[Self], seeds: Vec<T>) -> Vec<T>
    where
        Self: Sized,
        for<'a> &'a Self: Mul<&'a T, Output = T>,
        T: approx::AbsDiffEq,
    {
        let mut ret = seeds;
        let mut unprocessed_idx = 0;
        while unprocessed_idx < ret.len() {
            for gen in generators {
                let new = gen * &ret[unprocessed_idx];
                if !ret.iter().any(|old| abs_diff_eq!(*old, new)) {
                    ret.push(new);
                }
            }
            unprocessed_idx += 1;
        }
        ret
    }
}
impl<T> Group for T
where
    for<'a> &'a T: Mul<Output = T>,
{
    fn compose(&self, other: &Self) -> Self {
        self * other
    }
}

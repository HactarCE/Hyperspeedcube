use regex::Regex;

/// Puzzle element that can be named.
pub trait Nameable {
    /// Returns a regex (including `^` and `$`) that matches a string iff it is
    /// a valid name for this kind of puzzle element.
    fn whole_name_regex() -> &'static Regex;
}
macro_rules! impl_nameable {
    ($type:ty, $regex_str:expr $(,)?) => {
        impl Nameable for $type {
            fn whole_name_regex() -> &'static Regex {
                lazy_static! {
                    static ref CACHED_REGEX: Regex =
                        Regex::new(concat!("^", $regex_str, "$")).expect("bad regex");
                }
                &*CACHED_REGEX
            }
        }
    };
}
impl_nameable!(crate::Color, r"[a-zA-Z_][a-zA-Z0-9_]*");
impl_nameable!(crate::Axis, r"[a-zA-Z_][a-zA-Z0-9_]*");
impl_nameable!(crate::PieceType, r"[a-zA-Z_][a-zA-Z0-9_]*(/[a-zA-Z0-9_]*)*");
impl_nameable!(crate::Twist, r"[a-zA-Z_][a-zA-Z0-9_]*'?");

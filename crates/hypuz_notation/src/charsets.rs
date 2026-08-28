//! Character sets allowed in notation.

/// Character sets allowed in family names.
pub enum CharSet {
    /// Lowercase Latin letters `'a'..='z'`
    LowercaseLatin,
    /// Uppercase Latin letters `'A'..='Z'`
    UppercaseLatin,
    /// [`UPPERCASE_GREEK`]
    UppercaseGreek,
    /// [`TALL_LOWERCASE_GREEK`]
    TallLowercaseGreek,
    /// [`SHORT_LOWERCASE_GREEK`]
    ShortLowercaseGreek,
    /// `_`
    Underscore,
}

/// Returns the character set that `c` belongs to, or `None` if it is not
/// allowed in family names.
pub fn classify(c: char) -> Option<CharSet> {
    match c {
        'a'..='z' => Some(CharSet::LowercaseLatin),
        'A'..='Z' => Some(CharSet::UppercaseLatin),
        'Γ' | 'Δ' | 'Θ' | 'Λ' | 'Ξ' | 'Π' | 'Σ' | 'Φ' | 'Ψ' | 'Ω' => {
            Some(CharSet::UppercaseGreek)
        }
        'β' | 'δ' | 'ζ' | 'θ' | 'λ' | 'ξ' => Some(CharSet::TallLowercaseGreek),
        'ε' | 'η' | 'κ' | 'μ' | 'π' | 'τ' | 'φ' | 'ψ' | 'ω' => {
            Some(CharSet::ShortLowercaseGreek)
        }
        _ => None,
    }
}

/// Returns whether `c` is a letter of the Latin alphabet.
pub fn is_latin_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Returns whether `c` is a letter of the Greek alphabet that is visually
/// distinct from Latin letters; i.e., it is in [`UPPERCASE_GREEK`],
/// [`SHORT_LOWERCASE_GREEK`], or [`TALL_LOWERCASE_GREEK`].
pub fn is_greek_letter(c: char) -> bool {
    matches!(
        classify(c),
        Some(CharSet::UppercaseGreek | CharSet::TallLowercaseGreek | CharSet::ShortLowercaseGreek),
    )
}

/// Returns whether `c` is a character allowed in a move or rotation family
/// name.
pub fn is_family_char(c: char) -> bool {
    classify(c).is_some()
}

/// Uppercase Greek letters that are visually distinct from Latin letters.
pub const UPPERCASE_GREEK: [char; 10] = ['Γ', 'Δ', 'Θ', 'Λ', 'Ξ', 'Π', 'Σ', 'Φ', 'Ψ', 'Ω'];
/// Short lowercase Greek letters that are visually distinct from Latin letters.
pub const SHORT_LOWERCASE_GREEK: [char; 9] = ['ε', 'η', 'κ', 'μ', 'π', 'τ', 'φ', 'ψ', 'ω'];
/// Tall lowercase Greek letters that are visually distinct from Latin letters.
pub const TALL_LOWERCASE_GREEK: [char; 6] = ['β', 'δ', 'ζ', 'θ', 'λ', 'ξ'];

/// Regex character class (not including the surrounding `[]`) matching all
/// characters for which [`is_family_char()`] returns true.
pub const FAMILY_CHAR_CLASS: &str = "[A-Za-zΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω]";

/// String containing all characters for which [`is_family_char()`]
/// returns true.
pub const FAMILY_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω";

/// String containing all group prefix characters.
pub const GROUP_PREFIX_CHARS: &str = "!#$%&?^`";

#[cfg(test)]
lazy_static::lazy_static! {
    pub(crate) static ref FAMILY_REGEX: &'static str = format!("{FAMILY_CHAR_CLASS}+").leak();
    pub(crate) static ref OPT_FAMILY_REGEX: &'static str = format!("{FAMILY_CHAR_CLASS}*").leak();
}

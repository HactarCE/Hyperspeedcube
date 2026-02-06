//! Character sets allowed in notation.

/// Returns whether `c` is a letter of the Latin alphabet.
pub fn is_latin_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Returns whether `c` is a letter of the Greek alphabet that is visually
/// distinct from Latin letters; i.e., it is in [`UPPERCASE_GREEK`],
/// [`SMALL_LOWERCASE_GREEK`], or [`LARGE_LOWERCASE_GREEK`].
pub fn is_greek_letter(c: char) -> bool {
    match c {
        // Greek uppercase
        'Γ' | 'Δ' | 'Θ' | 'Λ' | 'Ξ' | 'Π' | 'Σ' | 'Φ' | 'Ψ' | 'Ω' => true,
        // Greek lowercase (large)
        'β' | 'δ' | 'ζ' | 'θ' | 'λ' | 'ξ' => true,
        // Greek lowercase (small)
        'ε' | 'η' | 'κ' | 'μ' | 'π' | 'τ' | 'φ' | 'ψ' | 'ω' => true,
        _ => false,
    }
}

/// Returns whether `c` is a character allowed in a move or rotation family
/// name.
pub fn is_family_char(c: char) -> bool {
    is_latin_letter(c) || is_greek_letter(c) || c == '_'
}

/// Returns whether `c` is a character allowed in a bracketed transform,
/// including ` `.
pub fn is_bracketed_transform_char(c: char) -> bool {
    is_family_char(c) || matches!(c, ' ' | '0'..='9' | '\'' | '<' | '>' | '|' | '-')
}

/// Returns whether `c` is a jumbling suffix character `h`, `j`, or `k`.
pub fn is_jumbling_suffix(c: char) -> bool {
    matches!(c, 'h' | 'j' | 'k')
}

/// Uppercase Greek letters that are visually distinct from Latin letters.
pub const UPPERCASE_GREEK: [char; 10] = ['Γ', 'Δ', 'Θ', 'Λ', 'Ξ', 'Π', 'Σ', 'Φ', 'Ψ', 'Ω'];
/// Small lowercase Greek letters that are visually distinct from Latin letters.
pub const SMALL_LOWERCASE_GREEK: [char; 9] = ['ε', 'η', 'κ', 'μ', 'π', 'τ', 'φ', 'ψ', 'ω'];
/// Large lowercase Greek letters that are visually distinct from Latin letters.
pub const LARGE_LOWERCASE_GREEK: [char; 6] = ['β', 'δ', 'ζ', 'θ', 'λ', 'ξ'];

/// Regex character class (not including the surrounding `[]`) matching all
/// characters for which [`is_family_char()`] returns true.
pub const FAMILY_CHAR_CLASS: &str = "[A-Za-zΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω]";

/// String containing all characters for which [`is_family_char()`]
/// returns true.
pub const FAMILY_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω";

/// Regex character class (including the surrounding `[]`) matching all
/// characters for which [`is_family_char()`] returns true, **except for ` `
/// (space)**.
pub const BRACKETED_TRANSFORM_CHAR_CLASS_NO_SPACE: &str =
    r"[A-Za-zΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω0-9'<>|-]";

/// Regex character class (including the surrounding `[]`) matching all
/// characters for which [`is_family_char()`] returns true.
pub const BRACKETED_TRANSFORM_CHAR_CLASS: &str = "[ A-Za-zΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω0-9'<>|-]";

/// String containing all characters for which [`is_bracketed_transform_char()`]
/// returns true.
pub const BRACKETED_TRANSFORM_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzΓΔΘΛΞΠΣΦΨΩβδζθλξεηκμπτφψω 0123456789'-<>|";

/// String containing all group prefix characters.
pub const GROUP_PREFIX_CHARS: &str = "!#$%&?^`";

#[cfg(test)]
lazy_static::lazy_static! {
    pub(crate) static ref FAMILY_REGEX: &'static str = format!("{FAMILY_CHAR_CLASS}+").leak();
    pub(crate) static ref OPT_FAMILY_REGEX: &'static str = format!("{FAMILY_CHAR_CLASS}*").leak();
    pub(crate) static ref TRANSFORM_REGEX: &'static str = format!(
        "{BRACKETED_TRANSFORM_CHAR_CLASS_NO_SPACE}\
         ({BRACKETED_TRANSFORM_CHAR_CLASS}*\
          {BRACKETED_TRANSFORM_CHAR_CLASS_NO_SPACE})?"
    )
    .leak();
}

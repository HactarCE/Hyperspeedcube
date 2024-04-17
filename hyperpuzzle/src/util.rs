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
}

use std::fmt;

/// Semantic-ish version for a puzzle or puzzle generator.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch version number.
    pub patch: u32,
}
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            major,
            minor,
            patch,
        } = self;
        write!(f, "{major}.{minor}.{patch}")
    }
}
impl Version {
    pub const ZERO: Version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };
}

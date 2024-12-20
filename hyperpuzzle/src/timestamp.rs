use chrono::SubsecRound;

/// Type used for UTC timestamps in log files.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Timestamp(chrono::DateTime<chrono::Utc>);
impl ToString for Timestamp {
    fn to_string(&self) -> String {
        self.0.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}
impl std::str::FromStr for Timestamp {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> chrono::ParseResult<Self> {
        chrono::DateTime::from_str(s).map(Self)
    }
}
impl Timestamp {
    /// Returns the UTC timestamp for the present moment, according to the system
    /// clock.
    pub fn now() -> Self {
        Self(chrono::Utc::now().trunc_subsecs(3))
    }
}

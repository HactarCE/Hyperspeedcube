//! Verification of end times using a Time Stamp Authority.
//!
//! # Example
//!
//! ```
//! use chrono::{SubsecRound, Utc};
//! use sha2::Digest;
//!
//! // Globals shared by client & server
//! let tsa = timecheck::tsa::Tsa::FREETSA; // to verify end times
//! let allowed_clock_drift = chrono::TimeDelta::seconds(5); // drift between TSA and game client
//! # fn play_game() -> String { "Hello, world!".to_string() }
//!
//! // CLIENT SIDE:
//!
//! let gameplay_log = play_game();
//! // Sign digest at end of run
//! let digest = sha2::Sha256::digest(gameplay_log.as_bytes());
//! let time_immediately_before_ending_run = Utc::now().trunc_subsecs(0) - allowed_clock_drift;
//! let signature = tsa.timestamp(&digest)?;
//! let time_immediately_after_ending_run = Utc::now().trunc_subsecs(0) + allowed_clock_drift;
//! // Save `gameplay_log`, and `signature` in the log file
//!
//! ////////////////////
//!
//! // SERVER SIDE:
//!
//! // Verify end time
//! tsa.verify(&digest, &signature)?;
//! let verified_end_time = signature.timestamp()?;
//! println!("{time_immediately_before_ending_run}..{time_immediately_after_ending_run}");
//! println!("{verified_end_time}");
//! assert!(
//!     (time_immediately_before_ending_run..=time_immediately_after_ending_run)
//!         .contains(&verified_end_time)
//! );
//! # Ok::<(), timecheck::tsa::TsaError>(())
//! ```

use std::{borrow::Cow, error::Error, fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tsp_http_client::TimeStampResponse;

#[allow(missing_docs)]
pub type Result<T, E = TsaError> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum TsaError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "openssl error: stdout={stdout:?} stderr={stderr:?}. time stamp authority public key may have expired."
    )]
    OpenSsl { stdout: String, stderr: String },
    #[error("tsp error: {0}")]
    Tsp(#[from] tsp_http_client::Error),

    #[error("{0}")]
    Other(Box<dyn Error>),
}
impl From<Box<dyn Error>> for TsaError {
    fn from(value: Box<dyn Error>) -> Self {
        Err(value)
            .or_else(|e| Ok(Self::Tsp(*e.downcast()?)))
            .unwrap_or_else(Self::Other)
    }
}

/// Timestamp authority.
pub struct Tsa {
    /// Base URL
    pub base_url: Cow<'static, str>,
    /// PEM (public key) file
    pub pem: Cow<'static, str>,
}

impl Tsa {
    /// [Free Time Stamp Authority](https://www.freetsa.org)
    pub const FREETSA: Self = Self {
        base_url: Cow::Borrowed("https://www.freetsa.org/tsr"),
        pem: Cow::Borrowed(include_str!("../certs/freetsa/cacert.pem")),
    };
}

/// Signature from a Time Stamp Authority.
///
/// Use [`Tsa::verify()`] to verify the claim.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Signature(#[serde(with = "hex::serde")] Vec<u8>);

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl FromStr for Signature {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        hex::decode(s).map(Self)
    }
}

impl Signature {
    /// Returns the timestamp claimed by the signature.
    pub fn timestamp(&self) -> Result<DateTime<Utc>> {
        Ok(TimeStampResponse::new(self.0.clone()).datetime()?)
    }
}

impl Tsa {
    /// Signs the digest and returns a timestamp response.
    ///
    /// `digest` must be a SHA-224, SHA-256, SHA-384, or SHA-512 hash.
    ///
    /// **This method blocks while waiting on a network request.**
    pub fn timestamp(&self, digest: &[u8]) -> Result<Signature> {
        let response =
            tsp_http_client::request_timestamp_for_digest(&self.base_url, &hex::encode(digest))?;
        Ok(Signature(response.as_der_encoded().to_vec()))
    }

    /// Verifies the digest using `openssl` on the command line and returns
    /// whether it is valid.
    ///
    /// **This method blocks while waiting on an external process.**
    pub fn verify(&self, expected_digest: &[u8], response: &Signature) -> Result<()> {
        let tsr_file_path = NamedTempFile::new()?;
        let pem_file_path = NamedTempFile::new()?;
        std::fs::write(&tsr_file_path, &response.0)?;
        std::fs::write(&pem_file_path, self.pem.as_bytes())?;
        let output = std::process::Command::new("openssl")
            .arg("ts")
            .arg("-verify")
            .arg("-digest")
            .arg(hex::encode(expected_digest))
            .arg("-in")
            .arg(tsr_file_path.path())
            .arg("-CAfile")
            .arg(pem_file_path.path())
            .output()?;
        drop(tsr_file_path);
        drop(pem_file_path);
        if output.status.success() {
            Ok(())
        } else {
            Err(TsaError::OpenSsl {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use sha2::Digest;

    use super::*;

    #[test]
    fn test_freetsa() -> Result<()> {
        test_tsa_with_config(Tsa::FREETSA)
    }

    fn test_tsa_with_config(tsa: Tsa) -> Result<()> {
        let data = "Hello, world!";
        let digest = sha2::Sha256::digest(data.as_bytes());
        let timestamp = Utc::now();
        let signature = tsa.timestamp(&digest)?;

        // Verify (allow up to 5 min of clock deviation)
        tsa.verify(&digest, &signature).unwrap();
        assert!((signature.timestamp()? - timestamp).abs() < TimeDelta::minutes(5));

        // Mismatched digest
        let other_data = "Goodbye, world";
        let other_digest = sha2::Sha256::digest(other_data.as_bytes());
        tsa.verify(&other_digest, &signature).unwrap_err();

        Ok(())
    }
}

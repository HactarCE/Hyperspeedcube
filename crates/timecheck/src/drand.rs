//! Verification of start times using random numbers from [drand], an
//! [Interoperable Randomness
//! Beacon](https://csrc.nist.gov/projects/interoperable-randomness-beacons).
//!
//! [drand]: https://docs.drand.love/docs/overview/
//!
//! # Example
//!
//! ```
//! use chrono::Utc;
//!
//! // Globals shared by client & server
//! let drand = timecheck::drand::Drand::default(); // to verify start times
//!
//! // CLIENT SIDE:
//!
//! // Fetch random seed at start of run
//! let time_immediately_before_starting_run = Utc::now();
//! let round = drand.get_latest_randomness_round()?;
//! let time_immediately_after_starting_run = Utc::now();
//! // Use `round.signature` to subtly influence gameplay
//! // Save `round` in the log file
//!
//! ////////////////////
//!
//! // SERVER SIDE:
//!
//! // Verify start time
//! drand.chain.verify(&round)?;
//! let verified_start_time_range = drand.chain.round_time_range(round.number)?;
//! assert!(
//!     verified_start_time_range.contains(&time_immediately_before_starting_run)
//!         || verified_start_time_range.contains(&time_immediately_after_starting_run)
//! );
//! # Ok::<(), timecheck::drand::DrandError>(())
//! ```

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use drand_verify::{G1Pubkey, G2PubkeyRfc, Pubkey};
use hex_literal::hex;
use serde::{Deserialize, Serialize};

use crate::Verified;

#[allow(missing_docs)]
pub type Result<T, E = DrandError> = std::result::Result<T, E>;

/// Default randomness beacon URL.
///
/// The [drand docs](https://docs.drand.love/developer/) contains a list of
/// beacon URLs.
pub const DEFAULT_DRAND_URL: &str = "https://api.drand.sh";

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum DrandError {
    #[error("timestamp out of range")]
    TimestampOutOfRange,
    #[error("network error: {0}")]
    Ureq(#[from] ureq::Error),
    #[error("drand verification error: {0}")]
    VerificationError(#[from] drand_verify::VerificationError),
    #[error("drand verification failed")]
    VerificationFailed,
}

/// Config values for retrieving and verifying random values from a drand
/// randomness beacon.
///
/// [`Drand::default()`] contains recommended values.
pub struct Drand {
    /// Defaults to [`DEFAULT_DRAND_URL`]
    pub base_url: String,
    /// Defaults to [`Chain::default()`]
    pub chain: Chain,
}

impl Default for Drand {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_DRAND_URL.to_string(),
            chain: Chain::default(),
        }
    }
}

impl Drand {
    /// Fetches a specific randomness round from some time in the past.
    ///
    /// **This method blocks while waiting on a network request.** Also note
    /// that it performs no caching.
    pub fn get_randomness_round(&self, round_number: u64) -> Result<DrandRound> {
        self.get_and_verify_round_from_uri(format!(
            "{base_url}/v2/chains/{chain_hash}/rounds/{round_number}",
            base_url = self.base_url(),
            chain_hash = self.chain.chain_hash,
        ))
    }

    /// Fetches the latest randomness round.
    ///
    /// **This method blocks while waiting on a network request.** Also note
    /// that it performs no caching.
    pub fn get_latest_randomness_round(&self) -> Result<DrandRound> {
        self.get_and_verify_round_from_uri(format!(
            "{base_url}/v2/chains/{chain_hash}/rounds/latest",
            base_url = self.base_url(),
            chain_hash = self.chain.chain_hash,
        ))
    }

    fn base_url(&self) -> &str {
        self.base_url.strip_suffix('/').unwrap_or(&self.base_url)
    }

    fn get_and_verify_round_from_uri(&self, uri: String) -> Result<DrandRound> {
        let round: DrandRound = ureq_get_json(uri)?;
        self.chain.verify(&round).map(|Verified| round)
    }
}

/// Chain of random values emitted by a randomness beacon.
///
/// To see a list of all chains, identified by chain hash:
///
/// ```sh
/// curl -L 'https://api.drand.sh/v2/chains'
/// ```
///
/// To see info for a specific chain:
///
/// ```sh
/// curl -L 'https://api.drand.sh/v2/chains/{chain_hash}/info'
/// ```
#[derive(Debug, Clone)]
pub struct Chain {
    /// Public key for verifying output.
    pub public_key: DrandChainPubKey,
    /// Number of seconds between rounds.
    pub period: u64,
    /// Unix timestamp (UTC) of round 1.
    pub genesis_time: u64,
    /// Hash value used to identify the chain.
    pub chain_hash: Cow<'static, str>,
}

impl Default for Chain {
    /// Returns the `default` chain, which publishes new randomness every 30
    /// seconds.
    fn default() -> Self {
        Self {
            public_key: DrandChainPubKey::g1(hex!(
                "868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31"
            )).unwrap(),
            period: 30,
            genesis_time: 1595431050,
            chain_hash: Cow::Borrowed(
                "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce",
            ),
        }
    }
}

impl Chain {
    /// Returns the `quicknet` chain, which publishes new randomness every 3
    /// seconds.
    pub fn quicknet() -> Self {
        Chain {
            public_key: DrandChainPubKey::g2(hex!(
                "83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a"
            )).unwrap(),
            period: 3,
            genesis_time: 1692803367,
            chain_hash: Cow::Borrowed(
                "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971",
            ),
        }
    }

    /// Returns the timestamp range for which the given round was the latest
    /// round.
    ///
    /// It is effectively impossible for a run with this seed to have been
    /// started before this time range.
    ///
    /// If a run was started after this time range, it should have used a later
    /// round from the randomness beacon.
    ///
    /// **This method blocks while waiting on a network request.**
    ///
    /// Returns `None` in case of an error.
    pub fn round_time_range(&self, round_number: u64) -> Result<Range<DateTime<Utc>>> {
        let round_start_time = round_number
            .saturating_sub(1) // round 1 started at `self.genesis_time`
            .saturating_mul(self.period)
            .saturating_add(self.genesis_time);
        let round_end_time = round_start_time.saturating_add(self.period);
        Ok(datetime_from_timestamp(round_start_time as i64)?
            ..datetime_from_timestamp(round_end_time as i64)?)
    }

    /// Cryptographically verifies the signature of a round.
    pub fn verify(&self, round: &DrandRound) -> Result<Verified, DrandError> {
        let DrandRound {
            number: round,
            signature,
            previous_signature,
        } = round;
        let result = match &self.public_key {
            DrandChainPubKey::G1(g1_pubkey) => {
                g1_pubkey.verify(*round, previous_signature, signature)
            }
            DrandChainPubKey::G2(g2_pubkey_rfc) => {
                g2_pubkey_rfc.verify(*round, previous_signature, signature)
            }
        };
        match result? {
            true => Ok(Verified),
            false => Err(DrandError::VerificationFailed),
        }
    }
}

/// Public key for a randomness chain.
#[derive(Clone)]
pub enum DrandChainPubKey {
    /// See [`G1Pubkey`]
    G1(Arc<G1Pubkey>),
    /// See [`G2PubkeyRfc`]
    G2(Arc<G2PubkeyRfc>),
}

impl fmt::Debug for DrandChainPubKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::G1(_) => f.debug_tuple("G1").finish_non_exhaustive(),
            Self::G2(_) => f.debug_tuple("G2").finish_non_exhaustive(),
        }
    }
}

impl DrandChainPubKey {
    /// See [`G1Pubkey`]
    pub fn g1(data: [u8; 48]) -> Result<Self, drand_verify::InvalidPoint> {
        Ok(Self::G1(Arc::new(G1Pubkey::from_fixed(data)?)))
    }
    /// See [`G2PubkeyRfc`]
    pub fn g2(data: [u8; 96]) -> Result<Self, drand_verify::InvalidPoint> {
        Ok(Self::G2(Arc::new(G2PubkeyRfc::from_fixed(data)?)))
    }
}

/// Randomness data fetched from a randomness beacon. This puts a lower bound on
/// the start of a run.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DrandRound {
    /// Round number.
    #[serde(rename = "round")]
    pub number: u64,
    /// Signature from which the random value is derived.
    ///
    /// Use this to seed randomness in the game.
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
    /// Previous signature (empty if unchained).
    #[serde(default, skip_serializing_if = "Vec::is_empty", with = "hex::serde")]
    pub previous_signature: Vec<u8>,
}

fn ureq_get_json<T: for<'de> serde::Deserialize<'de>>(uri: impl AsRef<str>) -> Result<T> {
    Ok(ureq::get(uri.as_ref())
        .header("Accept", "application/json")
        .call()?
        .into_body()
        .read_json()?)
}

fn datetime_from_timestamp(unix_timestamp: i64) -> Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp_secs(unix_timestamp).ok_or(DrandError::TimestampOutOfRange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drand_default() -> Result<()> {
        test_drand_with_config(Drand::default())
    }

    #[test]
    fn test_drand_quicknet() -> Result<()> {
        test_drand_with_config(Drand {
            chain: Chain::quicknet(),
            ..Default::default()
        })
    }

    fn test_drand_with_config(drand: Drand) -> Result<()> {
        let drand_chain = drand.chain.clone();

        let time_immediately_before_starting_run = Utc::now();
        let seed = drand.get_latest_randomness_round()?;
        let time_immediately_after_starting_run = Utc::now();

        drand_chain.verify(&seed).unwrap();
        let time_range = drand_chain.round_time_range(seed.number)?;
        assert!(
            time_range.contains(&time_immediately_before_starting_run)
                || time_range.contains(&time_immediately_after_starting_run)
        );
        Ok(())
    }
}

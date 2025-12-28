//! Scheme for cheating-resistant deterministic games using randomness beacons
//! and time stamp authorities.
//!
//! This is based on [a scheme for cheating-resistant racing
//! games](https://www.5snb.club/posts/2025/cheating-resistant-racing-games/) by
//! [5225225](https://www.5snb.club/).
//!
//! The idea is to use an [Interoperable Randomness Beacon][irb] to verify when
//! a run begins, and a [Time Stamping Authority][tsa] to verify when a run
//! ends.
//!
//! [irb]: https://csrc.nist.gov/projects/interoperable-randomness-beacons
//! [tsa]:
//!     https://en.wikipedia.org/wiki/Trusted_timestamping#Trusted_(digital)_timestamping
//!
//! Start times and end times are generated and verified indepedently. See
//! [`drand`] for start-time verification and [`tsa`] for end-time verification.
//!
//! # Example
//!
//! This example verifies both start and end times.
//!
//! ```
//! use chrono::Utc;
//! use sha2::Digest;
//!
//! // Globals shared by client & server
//! let drand = timecheck::drand::Drand::default(); // to verify start times
//! let tsa = timecheck::tsa::Tsa::FREETSA; // to verify end times
//! # fn play_game_with_seed(seed: &[u8]) -> String { "Hello, world!".to_string() }
//!
//! // CLIENT SIDE:
//!
//! // Fetch random seed at start of run
//! let round = drand.get_latest_randomness_round()?;
//! // Use `round.signature` to subtly influence gameplay
//! let gameplay_log = play_game_with_seed(&round.signature);
//! // Sign digest at end of run
//! let digest = sha2::Sha256::digest(gameplay_log.as_bytes());
//! let signature = tsa.timestamp(&digest)?;
//! // Save `seed`, `gameplay_log`, and `signature` in the log file
//!
//! ////////////////////
//!
//! // SERVER SIDE:
//!
//! // Verify start time
//! drand.chain.verify(&round)?;
//! let time_range = drand.chain.round_time_range(round.number)?;
//! println!("run started between {:?} and {:?}", time_range.start, time_range.end);
//!
//! // Verify end time
//! tsa.verify(&digest, &signature)?;
//! let end_time = signature.timestamp()?;
//! println!("run ended at {:?}", end_time);
//! # Ok::<(), timecheck::Error>(())
//! ```

#[cfg(feature = "drand")]
pub mod drand;
#[cfg(feature = "tsa")]
pub mod tsa;

#[allow(missing_docs)]
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    #[cfg(feature = "drand")]
    #[error("{0}")]
    Drand(#[from] drand::DrandError),
    #[cfg(feature = "tsa")]
    #[error("{0}")]
    Tsa(#[from] tsa::TsaError),

    #[error("{0}")]
    Other(String),
}

/// Indicates successful verification.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Verified;

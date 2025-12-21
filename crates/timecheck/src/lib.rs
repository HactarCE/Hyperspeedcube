//! Scheme for cheating-resistant deterministic games using randomness beacons
//! and time stamp authorities.
//!
//! This is based on [a scheme for cheating-resistant racing
//! games](https://www.5snb.club/posts/2025/cheating-resistant-racing-games/) by
//! [5225225](https://www.5snb.club/).

use std::ops::Range;

/// Random seed fetched from a randomness beacon. This puts a lower bound on the
/// start of a run.
///
/// # Security
///
/// Before constructing a `RandomSeed`, ensure that the URL is trusted. See
/// <https://docs.drand.love/developer/> for a list of beacon URLs.
///
/// # Example
///
/// ```
/// use std::time::SystemTime;
/// use timecheck::RandomSeed;
///
/// // on game client
/// let url = "https://api.drand.sh/";
/// let time_immediately_before_starting_run =
///     std::time::SystemTime::UNIX_EPOCH.elapsed()?.as_secs();
/// let seed = RandomSeed::get_latest(url)?;
/// let time_immediately_after_starting_run =
///     std::time::SystemTime::UNIX_EPOCH.elapsed()?.as_secs();
///
/// // include `seed` in replay file, then deserialize on the server
///
/// // on verification server
/// let seed2 = RandomSeed::get(url, seed.beacon_round)?;
/// assert_eq!(seed.random_bytes, seed2.random_bytes);
/// let time_range = seed.time_range()?;
/// assert!(
///     time_range.contains(&time_immediately_before_starting_run)
///         || time_range.contains(&time_immediately_after_starting_run)
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RandomSeed {
    /// Random bytes returned by the beacon.
    ///
    /// Use these to seed randomness in the game.
    pub random_bytes: Vec<u8>,
    /// Beacon URL.
    pub beacon_url: String,
    /// Beacon round.
    pub beacon_round: u64,
}
impl RandomSeed {
    /// Fetches a specific random seed from some time in the past.
    ///
    /// **This method blocks while waiting for a network request.** Also note
    /// that it performs no caching.
    pub fn get(beacon_url: &str, beacon_round: u64) -> Result<Self, drand_core::DrandError> {
        let client = drand_core::HttpClient::new(beacon_url, None)?;
        let beacon_output = client.get(beacon_round)?;
        let random_bytes = beacon_output.randomness();

        Ok(RandomSeed {
            random_bytes,
            beacon_url: beacon_url.to_string(),
            beacon_round,
        })
    }

    /// Fetches the latest random seed.
    ///
    /// **This method blocks while waiting for a network request.** Also note
    /// that it performs no caching.
    pub fn get_latest(beacon_url: &str) -> Result<Self, drand_core::DrandError> {
        let client = drand_core::HttpClient::new(beacon_url, None)?;
        let beacon_output = client.latest()?;
        let random_bytes = beacon_output.randomness();
        let beacon_round = beacon_output.round();

        Ok(RandomSeed {
            random_bytes,
            beacon_url: beacon_url.to_string(),
            beacon_round,
        })
    }

    /// Returns the earliest and latest timestamp for which the beacon would
    /// return this result, as seconds since UTC epoch.
    ///
    /// It is impossible a run with this seed to have been started before this
    /// time range, assuming the randomness beacon can be trusted.
    ///
    /// If a run was started after this time range, it should have used a more
    /// up-to-date output from the randomness beacon.
    ///
    /// **This method blocks while waiting for a network request.** Also note
    /// that it performs no caching.
    pub fn time_range(&self) -> Result<Range<u64>, drand_core::DrandError> {
        let client = drand_core::HttpClient::new(&self.beacon_url, None)?;
        let chain_info = client.chain_info()?;
        let period = chain_info.period();
        let genesis_time = chain_info.genesis_time();
        let round_start_time = self
            .beacon_round
            .saturating_sub(1)
            .saturating_mul(period)
            .saturating_add(genesis_time);
        let round_end_time = round_start_time.saturating_add(period);

        Ok(round_start_time..round_end_time)
    }
}

// pub struct SignedHash {

// }

async fn main() {
    // let signature = vec![1, 2, 3, 4].into();
    // let token = sigstore_tsa::TimestampClient::freetsa()
    //     .timestamp_signature(&signature)
    //     .await
    //     .unwrap();
    // sigstore_tsa::verify_timestamp_response(
    //     token.as_bytes(),
    //     &signature.as_bytes(),
    //     sigstore_tsa::VerifyOpts::new(),
    // )
    // .unwrap();
}

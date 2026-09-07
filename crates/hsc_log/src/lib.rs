//! Structures and serialization/deserialization for puzzle twist notation and
//! log files.

use std::str::FromStr;

use hyperkdl::Warning;
use kdl::{KdlDocument, KdlError};

mod proxy;
mod schema;
pub mod verify;

pub use proxy::KdlProxy;
pub use schema::latest::*;

/// Error returned when deserializing a log file fails.
#[expect(missing_docs)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing version number")]
    MissingVersion,
    #[error("unsupported version number")]
    UnsupportedVersion,
    #[error("{0}")]
    Kdl(#[from] KdlError),
}

/// Deserializes a log file from a string, automatically migrating it to the
/// latest version.
pub fn deserialize(s: &str) -> Result<(schema::latest::LogFile, Vec<Warning>), Error> {
    let mut doc = KdlDocument::from_str(s)?;

    let mut warnings = vec![];

    let (version, version_span) =
        remove_kdl_version_number(&mut doc).ok_or(Error::MissingVersion)?;
    if version > schema::latest::LOG_FILE_VERSION {
        warnings.push(Warning {
            span: version_span,
            msg: "This file was saved using a newer version, and might not load correctly"
                .to_owned(),
        });
    }

    let log_file = match version {
        1 => todo!("log file v1"),
        2 => schema::v2::deserialize(doc, &mut warnings)?,
        3 => schema::v3::deserialize(doc, &mut warnings)?,
        _ => return Err(Error::UnsupportedVersion),
    };

    Ok((log_file, warnings))
}

fn remove_kdl_version_number(
    doc: &mut KdlDocument,
) -> Option<(i128, hyperkdl::miette::SourceSpan)> {
    let nodes = doc.nodes_mut();
    let version_node_index = nodes
        .iter_mut()
        .position(|node| node.name().value() == "version")?;
    let version_node = nodes.remove(version_node_index);
    let version_number = version_node.entries().iter().next()?.value().as_integer()?;
    Some((version_number, version_node.span()))
}

#[cfg(test)]
mod tests {
    use hyperpuzzle_core::Timestamp;
    use hyperpuzzle_core::prelude::*;
    use itertools::Itertools;

    use crate::schema::latest::*;

    #[test]
    fn test_puzzle_log_export() {
        let log_file = LogFile {
            program: Some(Program {
                name: Some("Hyperspeedcube".to_string()),
                version: Some("2.0.0-pre.15".to_string()),
            }),
            solves: vec![Solve {
                replay: Some(false),
                puzzle: LogPuzzle {
                    id: "ft_cube:3".to_string(),
                    version: "1.0.0".to_string(),
                },
                solved: true,
                duration: Some(5 * 60 * 1000),
                scramble: Some(Scramble {
                    version: 1,
                    ty: ScrambleType::Partial(3),
                    time: Some(Timestamp::now()),
                    seed: Some("abc".to_string()),
                    twists: "R U L'".to_string(),
                    drand_round_v1: None,
                }),
                log: vec![
                    LogEvent::Scramble {
                        time: Some(Timestamp::now()),
                    },
                    LogEvent::Twists("L U' R'".to_string()),
                    LogEvent::EndSolve {
                        time: Some(Timestamp::now()),
                        duration: Some(3000),
                    },
                    LogEvent::EndSession {
                        time: Some(Timestamp::now()),
                    },
                ],
                tsa_signature_v1: None,
                tsa_signature_v2: None,
                tsa_signature_v3: None,
            }],
        };
        std::thread::sleep(std::time::Duration::from_millis(10)); // force timestamp to change
        let serialized = log_file.serialize();
        println!("{serialized}");
        std::thread::sleep(std::time::Duration::from_millis(10)); // force timestamp to change
        let (deserialized, _warnings) = crate::deserialize(&serialized).unwrap();
        assert_eq!(log_file, deserialized);

        assert_eq!(
            log_file.solves[0].digest_v3_events().collect_vec(),
            log_file.solves[0].log[0..3].iter().collect_vec(),
        );
    }
}

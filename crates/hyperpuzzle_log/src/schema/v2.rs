/// Log file version.
pub const LOG_FILE_VERSION: i128 = 3;

use hyperkdl::Warning;
use itertools::Itertools;
use kdl::*;

use crate::Error;

#[allow(unused)]
use super::v3::{DrandRound, LogEvent, LogFile, LogPuzzle, Program, Scramble, Solve};

/// Deserializes a log file from a string and updates it to the latest schema.
pub fn deserialize(mut doc: KdlDocument, warnings: &mut Vec<Warning>) -> Result<LogFile, Error> {
    if let Some((version, _)) = crate::remove_kdl_version_number(&mut doc)
        && version != LOG_FILE_VERSION
    {
        return Err(Error::UnsupportedVersion);
    }

    let mut log_file = super::v3::deserialize(doc, warnings)?;

    // Migrate twist names
    for solve in &mut log_file.solves {
        let migration_function: Option<MigrationFn> = match puzzle_or_generator_id(&solve.puzzle.id)
        {
            "ft_polygonal_prism" | "ft_triminx_prism" => Some(migrate_prism),
            "ft_polygonal_duoprism"
            | "ft_polygonal_duoprism_3_minx"
            | "ft_polygonal_duoprism_3_minx_3_minx" => Some(migrate_duoprism),
            "ft_4_simplex_a"
            | "ft_4_simplex_b"
            | "ft_4_simplex_c"
            | "ft_4_simplex_d"
            | "ft_4_simplex_pyraminx" => Some(migrate_4_simplex),
            _ => None,
        };
        if let Some(f) = migration_function {
            migrate_twists_in_solve(solve, f);
        }
    }

    Ok(log_file)
}

/// `ft_cube:3` -> `ft_cube`
fn puzzle_or_generator_id(id: &str) -> &str {
    match id.split_once(':') {
        Some((generator_name, _params)) => generator_name,
        None => id,
    }
}

fn migrate_twists_in_solve(solve: &mut Solve, migrate_fn: MigrationFn) {
    if let Some(scramble) = &mut solve.scramble {
        scramble.twists = migrate_fn(&scramble.twists);
    }
    for event in &mut solve.log {
        match event {
            LogEvent::Click { target, .. } => *target = migrate_fn(target),
            LogEvent::Twists(twists_string) => {
                migrate_twists_in_notation_string(twists_string, migrate_fn)
            }
            _ => (),
        }
    }

    // TSA signatures are invalid anyway
    solve.tsa_signature_v1 = None;
    solve.tsa_signature_v2 = None;
    solve.tsa_signature_v3 = None;
}

fn migrate_twists_in_notation_string(twists_string: &mut String, migrate_fn: MigrationFn) {
    if let Ok(mut node_list) =
        hypuz_notation::parse_notation(twists_string, hypuz_notation::Features::MAXIMAL)
    {
        migrate_twists_in_notation_node(&mut node_list, migrate_fn);
        *twists_string = node_list.to_string();
    }
}

fn migrate_twists_in_notation_node(
    node_list: &mut hypuz_notation::NodeList,
    migrate_fn: MigrationFn,
) {
    for node in &mut **node_list {
        match node {
            hypuz_notation::Node::Move(mv) => {
                mv.transform.family = migrate_fn(&mv.transform.family).into()
            }
            hypuz_notation::Node::Rotation(rot) => {
                rot.transform.family = migrate_fn(&rot.transform.family).into()
            }
            hypuz_notation::Node::Group(group) => {
                migrate_twists_in_notation_node(&mut group.contents, migrate_fn)
            }
            hypuz_notation::Node::BinaryGroup(group) => {
                migrate_twists_in_notation_node(&mut group.lhs, migrate_fn);
                migrate_twists_in_notation_node(&mut group.rhs, migrate_fn);
            }
            _ => (),
        }
    }
}

type MigrationFn = fn(&str) -> String;

fn migrate_prism(s: &str) -> String {
    // The prism generator in this version had a limit of 24-gon,
    // so all the names of the sides were exactly 2 letters.
    if let Some(rest) = s.strip_prefix('F') {
        format!("a{rest}")
    } else if let Some(rest) = s.strip_prefix('V') {
        format!("aβ{rest}")
    } else if s == "U" {
        format!("bA")
    } else if s == "D" {
        format!("bB")
    } else {
        s.to_string()
    }
}

fn migrate_duoprism(s: &str) -> String {
    s.replace("ε", "a")
        .replace("η", "b")
        .replace("ω", "aβ")
        .replace("ψ", "bβ")
}

// fn migrate_tetrahedron(s: &str) -> String {
//     match s {
//         // faces
//         "F" => "D",
//         "U" => "F",
//         "R" => "BR",
//         "L" => "BL",
//         // vertexes
//         "l" => "R",
//         "r" => "L",
//         "u" => "B",
//         "f" => "U",
//         _ => s,
//     }
//     .to_string()
// }

fn migrate_4_simplex(s: &str) -> String {
    s.split('_')
        .map(|ax| match ax {
            // faces
            "O" => "E",
            "D" => "D",
            "F" => "C",
            "BR" => "B",
            "BL" => "A",
            // vertexes
            "R" => "βA",
            "L" => "βB",
            "B" => "βC",
            "U" => "βD",
            "I" => "βE",
            _ => ax,
        })
        .join("_")
}

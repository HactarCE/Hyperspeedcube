//! Utility functions for generating

use hyperpuzzle_core::{AnyOrbit, Orbit, PuzzleElement, hypershape::GeneratorId};

/// Returns an HPS expression containing a string literal with the contents of
/// `s`, properly escaped.
pub fn to_str_literal(s: &str) -> String {
    let mut ret = String::with_capacity(s.len() + 2);
    ret.push('"');
    for c in s.chars() {
        match c {
            '\t' => ret.push_str(r#"\t"#),
            '\r' => ret.push_str(r#"\r"#),
            '\n' => ret.push_str(r#"\n"#),
            '$' => ret.push_str(r#"\$"#),
            '"' => ret.push_str(r#"\""#),
            '\\' => ret.push_str(r#"\\"#),
            _ => ret.push(c),
        }
    }
    ret.push('"');
    ret
}

/// Returns an HPS map key with the contents of `s`, properly escaped.
pub fn to_map_key(s: &str) -> std::borrow::Cow<'_, str> {
    match crate::parse::is_valid_ident(s) {
        true => s.into(),
        false => to_str_literal(s).into(),
    }
}

/// Returns the Hyperpuzzlescript source code to generate the given naming
/// and ordering.
pub fn orbit_hps_code(
    orbit: &AnyOrbit,
    new_names_and_order: &[(usize, String)],
    compact: bool,
) -> String {
    match orbit {
        AnyOrbit::Axes(orbit) => generic_orbit_hps_code(orbit, new_names_and_order, compact),
        AnyOrbit::Colors(orbit) => generic_orbit_hps_code(orbit, new_names_and_order, compact),
    }
}

/// Returns the Hyperpuzzlescript source code to generate the given naming
/// and ordering.
fn generic_orbit_hps_code<T: PuzzleElement>(
    orbit: &Orbit<T>,
    new_names_and_order: &[(usize, String)],
    compact: bool,
) -> String {
    let mut new_element_names = vec![None; orbit.elements.len()];
    for (i, new_name) in new_names_and_order {
        if *i < new_element_names.len() {
            new_element_names[*i] = Some(new_name);
        }
    }

    let mut s = "#{\n".to_owned();
    for (i, new_name) in new_names_and_order {
        s += "  ";
        s += &*to_map_key(new_name);
        s += " = [";
        let mut is_first = true;
        let mut elem_index = *i;
        while let Some(gen_seq) = orbit.generator_sequences.get(elem_index) {
            for GeneratorId(g) in &gen_seq.generators.0 {
                if is_first {
                    is_first = false;
                } else {
                    s += ", ";
                }
                s += &format!("{}", g + 1); // 1-indexed
            }
            let Some(next) = gen_seq.end else { break };
            elem_index = next;
            if compact {
                if let Some(Some(other_name)) = new_element_names.get(elem_index) {
                    s += &format!(", {}", to_str_literal(other_name));
                    break;
                }
            }
        }
        s += "],\n";
    }
    s += "}";
    s
}

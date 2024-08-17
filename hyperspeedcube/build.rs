//! Build script that sets the application icon on Windows.

use itertools::Itertools;
use owo_colors::OwoColorize;

fn main() {
    // Rebuild when locale files change
    println!("cargo::rerun-if-changed=locales");
    println!("cargo::rerun-if-changed=src");

    check_translation_strings("locales/en.yaml");

    #[cfg(all(windows, not(debug_assertions)))]
    {
        // Set application icon.
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon/hyperspeedcube.ico");
        res.compile().unwrap();
    }
}

fn check_translation_strings(locale_filename: &str) {
    let mut keys = std::collections::HashSet::new();

    // Find keys defined for English using a janky YAML parser.
    let mut key_stack = vec![];
    let mut in_multiline_string = false;
    for line in std::fs::read_to_string(locale_filename).unwrap().lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        if !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }

        while key_stack.last().is_some_and(|&(i, _)| i >= indent) {
            in_multiline_string = false;
            key_stack.pop();
        }

        if !in_multiline_string {
            key_stack.push((indent, key));
            if !rest.is_empty() {
                keys.insert(key_stack.iter().map(|(_, k)| k).join("."));
            }
        }
        if rest.trim_start().starts_with(&['|', '>']) {
            // multiline string
            in_multiline_string = true;
        }
    }

    let mut unused_keys = keys.clone();

    // Error if any is missing.
    let pat = regex::Regex::new(r#"\bt![\(\[{]\s*"([^"]*)""#).unwrap();
    for file_path in glob::glob("src/**/*.rs").unwrap() {
        let file = file_path.unwrap();
        if let Ok(contents) = std::fs::read_to_string(&file) {
            for captures in pat.captures_iter(&contents) {
                let key = captures.get(1).unwrap();
                unused_keys.remove(key.as_str());

                let index = key.start();
                let line = contents[..index].lines().count();
                let col = contents[..index].lines().last().unwrap_or_default().len();
                let file_line_col = format!("{}:{line}:{col}", file.to_string_lossy());

                if !keys.contains(key.as_str()) {
                    warn(format!(
                        "{}: {} in {} is missing from {}",
                        "missing i18n key".bold(),
                        key.as_str().red().bold(),
                        file_line_col.blue().bold(),
                        locale_filename.cyan().bold(),
                    ));
                }
            }
        }
    }

    for key in unused_keys {
        let last_segment = key.split('.').last().unwrap_or(&key);
        if !last_segment.starts_with('_') {
            warn(format!(
                "{}: {}",
                "unused localization key".bold(),
                key.blue().bold(),
            ));
        }
    }
}

fn warn(s: impl std::fmt::Display) {
    println!("cargo::warning={s}");
}

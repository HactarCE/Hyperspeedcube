//! Build script that sets the application icon on Windows.

use std::collections::HashSet;

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
    let mut keys = HashSet::new();

    // Find keys defined for English using a janky YAML parser.
    let mut key_stack = vec![];
    let mut in_multiline_string = false;
    for line in std::fs::read_to_string(locale_filename).unwrap().lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        {
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
    let pat = regex::Regex::new(r#"\b([tpq])([l]?)![\(\[{]\s*"([^"]*)""#).unwrap();
    for file_path in glob::glob("src/**/*.rs").unwrap() {
        let file = file_path.unwrap();
        if let Ok(contents) = std::fs::read_to_string(&file) {
            let mut last_prefix = None;

            for captures in pat.captures_iter(&contents) {
                let key = captures.get(3).unwrap();

                let get_file_loc = || {
                    let index = key.start();
                    let line = contents[..index].lines().count();
                    let col = contents[..index].lines().last().unwrap_or_default().len();
                    format!("{}:{line}:{col}", file.to_string_lossy())
                };

                let key = match captures.get(1).unwrap().as_str().chars().next().unwrap() {
                    't' => key.as_str().to_owned(),
                    'p' => {
                        last_prefix = Some(format!("{}.", key.as_str()));
                        continue;
                    }
                    'q' => {
                        let Some(prefix) = &last_prefix else {
                            warn_at("`ql!` without `p!`", get_file_loc, "");
                            continue;
                        };
                        format!("{prefix}{}", key.as_str())
                    }
                    other => {
                        warn_at("unknown i18n macro", get_file_loc, &format!("`{other}!`"));
                        continue;
                    }
                };

                match captures.get(2).unwrap().as_str().chars().next() {
                    Some('l') => {
                        unused_keys.remove(key.as_str());
                        unused_keys.remove(&format!("{}.label", key.as_str()));
                        unused_keys.remove(&format!("{}.full", key.as_str()));
                        unused_keys.remove(&format!("{}.desc", key.as_str()));
                        if !keys.contains(key.as_str()) {
                            warn_if_missing_key(
                                &keys,
                                &format!("{}.label", key.as_str()),
                                get_file_loc,
                                locale_filename,
                            );
                        }
                    }
                    None => {
                        unused_keys.remove(key.as_str());
                        warn_if_missing_key(&keys, key.as_str(), get_file_loc, locale_filename);
                    }
                    Some(other) => {
                        warn_at("unknown i18n macro", get_file_loc, &format!("`{other}!`"));
                    }
                }
            }
        }
    }

    for key in unused_keys {
        let last_segment = key.split('.').last().unwrap_or(&key);
        if !last_segment.starts_with('_') {
            warn(&format!(
                "{}: {}",
                "unused localization key".bold(),
                key.blue().bold(),
            ));
        }
    }
}

fn warn_if_missing_key(
    keys: &HashSet<String>,
    key: &str,
    get_file_loc: impl FnOnce() -> String,
    locale_filename: &str,
) {
    if !keys.contains(key) {
        let msg = format!(
            "{} is missing from {}",
            key.red().bold(),
            locale_filename.cyan().bold(),
        );
        warn_at("missing i18n key", get_file_loc, &msg);
    }
}

fn warn_at(msg: &str, get_file_loc: impl FnOnce() -> String, details: &str) {
    warn(&format!(
        "{msg} at {loc}: {details}",
        msg = msg.bold(),
        loc = get_file_loc().blue().bold(),
    ));
}
fn warn(msg: &str) {
    println!("cargo::warning={msg}")
}

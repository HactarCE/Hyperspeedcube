use std::fmt;

use owo_colors::OwoColorize;

#[derive(Debug, Default, Clone)]
pub struct SourceInfo {
    pub filename: String,
    pub contents: String,
}
impl SourceInfo {
    pub fn at(&self, offset: usize) -> String {
        let Self { filename, contents } = self;
        let line = contents[..offset].lines().count();
        let col = contents[..offset].lines().last().unwrap_or_default().len();
        format!("{filename}:{line}:{col}")
    }
}

pub fn warn_with(msg: &str, loc: String, details: impl fmt::Display) {
    warn(&format!(
        "{msg} at {loc}: {details}",
        msg = msg.bold(),
        loc = loc.blue().bold(),
    ));
}

pub fn warn_at(msg: &str, loc: String) {
    warn(&format!(
        "{msg} at {loc}",
        msg = msg.bold(),
        loc = loc.blue().bold(),
    ));
}

pub fn warn(msg: &str) {
    println!("cargo::warning={msg}");
}

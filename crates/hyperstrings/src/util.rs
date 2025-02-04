use std::path::Path;

use kdl::{KdlDocument, KdlEntry, KdlNode};
use owo_colors::OwoColorize;

use crate::warn::*;

pub fn read_kdl_file(path: impl AsRef<Path>) -> (SourceInfo, KdlDocument) {
    let src = SourceInfo {
        filename: path.as_ref().to_string_lossy().into_owned(),
        contents: std::fs::read_to_string(path).unwrap(),
    };
    match src.contents.parse() {
        Ok(kdl) => (src, kdl),
        Err(e) => {
            for diagnostic in e.diagnostics {
                warn_at(&diagnostic.to_string(), src.at(diagnostic.span.offset()));
            }
            std::process::exit(1);
            // panic!("bad KDL file");
        }
    }
}

pub fn ignore_node(src: &SourceInfo, node: &KdlNode) {
    warn_at("ignoring node", src.at(node.span().offset()));
}

#[must_use]
pub fn take_entry<'a>(
    src: &SourceInfo,
    node: &KdlNode,
    entries: &mut impl Iterator<Item = &'a KdlEntry>,
    error_msg: &str,
) -> Option<&'a KdlEntry> {
    warn_if_none(entries.next(), error_msg, || src.at(end_of_entries(node)))
}

#[must_use]
pub fn take_entry_str_value<'a>(src: &SourceInfo, entry: &'a KdlEntry) -> Option<&'a str> {
    warn_if_none(entry.value().as_string(), "expected string value", || {
        src.at(entry.span().offset())
    })
}

#[must_use]
pub fn take_entry_string_value(src: &SourceInfo, entry: &KdlEntry) -> Option<String> {
    take_entry_str_value(src, entry).map(str::to_owned)
}

#[must_use]
pub fn take_entry_bool_value(src: &SourceInfo, entry: &KdlEntry) -> Option<bool> {
    warn_if_none(entry.value().as_bool(), "expected bool value", || {
        src.at(entry.span().offset())
    })
}

pub fn ignore_entry_name(src: &SourceInfo, entry: &KdlEntry) {
    if let Some(name) = entry.name() {
        warn_at("ignoring key", src.at(name.span().offset()));
    }
}

pub fn ignore_entry_type(src: &SourceInfo, entry: &KdlEntry) {
    if let Some(ty) = entry.ty() {
        warn_at("ignoring type annotation", src.at(ty.span().offset()));
    }
}

pub fn warn_unknown_entry_name(src: &SourceInfo, entry: &KdlEntry) {
    match entry.name() {
        Some(key) => warn_with("unknown key", src.at(key.span().offset()), key.red().bold()),
        None => warn_at("unknown key", src.at(entry.span().offset())),
    }
}

pub fn warn_if_overwriting<T>(
    opt: &mut Option<T>,
    value: T,
    msg: &str,
    loc: impl FnOnce() -> String,
) {
    if opt.is_none() {
        *opt = Some(value);
    } else {
        warn_at(msg, loc());
    }
}

#[must_use]
pub fn take_children<'a>(src: &SourceInfo, node: &'a KdlNode) -> Option<&'a KdlDocument> {
    warn_if_none(node.children(), "expected child nodes", || {
        src.at(end_of_entries(node))
    })
}

pub fn ignore_entries<'a>(src: &SourceInfo, entries: impl IntoIterator<Item = &'a KdlEntry>) {
    for entry in entries {
        ignore_entry(src, entry);
    }
}

pub fn ignore_entry(src: &SourceInfo, entry: &KdlEntry) {
    warn_with("ignoring entry", src.at(entry.span().offset()), entry);
}

pub fn ignore_children(src: &SourceInfo, node: &KdlNode) {
    if let Some(children) = node.children() {
        warn_at("ignoring children", src.at(children.span().offset()));
    }
}

fn end_of_entries(node: &KdlNode) -> usize {
    let span = match node.entries().last() {
        Some(e) => e.span(),
        None => node.name().span(),
    };
    span.offset() + span.len()
}

#[must_use]
pub fn warn_if_none<T>(value: Option<T>, msg: &str, loc: impl FnOnce() -> String) -> Option<T> {
    if value.is_none() {
        warn_at(msg, loc());
    }
    value
}

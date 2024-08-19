use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use kdl::KdlNode;

use crate::schema::*;
use crate::util;
use crate::warn::*;

#[derive(Debug)]
pub struct Lang {
    pub name: String,
    pub values: HashMap<String, Option<String>>,
}
impl Lang {
    pub fn display_module(&self, f: &mut fmt::Formatter<'_>, schema: &Schema) -> fmt::Result {
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(non_upper_case_globals)]")?;
        writeln!(f, "pub mod {} {{", self.name)?;
        writeln!(f, "    use super::structs::*;")?;
        writeln!(f, "")?;
        self.display_consts(f, schema, "", LANG_STRUCT_TYPE)?;
        writeln!(f, "}}")?;
        Ok(())
    }

    fn display_consts(
        &self,
        f: &mut fmt::Formatter<'_>,
        schema: &Schema,
        path_prefix: &str,
        struct_name: &str,
    ) -> fmt::Result {
        let Some(struct_schema) = schema.structs.get(struct_name) else {
            warn(&format!("missing struct schema for `{struct_name}`"));
            return Ok(());
        };
        let const_name = path_to_const(path_prefix);
        let vis = if const_name == LANG_STRUCT_NAME {
            "pub "
        } else {
            ""
        };
        writeln!(
            f,
            "    {vis}const {const_name}: {struct_name} = {struct_name} {{",
        )?;
        let mut any_missing = false;
        let mut stack = vec![];
        for field in &struct_schema.fields {
            let field_name = &field.name;
            let field_value = match &field.ty.0 {
                Some(new_struct_name) => {
                    let new_path = format!("{path_prefix}{field_name}.");
                    stack.push((new_path.clone(), new_struct_name.clone()));
                    path_to_const(&new_path)
                }
                None => {
                    let Some(Some(v)) = self.values.get(&format!("{path_prefix}{field_name}"))
                    else {
                        any_missing = true;
                        continue;
                    };
                    format!("{v:?}")
                }
            };
            writeln!(f, "        {field_name}: {field_value},")?;
        }
        if any_missing {
            write!(f, "        ..")?;
            match struct_schema.try_default_value(struct_name) {
                Some(default) => write!(f, "{default}")?,
                None => {
                    if let Some(l) = schema.fallback_lang.as_ref().filter(|&l| *l != self.name) {
                        write!(f, "super::{l}::{const_name}")?;
                    } else {
                        warn(&format!("no default value for `{path_prefix}`"));
                        write!(f, "todo!()")?;
                    }
                }
            };
            writeln!(f)?;
        }
        writeln!(f, "    }};")?;

        for (new_path, new_struct_name) in stack {
            self.display_consts(f, schema, &new_path, &new_struct_name)?;
        }

        Ok(())
    }
}

pub fn parse_lang(path: impl AsRef<Path>, schema: &mut Schema) -> Lang {
    let (src, doc) = crate::read_kdl_file(&path);

    let name = path
        .as_ref()
        .file_stem()
        .expect("bad file path")
        .to_string_lossy()
        .into_owned();

    let mut lang = Lang {
        name,
        values: HashMap::new(),
    };
    for node in doc.nodes() {
        parse_lang_node(&src, schema, &mut lang, node, "", LANG_STRUCT_TYPE);
    }
    lang
}

fn parse_lang_node(
    src: &SourceInfo,
    schema: &mut Schema,
    lang: &mut Lang,
    node: &KdlNode,
    path_prefix: &str,
    struct_name: &str,
) {
    let field_name = node.name().value();
    let path = format!("{path_prefix}{field_name}");
    let field_ty;
    if let Some(ty) = node.ty() {
        let new_struct_name = ty.value();
        let new_prefix = path + ".";

        let Some(struct_schema) = schema.structs.get(new_struct_name) else {
            warn_with(
                "unknown struct",
                src.at(ty.span().offset()),
                new_struct_name,
            );
            return;
        };

        let mut entries = node.entries().iter();
        for field in &struct_schema.fields {
            if field.may_be_inline {
                let field_path = format!("{new_prefix}{}", field.name);
                parse_lang_inline_field(src, lang, &mut entries, field_path);
            }
        }
        util::ignore_entries(src, entries);

        if let Some(children) = node.children() {
            for child in children.nodes() {
                parse_lang_node(src, schema, lang, child, &new_prefix, new_struct_name);
            }
        }

        field_ty = Type(Some(new_struct_name.to_owned()));
    } else if let Some(children) = node.children() {
        let new_struct_name = schema.key_to_struct_name(&path);
        let new_prefix = path + ".";

        util::ignore_entries(src, node.entries());

        for child in children.nodes() {
            parse_lang_node(src, schema, lang, child, &new_prefix, &new_struct_name);
        }

        field_ty = Type(Some(new_struct_name));
    } else {
        let mut entries = node.entries().iter();
        let Some(entry) = util::take_entry(&src, node, &mut entries, "expected string value")
        else {
            return;
        };
        insert_lang_value(
            lang,
            path,
            util::take_entry_string_value(&src, entry),
            || src.at(entry.span().offset()),
        );
        util::ignore_entries(src, entries);

        field_ty = Type(None);
    }

    schema.ensure_field(struct_name, field_name.to_owned(), field_ty);
}

fn parse_lang_inline_field<'a>(
    src: &SourceInfo,
    lang: &mut Lang,
    entries: &mut impl Iterator<Item = &'a kdl::KdlEntry>,
    path: String,
) {
    if let Some(entry) = entries.next() {
        util::ignore_entry_type(src, entry);
        util::ignore_entry_name(src, entry);
        insert_lang_value(
            lang,
            path,
            util::take_entry_string_value(src, entry),
            || src.at(entry.span().offset()),
        );
    }
}

fn insert_lang_value(
    lang: &mut Lang,
    path: String,
    value: Option<String>,
    loc: impl FnOnce() -> String,
) {
    if let Some(v) = value {
        util::warn_if_overwriting(
            lang.values.entry(path).or_insert(None),
            unindent::unindent(&v),
            "duplicate field",
            loc,
        );
    }
}

pub fn path_to_const(path: &str) -> String {
    if path.is_empty() {
        LANG_STRUCT_NAME.to_owned()
    } else {
        path.trim_end_matches('.')
            .replace('.', "___")
            .to_uppercase()
    }
}

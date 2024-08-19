use std::collections::HashMap;
use std::fmt;

use indexmap::IndexMap;
use itertools::Itertools;
use kdl::KdlNode;

use crate::util;
use crate::warn::*;

pub const LANG_STRUCT_NAME: &str = "LANG";
pub const LANG_STRUCT_TYPE: &str = "Lang";

#[derive(Debug)]
pub struct Schema {
    pub traits: IndexMap<String, TraitSchema>,
    pub structs: IndexMap<String, StructSchema>,
    pub fallback_lang: Option<String>,

    path_to_private_struct: HashMap<String, String>,
}
impl Default for Schema {
    fn default() -> Self {
        let mut ret = Self {
            traits: Default::default(),
            structs: Default::default(),
            fallback_lang: None,

            path_to_private_struct: Default::default(),
        };
        ret.key_to_struct_name("");
        ret
    }
}
impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Re-export public structs and traits.
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(unused_imports)]")?;
        writeln!(
            f,
            "pub use structs::{{{}}};",
            self.structs
                .iter()
                .filter(|(_, struct_schema)| struct_schema.is_public)
                .map(|(name, _)| name)
                .sorted()
                .join(", ")
        )?;
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "pub use traits::*;")?;
        writeln!(f)?;

        // Define traits.
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "pub mod traits {{")?;
        for (trait_name, trait_schema) in &self.traits {
            trait_schema.display("    ", f, trait_name)?;
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        writeln!(f)?;

        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "mod structs {{")?;

        // Defnie structs.
        let mut is_first = true;
        for (struct_name, struct_schema) in &self.structs {
            if is_first {
                is_first = false;
            } else {
                writeln!(f)?;
            }

            struct_schema.display("    ", f, struct_name)?;

            // Implement traits.
            for (trait_name, trait_schema) in &self.traits {
                if trait_schema.fields.iter().all(|trait_field| {
                    struct_schema
                        .get(&trait_field.name)
                        .is_some_and(|struct_field| struct_field.ty == trait_field.ty)
                }) {
                    writeln!(f, "    impl super::{trait_name} for {struct_name} {{")?;
                    for field in &trait_schema.fields {
                        let TraitFieldSchema { name, ty } = field;
                        // Take a reference if the type isn't just `&str`.
                        let ref_ = if ty.0.is_some() { "&" } else { "" };
                        write!(f, "        fn {name}(&self) -> {ref_}{ty} ")?;
                        write!(f, "{{ {ref_}self.{name} }}")?;
                    }
                    writeln!(f, "    }}")?;
                }
            }
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}
impl Schema {
    pub fn ensure_field(&mut self, struct_name: &str, field_name: String, field_ty: Type) {
        let fields = &mut self.structs[struct_name].fields;
        match fields.iter().find(|f| f.name == field_name) {
            Some(f) => {
                if f.ty != field_ty {
                    warn(&format!("conflicting type for {struct_name}.{field_name}"));
                }
            }
            None => fields.push(StructFieldSchema {
                name: field_name,
                ty: field_ty,
                default_value: None,
                may_be_inline: false,
            }),
        }
    }

    pub fn key_to_struct_name(&mut self, key: &str) -> String {
        self.path_to_private_struct
            .entry(key.to_owned())
            .or_insert_with(|| {
                let mut desired_name = key
                    .trim_end_matches('.')
                    .split('.')
                    .map(|s| ident_case::RenameRule::PascalCase.apply_to_field(s))
                    .join("_");
                if desired_name.is_empty() {
                    desired_name = LANG_STRUCT_TYPE.to_owned();
                }

                // Append a number to the end to if necessary to ensure uniqueness.
                let mut candidate = desired_name.clone();
                let mut n = 0;
                while self.structs.contains_key(&candidate) {
                    n += 1;
                    candidate = format!("{desired_name}_{n}");
                }

                self.structs.insert(
                    candidate.clone(),
                    StructSchema {
                        is_public: key.is_empty(),
                        default: None,
                        fields: vec![],
                    },
                );

                candidate
            })
            .clone()
    }
}

#[derive(Debug, Default)]
pub struct TraitSchema {
    pub fields: Vec<TraitFieldSchema>,
}
impl TraitSchema {
    fn display(&self, indent: &str, f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
        writeln!(f, "{indent}pub trait {name} {{")?;
        for field in &self.fields {
            let TraitFieldSchema { name, ty } = field;
            writeln!(f, "{indent}    fn {name}(&self) -> {ty};")?;
        }
        writeln!(f, "{indent}}}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TraitFieldSchema {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Default)]
pub struct StructSchema {
    pub is_public: bool,
    pub default: Option<String>,
    pub fields: Vec<StructFieldSchema>,
}
impl StructSchema {
    pub fn get(&self, name: &str) -> Option<&StructFieldSchema> {
        self.fields.iter().find(|f| f.name == name)
    }

    fn display(&self, indent: &str, f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
        // Write struct definition.
        writeln!(f, "{indent}pub struct {name} {{")?;
        for field in &self.fields {
            let StructFieldSchema { name, ty, .. } = field;
            writeln!(f, "{indent}    pub {name}: {ty},")?;
        }
        writeln!(f, "{indent}}}")?;

        // Write default value, if applicable.
        if let Some(default_field_values) = self
            .fields
            .iter()
            .map(|field| Some((&field.name, field.default_value.as_ref()?)))
            .collect::<Option<Vec<(&String, &String)>>>()
        {
            writeln!(f, "{indent}impl {name} {{")?;
            writeln!(f, "{indent}    pub const DEFAULT: Self = Self {{")?;
            for (field_name, default_value) in default_field_values {
                writeln!(f, "{indent}        {field_name}: {default_value:?},")?;
            }
            writeln!(f, "{indent}    }};")?;
            writeln!(f, "{indent}}}")?;
        } else if self.fields.iter().any(|f| f.default_value.is_some()) {
            warn(&format!(
                "some fields of `{name}` are missing a default value"
            ));
        }

        Ok(())
    }
}
impl StructSchema {
    pub fn try_default_value(&self, name: &str) -> Option<String> {
        if let Some(default_path) = &self.default {
            Some(crate::lang::path_to_const(&default_path))
        } else if self.fields.iter().all(|f| f.default_value.is_some()) {
            Some(format!("{name}::DEFAULT"))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct StructFieldSchema {
    pub name: String,
    pub ty: Type,
    pub default_value: Option<String>,
    pub may_be_inline: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Type(pub Option<String>);
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(s) => write!(f, "{s}"),
            None => write!(f, "&'static str"),
        }
    }
}
impl From<&KdlNode> for Type {
    fn from(value: &KdlNode) -> Self {
        Self(value.ty().map(|t| t.value().to_owned()))
    }
}

pub fn parse_trait_schema(src: &SourceInfo, node: &KdlNode) -> Option<(String, TraitSchema)> {
    let mut entries = node.entries().iter();

    let name = {
        let entry = util::take_entry(src, node, &mut entries, "expected trait name")?;
        util::ignore_entry_name(src, entry);
        util::ignore_entry_type(src, entry);
        util::take_entry_str_value(src, entry)?
    };

    util::ignore_entries(src, entries);

    let mut fields = vec![];
    for child in util::take_children(src, node)?.nodes() {
        fields.extend(parse_trait_field_schema(src, child))
    }

    Some((name.to_owned(), TraitSchema { fields }))
}

pub fn parse_struct_schema(src: &SourceInfo, node: &KdlNode) -> Option<(String, StructSchema)> {
    let mut entries = node.entries().iter().peekable();

    let name = {
        let entry = util::take_entry(src, node, &mut entries, "expected struct name")?;
        util::ignore_entry_name(src, entry);
        util::ignore_entry_type(src, entry);
        util::take_entry_str_value(src, entry)?
    };

    let mut default = None;
    for entry in entries {
        if let Some(name) = entry.name() {
            util::ignore_entry_type(src, entry);
            match name.value() {
                "default" => {
                    util::warn_if_overwriting(
                        &mut default,
                        util::take_entry_string_value(src, entry)?,
                        "duplicate key 'default'",
                        || src.at(entry.span().offset()),
                    );
                }
                _ => util::warn_unknown_entry_name(src, entry),
            }
        } else {
            util::ignore_entry(src, entry);
        }
    }

    let mut fields = vec![];
    for child in util::take_children(src, node)?.nodes() {
        fields.extend(parse_struct_field_schema(src, child));
    }

    Some((
        name.to_owned(),
        StructSchema {
            is_public: true,
            default,
            fields,
        },
    ))
}

fn parse_trait_field_schema(src: &SourceInfo, node: &KdlNode) -> Option<TraitFieldSchema> {
    let ret = TraitFieldSchema {
        name: node.name().value().to_owned(),
        ty: Type::from(node),
    };
    util::ignore_entries(src, node.entries());
    util::ignore_children(src, node);
    Some(ret)
}

fn parse_struct_field_schema(src: &SourceInfo, node: &KdlNode) -> Option<StructFieldSchema> {
    let mut entries = node.entries().iter();
    let mut ret = StructFieldSchema {
        name: node.name().value().to_owned(),
        ty: Type::from(node),
        default_value: entries.next().and_then(|entry| {
            util::ignore_entry_type(src, entry);
            util::ignore_entry_name(src, entry);
            util::take_entry_string_value(src, entry)
        }),
        may_be_inline: false,
    };

    if let Some(entry) = entries.next() {
        if entry.name().is_some_and(|s| s.value() == "inline") {
            util::ignore_entry_type(src, entry);
            if let Some(v) = util::take_entry_bool_value(src, entry) {
                ret.may_be_inline = v;
            }
        } else {
            util::ignore_entry(src, entry);
        }
    }

    util::ignore_entries(src, entries);
    util::ignore_children(src, node);
    Some(ret)
}

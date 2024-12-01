use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use indexmap::IndexMap;
use itertools::Itertools;
use kdl::{KdlDocument, KdlIdentifier, KdlNode};
use owo_colors::OwoColorize;

use crate::util;
use crate::warn::*;

pub const LANG_CONST_NAME: &str = "LANG";
pub const LANG_STRUCT_NAME: &str = "Lang";

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct KeyPath(pub String);
impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl AsRef<str> for KeyPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl KeyPath {
    pub const ROOT: Self = Self(String::new());

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn split(&self) -> Option<(Self, &str)> {
        (!self.is_root()).then(|| match self.0.rsplit_once('.') {
            Some((parent, field)) => (Self(parent.to_owned()), field),
            None => (Self::ROOT, self.0.as_str()),
        })
    }
    #[must_use]
    pub fn push(&self, field_name: &str) -> Self {
        let mut ret = self.clone();
        if !self.is_root() {
            ret.0.push('.');
        }
        ret.0 += field_name;
        ret
    }

    pub fn const_name(&self) -> String {
        if self.is_root() {
            LANG_CONST_NAME.to_owned()
        } else {
            self.0.replace('.', "___").to_uppercase()
        }
    }
}

/// Append a number to the end if necessary to ensure uniqueness.
fn generate_name_candidates(desired_name: String) -> impl Iterator<Item = String> {
    let mut next_candidate = desired_name.clone();
    (0..).map(move |i| std::mem::replace(&mut next_candidate, format!("{desired_name}_{i}")))
}

#[derive(Debug, Default)]
pub struct Schema {
    pub traits: IndexMap<String, TraitSchema>,
    pub template_traits: IndexMap<String, Vec<TemplateParameter>>,

    pub structs: IndexMap<String, StructSchema>,
    pub path_to_type: HashMap<KeyPath, Type>,
    path_to_struct_name: HashMap<KeyPath, String>,

    pub template_params: HashMap<KeyPath, HashSet<TemplateParameter>>,

    pub fallback_lang: Option<String>,
}
impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Re-export public structs and traits.
        writeln!(f, "#[rustfmt::skip]")?;
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
        writeln!(f, "pub use traits::*;")?;
        writeln!(f)?;

        // Define traits.
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "pub mod traits {{")?;
        writeln!(f, "    use std::fmt::Debug;")?;
        for (trait_name, trait_schema) in &self.traits {
            writeln!(f)?;
            trait_schema.display("    ", f, trait_name)?;
        }
        for (trait_name, params) in &self.template_traits {
            writeln!(f)?;
            display_template_trait("    ", f, trait_name, params)?;
        }
        writeln!(f, "}}")?;
        writeln!(f)?;

        // Define structs.
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "mod structs {{")?;
        writeln!(f, "    use super::traits::*;")?;
        with_blank_lines_between(f, &self.structs, |f, (struct_name, struct_schema)| {
            struct_schema.display("    ", f, struct_name)?;

            // Implement traits.
            for (trait_name, trait_schema) in &self.traits {
                if struct_schema.fits_trait(trait_schema) {
                    trait_schema.display_impl("    ", f, trait_name, struct_name)?;
                }
            }

            Ok(())
        })?;
        writeln!(f, "}}")?;

        Ok(())
    }
}
impl Schema {
    pub fn init_struct_at_path(&mut self, path: &KeyPath, schema: StructSchema) -> String {
        let struct_name = self.path_to_struct_name(path);
        self.structs.insert(struct_name.clone(), schema);
        self.init_path(path, StructFieldType::Struct(struct_name.clone()));
        struct_name
    }

    pub fn path_to_struct_name(&mut self, path: &KeyPath) -> String {
        self.path_to_struct_name
            .entry(path.clone())
            .or_insert_with(|| {
                let struct_name = if path.is_root() {
                    LANG_STRUCT_NAME.to_owned()
                } else {
                    generate_name_candidates(snake_case_segments_to_struct_name(
                        path.as_ref().split('.'),
                    ))
                    .find(|s| !self.structs.contains_key(s) && s != LANG_STRUCT_NAME)
                    .unwrap()
                };
                struct_name
            })
            .clone()
    }

    pub fn init_path(&mut self, path: &KeyPath, ty: StructFieldType) {
        let ty = match ty {
            StructFieldType::StaticStr => Type::StaticStr,
            StructFieldType::DynTemplate { .. } => Type::Struct(self.path_to_struct_name(path)),
            StructFieldType::Struct(struct_name) => Type::Struct(struct_name),
        };
        match self.path_to_type.entry(path.clone()) {
            std::collections::hash_map::Entry::Occupied(e) => {
                if *e.get() != ty {
                    warn(&format!(
                        "conflicting types for `{}`: `{}` and `{}`",
                        path.red().bold(),
                        e.get().bold(),
                        ty.bold(),
                    ));
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(ty);
            }
        }
    }

    pub fn init_from_config_file(file_path: impl AsRef<Path>) -> Schema {
        let (src, doc) = util::read_kdl_file(file_path);

        let mut schema = Schema::default();

        for node in doc.nodes() {
            let node_loc = || src.at(node.span().offset());

            let mut entries = node.entries().iter();
            match node.name().value() {
                "fallback" => {
                    // IIFE to mimic try_block
                    let fallback_lang = util::take_entry(
                        &src,
                        node,
                        &mut entries,
                        "expected fallback lang specifier",
                    )
                    .and_then(|entry| util::take_entry_string_value(&src, entry));
                    if let Some(s) = fallback_lang {
                        util::warn_if_overwriting(
                            &mut schema.fallback_lang,
                            s,
                            "duplicate `fallback` specification",
                            || src.at(node.span().offset()),
                        );
                    }

                    util::ignore_entries(&src, entries);
                    util::ignore_children(&src, node);
                }

                "trait" => schema.parse_trait_schema(&src, node),

                "struct" => schema.parse_struct_schema(&src, node),

                k => warn_with("unknown node", node_loc(), k.red()),
            }
        }

        schema
    }

    fn parse_trait_schema(&mut self, src: &SourceInfo, node: &KdlNode) {
        let mut entries = node.entries().iter();

        // IIFE to mimic try_block
        let Some(name) = (|| {
            let entry = util::take_entry(src, node, &mut entries, "expected trait name")?;
            util::ignore_entry_name(src, entry);
            util::ignore_entry_type(src, entry);
            util::take_entry_str_value(src, entry)
        })() else {
            return;
        };

        util::ignore_entries(src, entries);

        let mut fields = IndexMap::new();
        if let Some(children) = util::take_children(src, node) {
            for child in children.nodes() {
                fields.extend(
                    self.parse_field_schema(src, child, true)
                        .map(|(name, field)| (name, field.ty)),
                );
            }
        }

        self.traits.insert(name.to_owned(), TraitSchema { fields });
    }

    fn parse_struct_schema(&mut self, src: &SourceInfo, node: &KdlNode) {
        let mut entries = node.entries().iter().peekable();

        // IIFE to mimic try_block
        let Some(name) = (|| {
            let entry = util::take_entry(src, node, &mut entries, "expected struct name")?;
            util::ignore_entry_name(src, entry);
            util::ignore_entry_type(src, entry);
            util::take_entry_str_value(src, entry)
        })() else {
            return;
        };

        let mut fallback_key = None;
        for entry in entries {
            if let Some(name) = entry.name() {
                util::ignore_entry_type(src, entry);
                match name.value() {
                    "default" => {
                        let Some(s) = util::take_entry_string_value(src, entry) else {
                            continue;
                        };
                        util::warn_if_overwriting(
                            &mut fallback_key,
                            s,
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

        let mut fields = IndexMap::new();
        if let Some(children) = util::take_children(src, node) {
            for child in children.nodes() {
                fields.extend(self.parse_field_schema(src, child, false));
            }
        }

        self.structs.insert(
            name.to_owned(),
            StructSchema {
                is_public: true,
                has_global_default: fallback_key.is_none(),
                fallback_key,
                fields,
            },
        );
    }

    fn parse_field_schema(
        &mut self,
        src: &SourceInfo,
        node: &KdlNode,
        is_trait_definition: bool,
    ) -> Option<(String, StructField)> {
        let field_name = node.name().value().to_owned();

        let mut may_be_inline = None;
        let mut template_parameters = vec![];
        for entry in node.entries() {
            if !is_trait_definition && entry.name().is_some_and(|s| s.value() == "inline") {
                util::ignore_entry_type(src, entry);
                let Some(v) = util::take_entry_bool_value(src, entry) else {
                    continue;
                };
                util::warn_if_overwriting(&mut may_be_inline, v, "duplicate key `inline`", || {
                    src.at(entry.span().offset())
                });
            } else if let Some(entry_ty) = entry.ty() {
                util::ignore_entry_name(src, entry);
                // IIFE to mimic try_block
                template_parameters.extend((|| {
                    let name = util::take_entry_string_value(src, entry)?;
                    let ty = TemplateParameterType::from_kdl(src, entry_ty)?;
                    Some(TemplateParameter { name, ty })
                })());
            } else {
                warn_at("expected type annotation", src.at(entry.span().offset()));
            }
        }

        let ty = if let Some(node_ty) = node.ty() {
            StructFieldType::Struct(node_ty.value().to_owned())
        } else if template_parameters.is_empty() {
            StructFieldType::StaticStr
        } else {
            StructFieldType::DynTemplate {
                template_trait_name: self.template_trait_name(template_parameters),
            }
        };
        let may_be_inline = may_be_inline.unwrap_or(false);

        util::ignore_children(src, node);

        Some((field_name, StructField { ty, may_be_inline }))
    }

    pub fn infer_from_lang_file(&mut self, file_path: impl AsRef<Path>) {
        let (src, doc) = util::read_kdl_file(file_path);
        self.infer_struct_from_node_children(&src, &KeyPath::ROOT, &doc);
    }

    fn infer_struct_from_node_children(
        &mut self,
        src: &SourceInfo,
        path: &KeyPath,
        doc: &KdlDocument,
    ) -> StructFieldType {
        let fields = self.infer_struct_fields_from_node_children(src, path, doc);
        StructFieldType::Struct(self.init_struct_at_path(
            path,
            StructSchema {
                is_public: false,
                has_global_default: false,
                fallback_key: None,
                fields,
            },
        ))
    }

    fn infer_struct_fields_from_node_children(
        &mut self,
        src: &SourceInfo,
        path: &KeyPath,
        doc: &KdlDocument,
    ) -> IndexMap<String, StructField> {
        let mut fields = IndexMap::new();
        for node in doc.nodes() {
            let field_name = node.name().value();
            let field_path = path.push(field_name);
            let ty = if let Some(node_ty) = node.ty() {
                let Some(ty) = self.type_from_node_type(src, node_ty) else {
                    continue;
                };
                if let Some(children) = node.children() {
                    let inferred_fields =
                        self.infer_struct_fields_from_node_children(src, &field_path, children);
                    let struct_schema = self.structs.get(node_ty.value()).unwrap();
                    for (field_name, field_data) in inferred_fields {
                        let Some(expected_field) = struct_schema.fields.get(&field_name) else {
                            warn_with(
                                "unknown field",
                                src.at(children.span().offset()),
                                &field_name,
                            );
                            continue;
                        };
                        let expected = &expected_field.ty;
                        let got = &field_data.ty;
                        if matches!(expected, StructFieldType::DynTemplate { .. })
                            && matches!(got, StructFieldType::StaticStr)
                        {
                            // Allow this particular mismatch
                        } else if expected != got {
                            warn_with(
                                &format!("wrong type for field `{field_name}`"),
                                src.at(children.span().offset()),
                                &format!("expected {expected}; got {got}"),
                            );
                        }
                    }
                }
                ty
            } else if let Some(children) = node.children() {
                self.infer_struct_from_node_children(src, &field_path, children)
            } else {
                StructFieldType::StaticStr
            };
            self.init_path(&field_path, ty.clone());
            let may_be_inline = false;
            fields.insert(field_name.to_owned(), StructField { ty, may_be_inline });
        }
        fields
    }

    fn type_from_node_type(
        &self,
        src: &SourceInfo,
        node_ty: &KdlIdentifier,
    ) -> Option<StructFieldType> {
        let struct_name = node_ty.value();
        if self.structs.contains_key(struct_name) {
            Some(StructFieldType::Struct(struct_name.to_owned()))
        } else {
            warn_with(
                "unknown type",
                src.at(node_ty.span().offset()),
                struct_name.red().bold(),
            );
            None
        }
    }

    pub fn infer_template_param(&mut self, path: &KeyPath, param: &TemplateParameter) {
        // Ignore templates we already know about.
        if self.path_to_type.get(path) == Some(&Type::StaticStr) {
            self.template_params
                .entry(path.clone())
                .or_default()
                .insert(param.clone());
        }
    }

    pub fn finalize(&mut self) {
        if let Some(struct_schema) = self.structs.get_mut(LANG_STRUCT_NAME) {
            struct_schema.is_public = true;
        }

        for (path, template_params) in std::mem::take(&mut self.template_params) {
            let struct_name = self.path_to_struct_name(&path);
            let template_trait_name =
                self.template_trait_name(template_params.into_iter().collect_vec());

            // Overwrite the old `Type::StaticStr` value.
            self.path_to_type
                .insert(path.clone(), Type::Struct(struct_name));

            if let Some((parent, field_name)) = path.split() {
                if let Some(Type::Struct(struct_name)) = self.path_to_type.get(&parent) {
                    if let Some(struct_schema) = self.structs.get_mut(struct_name) {
                        if let Some(field) = struct_schema.fields.get_mut(field_name) {
                            if matches!(field.ty, StructFieldType::StaticStr) {
                                field.ty = StructFieldType::DynTemplate {
                                    template_trait_name,
                                };
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn template_trait_name(&mut self, mut params: Vec<TemplateParameter>) -> String {
        params.sort();
        let mut name = "Template".to_owned();
        for param in &params {
            name += "___";
            name += &param.name;
            name += "_";
            name += param.ty.short_ident();
        }
        self.template_traits
            .entry(name.clone())
            .or_insert_with(|| params.clone());
        name
    }
}

fn display_template_trait(
    indent: &str,
    f: &mut fmt::Formatter<'_>,
    trait_name: &str,
    params: &[TemplateParameter],
) -> fmt::Result {
    writeln!(f, "{indent}pub trait {trait_name}: Debug {{")?;
    writeln!(f, "{indent}    #[allow(unused)]")?;
    writeln!(f, "{indent}    fn with(")?;
    writeln!(f, "{indent}        &self,")?;
    for TemplateParameter { name, ty } in params {
        writeln!(f, "{indent}        {name}: {ty},")?;
    }
    writeln!(f, "{indent}    ) -> String;")?;
    writeln!(f, "{indent}}}")?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct TraitSchema {
    pub fields: IndexMap<String, StructFieldType>,
}
impl TraitSchema {
    fn display(&self, indent: &str, f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
        writeln!(f, "{indent}pub trait {name} {{")?;
        for (name, ty) in &self.fields {
            writeln!(f, "{indent}    fn {name}(&self) -> {ty};")?;
        }
        writeln!(f, "{indent}}}")?;
        Ok(())
    }
    fn display_impl(
        &self,
        indent: &str,
        f: &mut fmt::Formatter<'_>,
        trait_name: &str,
        struct_name: &str,
    ) -> fmt::Result {
        writeln!(f, "{indent}impl super::{trait_name} for {struct_name} {{")?;
        for (name, ty) in &self.fields {
            // Take a reference if the type isn't `Copy`.
            let ref_ = if ty.is_copy() { "" } else { "&" };
            write!(f, "{indent}    fn {name}(&self) -> {ref_}{ty} ")?;
            writeln!(f, "{{ {ref_}self.{name} }}")?;
        }
        writeln!(f, "{indent}}}")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct StructSchema {
    /// Whether the struct should be public.
    pub is_public: bool,
    /// Whether the struct should have a global default value.
    pub has_global_default: bool,
    /// Key to use to fill in default values.
    pub fallback_key: Option<String>,
    /// Contents of the struct.
    pub fields: IndexMap<String, StructField>,
}
impl StructSchema {
    fn display(&self, indent: &str, f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
        // Write struct definition.
        writeln!(f, "{indent}#[derive(Debug)]")?;
        writeln!(f, "{indent}pub struct {name} {{")?;
        for (field_name, field) in &self.fields {
            let ty = &field.ty;
            writeln!(f, "{indent}    pub {field_name}: {ty},")?;
        }
        writeln!(f, "{indent}}}")?;

        // Write global default value, if applicable.
        if self.has_global_default {
            writeln!(f, "{indent}impl {name} {{")?;
            writeln!(f, "{indent}    pub const DEFAULT: Self = Self {{")?;
            for (field_name, field) in &self.fields {
                match &field.ty {
                    StructFieldType::StaticStr => {
                        writeln!(f, "{indent}        {field_name}: \"\",")?;
                    }
                    ty => warn(&format!("no default value for `{field_name}: {ty}`")),
                }
            }
            writeln!(f, "{indent}    }};")?;
            writeln!(f, "{indent}}}")?;
        }

        Ok(())
    }

    fn fits_trait(&self, trait_schema: &TraitSchema) -> bool {
        trait_schema.fields.iter().all(|(name, ty)| {
            self.fields
                .get(name)
                .is_some_and(|struct_field| struct_field.ty == *ty)
        })
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub ty: StructFieldType,
    pub may_be_inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    StaticStr,
    Struct(String),
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::StaticStr => write!(f, "&'static str"),
            Type::Struct(struct_name) => write!(f, "{struct_name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructFieldType {
    StaticStr,
    DynTemplate { template_trait_name: String },
    Struct(String),
}
impl fmt::Display for StructFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticStr => write!(f, "&'static str"),
            Self::DynTemplate {
                template_trait_name,
                ..
            } => write!(f, "&'static dyn {template_trait_name}"),
            Self::Struct(struct_name) => write!(f, "{struct_name}"),
        }
    }
}
impl StructFieldType {
    pub fn is_copy(&self) -> bool {
        match self {
            Self::StaticStr => true,
            Self::DynTemplate { .. } => true,
            Self::Struct(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateParameter {
    pub name: String,
    pub ty: TemplateParameterType,
}
impl fmt::Display for TemplateParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { name, ty } = self;
        write!(f, "{name}: {ty}")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TemplateParameterType {
    String,
    Bool,
}
impl fmt::Display for TemplateParameterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String => write!(f, "&str"),
            Self::Bool => write!(f, "bool"),
        }
    }
}
impl TemplateParameterType {
    fn from_kdl(src: &SourceInfo, entry_ty: &KdlIdentifier) -> Option<Self> {
        match entry_ty.value() {
            "str" => Some(TemplateParameterType::String),
            "bool" => Some(TemplateParameterType::Bool),
            other => {
                warn_with(
                    "unknown parameter type (expected `str` or `bool`)",
                    src.at(entry_ty.span().offset()),
                    other.red().bold(),
                );
                None
            }
        }
    }

    fn short_ident(&self) -> &'static str {
        match self {
            TemplateParameterType::String => "str",
            TemplateParameterType::Bool => "bool",
        }
    }
}

fn with_blank_lines_between<'fmt, T>(
    f: &mut fmt::Formatter<'fmt>,
    iter: impl IntoIterator<Item = T>,
    mut func: impl FnMut(&mut fmt::Formatter<'fmt>, T) -> fmt::Result,
) -> fmt::Result {
    let mut is_first = true;
    for it in iter {
        if is_first {
            is_first = false;
        } else {
            writeln!(f)?;
        }
        func(f, it)?;
    }
    Ok(())
}

fn snake_case_segments_to_struct_name(
    segments: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    segments
        .into_iter()
        .map(|s| ident_case::RenameRule::PascalCase.apply_to_field(s))
        .join("_")
}

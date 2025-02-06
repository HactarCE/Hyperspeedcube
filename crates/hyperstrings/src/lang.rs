use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::Path;

use itertools::Itertools;
use kdl::{KdlDocument, KdlEntry};
use owo_colors::OwoColorize;
use regex::Regex;

use crate::schema::*;
use crate::util;
use crate::warn::*;

lazy_static! {
    pub static ref TEMPLATE_REGEX: Regex = Regex::new(r"\{\{([\s\S]*?)}}").expect("bad regex");
}

#[derive(Debug)]
pub struct Lang {
    pub name: String,
    pub values: HashMap<KeyPath, LangValue>,
}
impl Lang {
    pub fn display_module(&self, f: &mut fmt::Formatter<'_>, schema: &Schema) -> fmt::Result {
        writeln!(f, "#[rustfmt::skip]")?;
        writeln!(f, "#[allow(non_camel_case_types, non_upper_case_globals)]")?;
        writeln!(f, "pub mod {} {{", self.name)?;
        writeln!(f, "    use super::structs::*;")?;
        writeln!(f, "    use super::traits::*;")?;
        writeln!(f)?;
        self.display_consts(f, schema, &KeyPath::ROOT, LANG_STRUCT_NAME)?;
        writeln!(f, "}}")?;
        Ok(())
    }

    fn display_consts(
        &self,
        f: &mut fmt::Formatter<'_>,
        schema: &Schema,
        root_path: &KeyPath,
        struct_name: &str,
    ) -> fmt::Result {
        let mut template_queue = vec![];

        let mut struct_queue = VecDeque::from(vec![(root_path.clone(), struct_name)]);
        while let Some((path, struct_name)) = struct_queue.pop_front() {
            let const_name = path.const_name();
            write!(f, "    pub const {const_name}: {struct_name} = ",)?;

            if let Some(struct_schema) = schema.structs.get(struct_name) {
                writeln!(f, "{struct_name} {{")?;
                let mut any_missing = false;
                for (field_name, field_data) in &struct_schema.fields {
                    let field_path = path.push(field_name);
                    let field_value = match &field_data.ty {
                        StructFieldType::StaticStr => match self.values.get(&field_path) {
                            Some(LangValue::String(s)) => {
                                format!("{s:?}")
                            }
                            Some(LangValue::Template { .. }) => {
                                warn(&format!(
                                    "unexpected template at {field_path} in {}",
                                    self.name,
                                ));
                                "unimplemented!(\"unexpected template\")".to_owned()
                            }
                            None => {
                                any_missing = true;
                                continue;
                            }
                        },
                        StructFieldType::DynTemplate {
                            template_trait_name,
                        } => {
                            if let Some(value) = self.values.get(&field_path) {
                                let field_const_name = field_path.const_name();
                                let Some(Type::Struct(struct_name)) =
                                    schema.path_to_type.get(&field_path)
                                else {
                                    warn(&format!("expected template at `{field_path}`"));
                                    continue;
                                };
                                template_queue.push((
                                    field_path,
                                    struct_name,
                                    template_trait_name,
                                    value.segments(),
                                ));
                                format!("&{field_const_name}")
                            } else {
                                any_missing = true;
                                continue;
                            }
                        }
                        StructFieldType::Struct(new_struct_name) => {
                            let field_const_name = field_path.const_name();
                            struct_queue.push_back((field_path, new_struct_name));
                            field_const_name
                        }
                    };
                    writeln!(f, "        {field_name}: {field_value},")?;
                }
                if any_missing {
                    write!(f, "        ..")?;
                    match &struct_schema.fallback_key {
                        Some(fallback_key) => {
                            let fallback_const = KeyPath(fallback_key.clone()).const_name();
                            write!(f, "{fallback_const}")?;
                        }
                        None => {
                            if let Some(l) =
                                schema.fallback_lang.as_ref().filter(|&l| *l != self.name)
                            {
                                write!(f, "super::{l}::{const_name}")?;
                            } else if struct_schema.has_global_default {
                                write!(f, "{struct_name}::DEFAULT")?;
                            } else {
                                write!(f, "unimplemented!(\"no default\")")?;
                            }
                        }
                    };
                    writeln!(f)?;
                }
                writeln!(f, "    }};")?;
            } else {
                warn(&format!("missing struct schema for `{struct_name}`"));
                writeln!(f, "todo!();")?;
                continue;
            }
        }

        for (path, struct_name, template_trait_name, segments) in template_queue {
            const INDENT3: &str = "            ";
            const INDENT4: &str = "                ";

            let const_name = path.const_name();
            writeln!(f, "    const {const_name}: {struct_name} = {struct_name};")?;
            writeln!(f, "    #[derive(Debug, Default, Copy, Clone)]")?;
            writeln!(f, "    struct {struct_name};")?;

            writeln!(f, "    impl {template_trait_name} for {struct_name} {{")?;
            writeln!(f, "        fn with(")?;
            writeln!(f, "{INDENT3}&self,")?;
            let params = schema
                .template_traits
                .get(template_trait_name)
                .expect("templates not finalized");
            for TemplateParameter { name, ty } in params {
                writeln!(f, "{INDENT3}{name}: {ty},")?;
            }
            writeln!(f, "        ) -> String {{")?;
            writeln!(f, "{INDENT3}String::new()")?;
            let mut unused_params: HashSet<&String> = params.iter().map(|p| &p.name).collect();
            for segment in segments {
                write!(f, "{INDENT4}+ ")?;
                match segment {
                    TemplateSegment::Literal(s) => writeln!(f, "{s:?}")?,
                    TemplateSegment::StringParam { param_name } => {
                        unused_params.remove(&param_name);
                        writeln!(f, "{param_name}")?;
                    }
                    TemplateSegment::BoolParam {
                        param_name,
                        literal,
                    } => {
                        unused_params.remove(&param_name);
                        writeln!(f, "if {param_name} {{ {literal:?} }} else {{ \"\" }}")?;
                    }
                }
            }
            if !unused_params.is_empty() {
                warn(&format!(
                    "unused parameters: {params} at `{path}` in `{lang}`",
                    params = unused_params
                        .into_iter()
                        .map(|p| format!("`{p}`"))
                        .join(", "),
                    lang = self.name,
                ));
            }
            writeln!(f, "        }}")?;
            writeln!(f, "    }}")?;
        }

        Ok(())
    }

    pub fn from_file(path: impl AsRef<Path>, schema: &mut Schema) -> Self {
        let (src, doc) = util::read_kdl_file(&path);

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
        lang.populate_from_node_children(&src, schema, &KeyPath::ROOT, &doc);
        lang
    }

    fn populate_from_node_children(
        &mut self,
        src: &SourceInfo,
        schema: &mut Schema,
        path: &KeyPath,
        doc: &KdlDocument,
    ) {
        for node in doc.nodes() {
            let field_path = path.push(node.name().value());

            let Some(field_type) = schema.path_to_type.get(&field_path) else {
                util::ignore_node(src, node);
                continue;
            };

            // Check for type mismatch
            if let Some(node_ty) = node.ty() {
                let annotated_field_type = Type::Struct(node_ty.value().to_owned());
                if annotated_field_type != *field_type {
                    warn_with(
                        "type mismatch",
                        src.at(node.span().offset()),
                        format!(
                            "expected `{}`, got `{}`",
                            field_type.red(),
                            annotated_field_type.red(),
                        ),
                    );
                }
            }

            match field_type {
                Type::Struct(struct_name) if schema.structs.contains_key(struct_name) => {
                    let struct_schema = &schema.structs[struct_name];

                    let mut entries = node.entries().iter();
                    for (field_name, field_data) in struct_schema.fields.clone() {
                        if field_data.may_be_inline {
                            let field_path = field_path.push(&field_name);
                            if let Some(entry) = entries.next() {
                                util::ignore_entry_type(src, entry);
                                util::ignore_entry_name(src, entry);
                                self.set_value_from_entry(src, schema, field_path, entry);
                            }
                        }
                    }
                    util::ignore_entries(src, entries);

                    if let Some(children) = node.children() {
                        self.populate_from_node_children(src, schema, &field_path, children);
                    }
                }

                Type::StaticStr | Type::Struct(_) => {
                    util::ignore_children(src, node);
                    let mut entries = node.entries().iter();
                    if let Some(entry) =
                        util::take_entry(src, node, &mut entries, "expected string value")
                    {
                        util::ignore_entry_type(src, entry);
                        util::ignore_entry_name(src, entry);
                        self.set_value_from_entry(src, schema, field_path, entry);
                    } else {
                        continue;
                    };
                    util::ignore_entries(src, entries);
                }
            }
        }
    }

    fn set_value_from_entry(
        &mut self,
        src: &SourceInfo,
        schema: &mut Schema,
        field_path: KeyPath,
        entry: &KdlEntry,
    ) {
        let Some(s) = util::take_entry_str_value(src, entry) else {
            return;
        };
        // Parse template
        let mut segments = vec![];
        let mut parameters = vec![];
        let mut last_index = 0;
        for captures in TEMPLATE_REGEX.captures_iter(s) {
            let whole = captures.get(0).unwrap();
            segments.push(s[last_index..whole.start()].into());
            let (param, param_segment) = parse_template_inner(captures.get(1).unwrap().as_str());
            parameters.push(param);
            segments.push(param_segment);
            last_index = whole.end();
        }
        segments.push(s[last_index..].into());

        for param in &parameters {
            schema.infer_template_param(&field_path, param);
        }

        let value = if segments.len() == 1 {
            LangValue::String(s.to_string())
        } else {
            LangValue::Template { segments }
        };
        if self.values.contains_key(&field_path) {
            warn_with(
                "duplicate value",
                src.at(entry.span().offset()),
                field_path.red(),
            );
        }
        self.values.insert(field_path, value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LangValue {
    String(String),
    Template { segments: Vec<TemplateSegment> },
}
impl LangValue {
    pub fn segments(&self) -> Vec<TemplateSegment> {
        match self.clone() {
            LangValue::String(s) => vec![TemplateSegment::Literal(s)],
            LangValue::Template { segments } => segments,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSegment {
    Literal(String),
    StringParam { param_name: String },
    BoolParam { param_name: String, literal: String },
}
impl From<String> for TemplateSegment {
    fn from(value: String) -> Self {
        Self::Literal(value)
    }
}
impl From<&str> for TemplateSegment {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}

pub fn parse_template_inner(contents: &str) -> (TemplateParameter, TemplateSegment) {
    if let Some((name, contents_if_true)) = contents.split_once('?') {
        (
            TemplateParameter {
                name: name.to_owned(),
                ty: TemplateParameterType::Bool,
            },
            TemplateSegment::BoolParam {
                param_name: name.to_owned(),
                literal: contents_if_true.to_owned(),
            },
        )
    } else {
        (
            TemplateParameter {
                name: contents.to_owned(),
                ty: TemplateParameterType::String,
            },
            TemplateSegment::StringParam {
                param_name: contents.to_owned(),
            },
        )
    }
}

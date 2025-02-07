use std::fmt;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Token;

use crate::fields::KdlFieldStyle;

pub(crate) struct StructAttrs {
    pub node_name: Option<String>,
}

pub(crate) fn parse_kdl_struct_attrs(attrs: &[syn::Attribute]) -> StructAttrs {
    let mut node_name = None;
    for attr in crate::attrs::find_kdl_attrs(attrs) {
        match attr {
            KdlAttr::Name(name) => {
                assert!(
                    node_name.is_none(),
                    "duplicate #[kdl(name = ...)] attribute"
                );
                node_name = Some(name);
            }
            _ => panic!("invalid attribute {attr} for struct"),
        }
    }
    StructAttrs { node_name }
}

pub(crate) struct KdlFieldAttrs {
    /// How to represent the field in KDL.
    pub style: KdlFieldStyle,
    /// Whether the field is optional.
    pub optional: bool,
    /// Default value (skip serializing if equal).
    ///
    /// Not allowed with `optional = true`.
    pub default_value: Option<TokenStream>,
    /// Proxy struct to use.
    pub proxy: Option<syn::Path>,
}

pub(crate) fn parse_kdl_field_attrs(arg_index: &mut usize, field: &syn::Field) -> KdlFieldAttrs {
    let mut style = None;
    let mut optional = false;
    let mut default_value = None;
    let mut proxy = None;
    for attr in crate::attrs::find_kdl_attrs(&field.attrs) {
        match attr {
            KdlAttr::Argument => {
                assert!(style.is_none(), "conflicting #[kdl] attributes");
                style = Some(KdlFieldStyle::Argument(*arg_index));
                *arg_index += 1;
            }
            KdlAttr::Property(name) => {
                assert!(style.is_none(), "conflicting #[kdl] attributes");
                style = Some(KdlFieldStyle::Property(name));
            }
            KdlAttr::Child(name) => {
                assert!(style.is_none(), "conflicting #[kdl] attributes");
                style = Some(KdlFieldStyle::ChildNode(name));
            }
            KdlAttr::Children => {
                assert!(style.is_none(), "conflicting #[kdl] attributes");
                style = Some(KdlFieldStyle::Children);
            }
            KdlAttr::Optional => {
                optional = true;
            }
            KdlAttr::Default(expr) => {
                assert!(
                    default_value.is_none(),
                    "duplicate #[kdl(default = ...)] attribute",
                );
                let ty = &field.ty;
                default_value = Some(match expr {
                    Some(expr) => expr.to_token_stream(),
                    None => quote!(<#ty as ::std::default::Default>::default()),
                });
            }
            KdlAttr::Proxy(path) => {
                assert!(proxy.is_none(), "duplicate #[kdl(proxy = ...)] attribute",);
                proxy = Some(path);
            }
            _ => panic!("invalid attribute {attr} for field"),
        }
    }

    if optional && default_value.is_some() {
        panic!("#[kdl(optional)] and #[kdl(default)] are mutually exclusive");
    }

    if optional && matches!(style, Some(KdlFieldStyle::Argument(_))) {
        panic!("#[kdl(optional)] is not compatible with #[kdl(argument)]");
    }

    KdlFieldAttrs {
        style: style
            .expect("field is missing #[kdl(...)] attribute specifying how to serialize it"),
        optional,
        default_value,
        proxy,
    }
}

fn find_kdl_attrs(attrs: &[syn::Attribute]) -> Vec<KdlAttr> {
    let mut ret = vec![];
    for attr in attrs {
        if attr.meta.path().is_ident("kdl") {
            let nested = attr
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, Token![,]>::parse_terminated,
                )
                .unwrap();
            for meta in nested {
                ret.push(
                    match meta
                        .path()
                        .get_ident()
                        .expect("invalid #[kdl(...)] attribute")
                        .to_string()
                        .as_str()
                    {
                        "name" => KdlAttr::Name(match &meta.require_name_value().unwrap().value {
                            syn::Expr::Lit(syn::ExprLit {
                                attrs: _,
                                lit: syn::Lit::Str(s),
                            }) => s.value(),
                            _ => panic!("#[kdl(name = ...)] requires string literal"),
                        }),
                        "argument" => {
                            meta.require_path_only()
                                .expect("invalid form of #[kdl(argument)]");
                            KdlAttr::Argument
                        }
                        "property" => KdlAttr::Property(
                            meta.require_list()
                                .unwrap()
                                .parse_args::<syn::LitStr>()
                                .unwrap()
                                .value(),
                        ),
                        "child" => KdlAttr::Child({
                            meta.require_list()
                                .unwrap()
                                .parse_args::<syn::LitStr>()
                                .unwrap()
                                .value()
                        }),
                        "children" => {
                            meta.require_path_only().unwrap();
                            KdlAttr::Children
                        }
                        "optional" => {
                            meta.require_path_only().unwrap();
                            KdlAttr::Optional
                        }
                        "default" => match meta {
                            syn::Meta::Path(_) => KdlAttr::Default(None),
                            syn::Meta::List(_) => panic!("invalid form of #[kdl(default)]"),
                            syn::Meta::NameValue(meta_name_value) => {
                                KdlAttr::Default(Some(meta_name_value.value.clone()))
                            }
                        },
                        "proxy" => {
                            KdlAttr::Proxy(match &meta.require_name_value().unwrap().value {
                                syn::Expr::Path(expr_path) => expr_path.path.clone(),
                                _ => panic!("#[kdl(path = ...)] requires string literal"),
                            })
                        }
                        key => panic!("unknown #[kdl(...)] attribute {key:?}"),
                    },
                );
            }
        }
    }
    ret
}

enum KdlAttr {
    Name(String),

    Argument,
    Property(String),
    Child(String),
    Children,

    Optional,
    Default(Option<syn::Expr>),
    Proxy(syn::Path),
}
impl fmt::Display for KdlAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KdlAttr::Name(name) => write!(f, "#[kdl(name = {name:?})]"),
            KdlAttr::Argument => write!(f, "#[kdl(argument)]"),
            KdlAttr::Property(name) => write!(f, "#[kdl(property({name:?}))]"),
            KdlAttr::Child(name) => write!(f, "#[kdl(child({name:?}))]"),
            KdlAttr::Children => write!(f, "#[kdl(children)]"),
            KdlAttr::Optional => write!(f, "#[kdl(optional)]"),
            KdlAttr::Default(None) => write!(f, "#[kdl(default)]"),
            KdlAttr::Default(Some(expr)) => {
                write!(f, "#[kdl(default = {})]", expr.to_token_stream())
            }
            KdlAttr::Proxy(path) => write!(f, "#[kdl(proxy = {})]", path.to_token_stream()),
        }
    }
}

use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};

use crate::fields::{KdlField, KdlFieldStyle};

/// Destructure a struct or enum variant
#[must_use]
pub(crate) fn gen_destructure_struct_pattern(
    self_tokens: impl ToTokens,
    struct_fields: &syn::Fields,
    kdl_fields: &[KdlField<'_>],
) -> TokenStream {
    let field_tokens = TokenStream::from_iter(kdl_fields.iter().map(|field| {
        let ident = &field.ident;
        match &field.field.ident {
            Some(field_name) => quote! { #field_name: #ident, },
            None => quote! { #ident, },
        }
    }));

    let mut ret = self_tokens.to_token_stream();

    match struct_fields {
        syn::Fields::Named(_) => ret.append_all(quote!({ #field_tokens })),
        syn::Fields::Unnamed(_) => ret.append_all(quote!((#field_tokens))),
        syn::Fields::Unit => (),
    }

    ret
}

/// Construct KDL node
#[must_use]
pub(crate) fn gen_construct_node(
    node_name: impl ToTokens,
    kdl_fields: &[KdlField<'_>],
) -> TokenStream {
    let mut ret = quote! {
        let mut node = ::hyperkdl::kdl::KdlNode::new(#node_name);
    };
    for field in kdl_fields {
        let ident = &field.ident;
        match &field.attrs.style {
            KdlFieldStyle::Argument(_) => {
                let func = field.proxy_fn("ValueSchema", "to_kdl_value");
                ret.append_all(quote! { node.push(#func(#ident)); });
            }
            KdlFieldStyle::Property(name) => {
                let func = field.proxy_fn("ValueSchema", "to_kdl_value");
                ret.append_all(exec_if_non_default(
                    field,
                    quote! { node.push((#name, #func(#ident))); },
                ));
            }
            KdlFieldStyle::ChildNode(_) | KdlFieldStyle::Children => (),
        }
    }
    ret.append_all(gen_pack_node_children(kdl_fields));
    ret.append_all(quote! {
        if !children.nodes().is_empty() {
            node.set_children(children);
        }
    });
    ret
}

/// Construct KDL node children and store it in the variable `children`.
#[must_use]
pub(crate) fn gen_pack_node_children(kdl_fields: &[KdlField<'_>]) -> TokenStream {
    let mut ret = quote! {
        let mut children = ::hyperkdl::kdl::KdlDocument::new();
    };
    for field in kdl_fields {
        let ident = &field.ident;
        match &field.attrs.style {
            KdlFieldStyle::Argument(_) | KdlFieldStyle::Property(_) => (),
            KdlFieldStyle::ChildNode(name) => {
                let func = field.proxy_fn("NodeContentsSchema", "to_kdl_node_with_name");
                ret.append_all(exec_if_non_default(
                    field,
                    quote! { children.nodes_mut().push(#func(#ident, #name)); },
                ));
            }
            KdlFieldStyle::Children => {
                let func = field.proxy_fn("NodeSchema", "to_kdl_node");
                ret.append_all(exec_if_non_default(
                    field,
                    quote! { children.nodes_mut().extend(#ident.iter().map(#func)); },
                ));
            }
        }
    }
    ret
}

/// Handles the `optional` and `default` attribute when packing a field
#[must_use]
fn exec_if_non_default(
    field: &KdlField<'_>,
    serialize_field_statements: TokenStream,
) -> TokenStream {
    let ident = &field.ident;
    if field.attrs.optional {
        quote! { if let ::std::option::Option::Some(#ident) = #ident { #serialize_field_statements } }
    } else if let Some(default_value) = &field.attrs.default_value {
        quote! { if *#ident != #default_value { #serialize_field_statements } }
    } else {
        serialize_field_statements
    }
}

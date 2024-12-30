use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::token;

use crate::fields::{KdlField, KdlFieldStyle};

#[must_use]
pub(crate) fn gen_unpack_node_contents(kdl_fields: &[KdlField<'_>]) -> TokenStream {
    TokenStream::from_iter([
        // Unpack arguments and properties
        crate::from_kdl::gen_unpack_entries(&quote!(node), kdl_fields),
        // Unpack children
        crate::from_kdl::gen_unpack_option_children(&quote!(node.children()), kdl_fields),
    ])
}

/// Define a variable for each field
#[must_use]
pub(crate) fn gen_field_variables(fields: &[KdlField<'_>]) -> TokenStream {
    let mut ret = TokenStream::new();
    for field in fields {
        let ident = &field.ident;
        let ty = &field.field.ty;
        match field.attrs.style {
            KdlFieldStyle::Argument(_)
            | KdlFieldStyle::Property(_)
            | KdlFieldStyle::ChildNode(_) => {
                ret.append_all(if field.attrs.optional {
                    quote! {
                        let mut #ident: #ty = ::std::option::Option::None;
                    }
                } else {
                    quote! {
                        let mut #ident: ::std::option::Option<#ty> = ::std::option::Option::None;
                    }
                });
            }
            KdlFieldStyle::Children => {
                ret.append_all(quote! {
                    let mut #ident: #ty = ::std::vec::Vec::new();
                });
            }
        }
    }
    ret
}

/// Unpack arguments and properties of a node
#[must_use]
pub(crate) fn gen_unpack_entries(node: impl ToTokens, kdl_fields: &[KdlField<'_>]) -> TokenStream {
    // Construct a `match` arm for each argument
    let argument_match_arms =
        TokenStream::from_iter(
            kdl_fields
                .iter()
                .filter_map(|field| match &field.attrs.style {
                    KdlFieldStyle::Argument(index) => {
                        let ident = &field.ident;
                        let func = field.proxy_fn("ValueSchema", "from_kdl_entry");
                        Some(quote! { #index => #ident = #func(entry, ctx), })
                    }
                    _ => None,
                }),
        );

    // Construct a `match` arm for each property
    let property_match_arms =
        TokenStream::from_iter(
            kdl_fields
                .iter()
                .filter_map(|field| match &field.attrs.style {
                    KdlFieldStyle::Property(kdl_key) => {
                        let ident = &field.ident;
                        let func = field.proxy_fn("ValueSchema", "from_kdl_entry");
                        Some(quote! { #kdl_key => #ident = #func(entry, ctx), })
                    }
                    _ => None,
                }),
        );

    quote! {
        let mut arg_index = 0;
        for entry in #node.entries() {
            match entry.name() {
                None => {
                    let mut ctx = ctx.with(::hyperkdl::KeyPathElem::Argument(arg_index));
                    match arg_index {
                        #argument_match_arms
                        _ => ctx.warn_unused_arg(arg_index + 1, *entry.span()),
                    }
                    arg_index += 1;
                },
                Some(name) => {
                    let mut ctx = ctx.reborrow();
                    match name.value() {
                        #property_match_arms
                        key => ctx.warn_unknown_property(key, *entry.span()),
                    }
                },
            }
        }
    }
}

// Unpack children of a node
#[must_use]
pub(crate) fn gen_unpack_option_children(
    option_children: impl ToTokens,
    kdl_fields: &[KdlField<'_>],
) -> TokenStream {
    let inner = gen_unpack_children(quote!(children), kdl_fields);
    quote! {
        if let ::std::option::Option::Some(children) = #option_children {
            #inner
        }
    }
}

/// Unpack children of a node
#[must_use]
pub(crate) fn gen_unpack_children(
    children: impl ToTokens,
    kdl_fields: &[KdlField<'_>],
) -> TokenStream {
    let overflow_children_field = crate::fields::find_overflow_children_field(kdl_fields);

    // Construct a match arm for each child
    let children_match_arms =
        TokenStream::from_iter(kdl_fields.iter().filter_map(|field| match &field.attrs.style {
            KdlFieldStyle::ChildNode(kdl_node_name) => {
                let ident = &field.ident;
                let func = field.proxy_fn("NodeContentsSchema", "from_kdl_node_contents");
                Some(quote! {
                    #kdl_node_name => #ident = ::hyperkdl::NodeContentsSchema::from_kdl_node_contents(child_node, ctx),
                })
            },
            _ => None,
        }));
    let fallback_match_expr = match overflow_children_field {
        Some(field) => {
            let ident = &field.ident;
            quote! {
                if let ::std::option::Option::Some(value) = ::hyperkdl::NodeSchema::from_kdl_node(child_node, ctx) {
                    #ident.push(value);
                }
            }
        }
        None => quote! { ctx.warn_unknown_child(child_node.name().value(), *child_node.span()) },
    };

    quote! {
        for (i, child_node) in #children.nodes().iter().enumerate() {
            let mut ctx = ctx.with(::hyperkdl::KeyPathElem::Child(i));
            match child_node.name().value() {
                #children_match_arms
                _ => #fallback_match_expr,
            }
        }
    }
}

/// Construct a struct or enum variant
#[must_use]
pub(crate) fn gen_construct_struct(
    self_tokens: impl ToTokens,
    span: impl ToTokens,
    struct_fields: &syn::Fields,
    kdl_fields: &[KdlField<'_>],
) -> TokenStream {
    let mut field_tokens = TokenStream::new();
    for field in kdl_fields {
        let ident = &field.ident;
        if let Some(field_name) = &field.field.ident {
            field_tokens.append_all(quote! { #field_name: });
        }

        if matches!(field.attrs.style, KdlFieldStyle::Children) || field.attrs.optional {
            field_tokens.append_all(quote! { #ident });
        } else if let Some(default_value) = &field.attrs.default_value {
            field_tokens.append_all(quote! {
                #ident.unwrap_or_else(|| #default_value)
            });
        } else {
            let thing_missing = field.human_string();
            field_tokens.append_all(quote!(
                #ident.or_else(|| {
                    ctx.warn_missing(#thing_missing, #span);
                    None
                })?
            ));
        }

        field_tokens.append_all(quote!(,));
    }

    let mut ret = self_tokens.to_token_stream();

    match struct_fields {
        syn::Fields::Named(_) => ret.append_all(quote!({ #field_tokens })),
        syn::Fields::Unnamed(_) => ret.append_all(quote!((#field_tokens))),
        syn::Fields::Unit => (),
    }

    quote!(Some(#ret))
}

//! `#[derive]` macros for KDL serialization/deserialization built specifically
//! for [Hyperspeedcube](https://ajfarkas.dev/projects/hyperspeedcube/).
//!
//! # Field attributes
//!
//! Each field of a struct or enum may have a `#[kdl(...)]` attribute specifying
//! how it should be serialized.
//!
//! ## Field Style
//!
//! At most one of the following attributes are allowed on fields containing
//! simple values (numbers, strings, or other types that implement
//! [`hyperkdl::ValueSchema`]):
//!
//! - A field with no `#[kdl(...)]` attribute is serialized as an argument
//! - `#[kdl(property("..."))]` specifies the property key for a field that
//!   should be serialized as a property
//! - `#[kdl(child("..."))]` specifies the node name for a field that should be
//!   serialized as a value on a child node
//!
//! The following attributes are allowed on fields containing node content
//! values (types that implement [`hyperkdl::NodeContentsSchema`]):
//!
//! - `#[kdl(child("..."))]` specifies the node name for a field that should be
//!   serialized as a value on a child node
//!
//! The following attributes are allowed on fields containing a [`Vec`] of node
//! values (types that implement [`hyperkdl::NodeContents`]):
//!
//! - `#[kdl(children)]` specifies that the field should be serialized as a list
//!   of child nodes
//!
//! Within one struct or enum variant, no two fields may have the same property
//! name or child node name. Only one field may have the `#[kdl(children)]`
//! attribute. `#[kdl(child("..."))]` fields are matched first, and any
//! remaining fields are parsed into `#[kdl(children)]`.
//!
//! ## Other field properties
//!
//! - `#[kdl(optional)]` specifies that the field has type [`Option<T>`] and
//!   should be skipped when serializing [`None`] and assumed to be [`None`] if
//!   it is missing.
//! - `#[kdl(default = ...)]` specifies the default value for the field that
//!   should be assumed if it is missing (and the field will be skipped when
//!   serializing if it has this value).
//! - `#[kdl(default)]` is equivalent to `#[kdl(default =
//!   ::std::default::Default::default())]`
//! - `#[kdl(proxy = ...)]` specifies a proxy type, which can be used to
//!   implement serialization and deserialization for external types.
//!
//! `#[kdl(optional)]` is incompatible with `#[kdl(default)]`, `#[kdl(default =
//! ...)]`, and `#[kdl(argument)]`.
//!
//! # Example
//!
//! ```
//! #[derive(hyperkdl::Doc)]
//! struct RootStruct {
//!     #[kdl(child("thing"))]
//!     some_thing: i64,
//!     #[kdl(child("some-struct"))]
//!     some_struct: MyStruct,
//! }
//!
//! #[derive(hyperkdl::Node, hyperkdl::NodeContents)]
//! #[kdl(name = "my-struct")] // only used for `Node`; ignored for `NodeContents`
//! struct MyStruct {
//!     some_argument: String,
//!     #[kdl(property("property-key"))]
//!     some_property: i64,
//!     #[kdl(child("child-key"))]
//!     some_child: i64,
//!     #[kdl(children)]
//!     other_children: Vec<MyEnum>,
//! }
//!
//! #[derive(hyperkdl::Node)]
//! #[kdl(name = "uv-struct")]
//! struct UvStruct {
//!     #[kdl(property("u"))]
//!     u: i64,
//!     #[kdl(property("v"))]
//!     v: i64,
//! }
//!
//! #[derive(hyperkdl::Node)]
//! enum MyEnum {
//!     #[kdl(name = "variant-a")]
//!     A,
//!     #[kdl(name = "variant-b")]
//!     B(i64, i64)
//!     #[kdl(name = "variant-c")]
//!     C {
//!         field1: i64,
//!         #[kdl(property("field2"))]
//!         field2: String,
//!         #[kdl(child("field3"))]
//!         field3: String,
//!         #[kdl(child)]
//!         field4: UvStruct,
//!     },
//! }
//!
//! let expected = RootStruct {
//!     some_thing: 0,
//!     some_struct: MyStruct {
//!         some_argument: "a string here".to_string(),
//!         some_property: 42,
//!         some_child: 16,
//!         other_children: vec![
//!             A,
//!             C {
//!                 field1: 12,
//!                 field2: "another string".to_string(),
//!                 field3: "this field is a child",
//!                 field4: UvStruct {
//!                     u: 1920,
//!                     v: 1080,
//!                 },
//!             },
//!             B(2, 16),
//!             B(-3, -6),
//!         ]
//!     }
//! };
//!
//! let deserialized = MyStruct::from_kdl(
//!     &kdl::Document::from_str(
//!         r#"
//!             thing 0
//!             some-struct "a string here" property-key=42 {
//!                 a
//!                 c 12 field2="another string" {
//!                     wrapper-struct u=1920 v=1080
//!                     field3 "this field is a child"
//!                 }
//!                 b 2 16
//!                 child-key 16
//!                 b -3 -6
//!             }
//!         "#,
//!     )
//!     .unwrap(),
//! )
//! .unwrap();
//!
//! assert_eq!(expected, deserialized);
//! ```
//!
//! # Possible future features
//!
//! - Tuples of values with `#[kdl(child("..."))]`
//! - Repeated values as separate fields or as a list
//! - Loosen name overlap restrictions
//! - Associative maps (indexmap, hashmap, etc.)

#![allow(clippy::unwrap_used)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod fields;
mod from_kdl;
mod to_kdl;

use fields::KdlField;

/// Derive macro that implements [`hyperkdl::DocSchema`] for a struct.
///
/// Only `#[kdl(child("..."))]` and `#[kdl(children)]` attributes are allowed on
/// fields.
///
/// See crate documentation for field attributes and examples.
#[proc_macro_derive(Doc, attributes(kdl))]
pub fn derive_kdl_doc(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = input.ident;

    proc_macro::TokenStream::from(match input.data {
        syn::Data::Struct(data_struct) => {
            // Parse struct and field attributes
            let _struct_attrs = crate::attrs::parse_kdl_struct_attrs(&input.attrs);
            let kdl_fields: Vec<KdlField<'_>> = crate::fields::parse_fields(&data_struct.fields);

            // Ignore node name

            crate::fields::assert_only_children(&kdl_fields);

            let from_kdl_impl = TokenStream::from_iter([
                crate::from_kdl::gen_field_variables(&kdl_fields),
                crate::from_kdl::gen_unpack_children(quote!(doc), &kdl_fields),
                crate::from_kdl::gen_construct_struct(
                    &type_name,
                    quote!(doc.span()),
                    &data_struct.fields,
                    &kdl_fields,
                ),
            ]);

            let to_kdl_impl = TokenStream::from_iter([
                {
                    let destructuring_pattern = crate::to_kdl::gen_destructure_struct_pattern(
                        &type_name,
                        &data_struct.fields,
                        &kdl_fields,
                    );
                    quote! { let #destructuring_pattern = self; }
                },
                crate::to_kdl::gen_pack_node_children(&kdl_fields),
                quote! { children },
            ]);

            gen_impl_doc_schema(type_name, &from_kdl_impl, &to_kdl_impl)
        }
        syn::Data::Enum(_) => panic!("#[derive(NodeContents)] does not support enums"),
        syn::Data::Union(_) => panic!("#[derive(NodeContents)] does not support unions"),
    })
}

/// Derive macro that implements [`hyperkdl::NodeContentsSchema`] for a struct.
///
/// See crate documentation for field attributes and examples.
#[proc_macro_derive(NodeContents, attributes(kdl))]
pub fn derive_kdl_node_contents(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = input.ident;

    proc_macro::TokenStream::from(match input.data {
        syn::Data::Struct(data_struct) => {
            // Parse struct and field attributes
            let _struct_attrs = crate::attrs::parse_kdl_struct_attrs(&input.attrs);
            let kdl_fields: Vec<KdlField<'_>> = crate::fields::parse_fields(&data_struct.fields);

            // Ignore node name

            let from_kdl_impl = TokenStream::from_iter([
                crate::from_kdl::gen_field_variables(&kdl_fields),
                crate::from_kdl::gen_unpack_node_contents(&kdl_fields),
                crate::from_kdl::gen_construct_struct(
                    &type_name,
                    quote!(node.span()),
                    &data_struct.fields,
                    &kdl_fields,
                ),
            ]);

            let to_kdl_impl = TokenStream::from_iter([
                {
                    let destructuring_pattern = crate::to_kdl::gen_destructure_struct_pattern(
                        &type_name,
                        &data_struct.fields,
                        &kdl_fields,
                    );
                    quote! { let #destructuring_pattern = self; }
                },
                crate::to_kdl::gen_construct_node(quote!(node_name), &kdl_fields),
                quote! { node },
            ]);

            gen_impl_node_contents_schema(type_name, &from_kdl_impl, &to_kdl_impl)
        }
        syn::Data::Enum(_) => panic!("#[derive(NodeContents)] does not support enums"),
        syn::Data::Union(_) => panic!("#[derive(NodeContents)] does not support unions"),
    })
}

/// Derive macro that implements [`hyperkdl::NodeSchema`] for a struct or enum.
///
/// Every struct and enum variant must have a `#[kdl(name = "...")]` attribute
/// specifying the node name.
///
/// See crate documentation for field attributes and examples.
#[proc_macro_derive(Node, attributes(kdl))]
pub fn derive_kdl_node(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = input.ident;

    proc_macro::TokenStream::from(match input.data {
        syn::Data::Struct(data_struct) => {
            // Parse struct and field attributes
            let struct_attrs = crate::attrs::parse_kdl_struct_attrs(&input.attrs);
            let kdl_fields = crate::fields::parse_fields(&data_struct.fields);

            let expected_node_name = struct_attrs
                .node_name
                .expect("missing #[kdl(name = ...)] attribute (required for #[derive(Node)])");

            let from_kdl_impl = TokenStream::from_iter([
                // Check node name and add it to `ctx`
                quote! {
                    if #expected_node_name != node.name().value() {
                        ctx.warn_wrong_node_name(#expected_node_name, node.name().value(), node.span());
                        return None;
                    }
                    let mut ctx = ctx.with(::hyperkdl::KeyPathElem::Node(node.name()));
                },
                crate::from_kdl::gen_field_variables(&kdl_fields),
                crate::from_kdl::gen_unpack_node_contents(&kdl_fields),
                crate::from_kdl::gen_construct_struct(
                    &type_name,
                    quote!(node.span()),
                    &data_struct.fields,
                    &kdl_fields,
                ),
            ]);

            let to_kdl_impl = TokenStream::from_iter([
                {
                    let destructuring_pattern = crate::to_kdl::gen_destructure_struct_pattern(
                        &type_name,
                        &data_struct.fields,
                        &kdl_fields,
                    );
                    quote! { let #destructuring_pattern = self; }
                },
                crate::to_kdl::gen_construct_node(&expected_node_name, &kdl_fields),
                quote!(node),
            ]);

            gen_impl_node_schema(type_name, &from_kdl_impl, &to_kdl_impl)
        }
        syn::Data::Enum(data_enum) => {
            let mut from_kdl_match_arms = TokenStream::new();
            let mut to_kdl_match_arms = TokenStream::new();
            for variant in &data_enum.variants {
                // Parse variant and field attributes
                let variant_attrs = crate::attrs::parse_kdl_struct_attrs(&variant.attrs);
                let kdl_fields = crate::fields::parse_fields(&variant.fields);

                let node_name = variant_attrs
                    .node_name
                    .expect("missing #[kdl(name = ...)] attribute (required for enum variant)");

                let variant_ident = &variant.ident;
                let self_tokens = quote!(#type_name::#variant_ident);

                // from KDL
                let match_arm_contents = TokenStream::from_iter([
                    crate::from_kdl::gen_field_variables(&kdl_fields),
                    crate::from_kdl::gen_unpack_node_contents(&kdl_fields),
                    crate::from_kdl::gen_construct_struct(
                        &self_tokens,
                        quote!(node.span()),
                        &variant.fields,
                        &kdl_fields,
                    ),
                ]);
                from_kdl_match_arms.append_all(quote! { #node_name => { #match_arm_contents }, });

                // to KDL
                let destructuring_pattern = crate::to_kdl::gen_destructure_struct_pattern(
                    &self_tokens,
                    &variant.fields,
                    &kdl_fields,
                );
                let match_arm_contents = TokenStream::from_iter([
                    crate::to_kdl::gen_construct_node(&node_name, &kdl_fields),
                    quote! { node },
                ]);
                to_kdl_match_arms
                    .append_all(quote! { #destructuring_pattern => { #match_arm_contents }, });
            }

            let from_kdl_impl = quote!({
                let mut ctx = ctx.with(::hyperkdl::KeyPathElem::Node(node.name()));
                match node.name().value() {
                    #from_kdl_match_arms
                    node_name => {
                        ctx.warn_unknown_node_name(node_name, node.span());
                        None
                    }
                }
            });
            let to_kdl_impl = quote!(match self { #to_kdl_match_arms });

            gen_impl_node_schema(type_name, &from_kdl_impl, &to_kdl_impl)
        }
        syn::Data::Union(_) => panic!("#[derive(Node)] does not support unions"),
    })
}

#[must_use]
fn gen_impl_doc_schema(
    type_name: impl ToTokens,
    from_kdl_impl: impl ToTokens,
    to_kdl_impl: impl ToTokens,
) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ::hyperkdl::DocSchema for #type_name {
            fn from_kdl_doc(
                doc: &::hyperkdl::kdl::KdlDocument,
                mut ctx: ::hyperkdl::DeserCtx<'_>,
            ) -> ::std::option::Option<Self> {
                #from_kdl_impl
            }

            fn to_kdl_doc(&self) -> ::hyperkdl::kdl::KdlDocument {
                #to_kdl_impl
            }
        }
    }
}

#[must_use]
fn gen_impl_node_contents_schema(
    type_name: impl ToTokens,
    from_kdl_impl: impl ToTokens,
    to_kdl_impl: impl ToTokens,
) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ::hyperkdl::NodeContentsSchema for #type_name {
            fn from_kdl_node_contents(
                node: &::hyperkdl::kdl::KdlNode,
                mut ctx: ::hyperkdl::DeserCtx<'_>,
            ) -> ::std::option::Option<Self> {
                #from_kdl_impl
            }

            fn to_kdl_node_with_name(&self, node_name: &str) -> ::hyperkdl::kdl::KdlNode {
                #to_kdl_impl
            }
        }
    }
}

#[must_use]
fn gen_impl_node_schema(
    type_name: impl ToTokens,
    from_kdl_impl: impl ToTokens,
    to_kdl_impl: impl ToTokens,
) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ::hyperkdl::NodeSchema for #type_name {
            fn from_kdl_node(
                node: &::hyperkdl::kdl::KdlNode,
                mut ctx: ::hyperkdl::DeserCtx<'_>,
            ) -> ::std::option::Option<Self> {
                #from_kdl_impl
            }

            fn to_kdl_node(&self) -> ::hyperkdl::kdl::KdlNode {
                #to_kdl_impl
            }
        }
    }
}

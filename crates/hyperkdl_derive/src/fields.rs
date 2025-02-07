use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

/// Field of a struct or enum variant.
pub(crate) struct KdlField<'a> {
    /// Identifier for use in generated code.
    pub ident: syn::Ident,
    /// Parsed field from the struct or enum variant.
    pub field: &'a syn::Field,
    /// Field attributes.
    pub attrs: crate::attrs::KdlFieldAttrs,
}
impl KdlField<'_> {
    pub fn human_string(&self) -> String {
        match &self.attrs.style {
            KdlFieldStyle::Argument(i) => format!("argument #{}", i + 1),
            KdlFieldStyle::Property(k) => format!("property {k:?}"),
            KdlFieldStyle::ChildNode(k) => format!("child node {k:?}"),
            KdlFieldStyle::Children => "children".to_string(),
        }
    }

    pub fn proxy_fn(&self, trait_name: &str, fn_name: &str) -> TokenStream {
        match &self.attrs.proxy {
            Some(proxy_type) => {
                let proxy_trait_name = format_ident!("{trait_name}Proxy");
                let proxy_fn_name = format_ident!("proxy_{fn_name}");
                quote!(<#proxy_type as ::hyperkdl::#proxy_trait_name<_>>::#proxy_fn_name)
            }
            None => {
                let trait_name = format_ident!("{trait_name}");
                let fn_name = format_ident!("{fn_name}");
                quote!(::hyperkdl::#trait_name::#fn_name)
            }
        }
    }
}

/// How to represent a field of a struct or enum variant in KDL.
pub(crate) enum KdlFieldStyle {
    /// Argument with an index
    Argument(usize),
    /// Property with a name
    Property(String),
    /// Child node with a name
    ChildNode(String),
    /// Multiple children
    Children,
}

pub(crate) fn parse_fields(fields: &syn::Fields) -> Vec<KdlField<'_>> {
    let mut arg_index = 0;
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let ident = syn::Ident::new(&format!("prop_{i}"), Span::call_site());
            let attrs = crate::attrs::parse_kdl_field_attrs(&mut arg_index, field);

            KdlField {
                ident,
                field,
                attrs,
            }
        })
        .collect()
}

pub(crate) fn assert_only_children(fields: &[KdlField<'_>]) {
    assert!(
        fields.iter().all(|field| matches!(
            field.attrs.style,
            KdlFieldStyle::ChildNode(_) | KdlFieldStyle::Children
        )),
        "only #[kdl(child(...))] and #[kdl(children)] are allowed with #[kdl(root)]",
    );
}

pub(crate) fn find_overflow_children_field<'a, 'b>(
    fields: &'a [KdlField<'b>],
) -> Option<&'a KdlField<'b>> {
    let mut overflow_children_field = None;
    for field in fields {
        if matches!(field.attrs.style, KdlFieldStyle::Children) {
            assert!(
                overflow_children_field.is_none(),
                "multiple #[kdl(children)] is not allowed",
            );
            overflow_children_field = Some(field);
        }
    }
    overflow_children_field
}

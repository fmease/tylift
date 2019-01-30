//! Lift enum variants to the type-level.

#![cfg_attr(feature = "unstable", feature(proc_macro_diagnostic))]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Fields, Ident, ItemEnum};

macro_rules! identifier {
    ($fmt:literal) => {
        Ident::new(concat!("__", $fmt), Span::call_site())
    };
    ($fmt:literal $( $tokens:tt )+) => {
        Ident::new(&format!(concat!("__", $fmt) $( $tokens )*), Span::call_site())
    };
}

fn module(name: Ident, content: TokenStream2) -> TokenStream2 {
    quote! { mod #name { #content } }
}

/// Lift enum variants to the type-level.
#[proc_macro_attribute]
pub fn tylift(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);

    if !item.generics.params.is_empty() {
        #[cfg(feature = "unstable")]
        {
            use syn::spanned::Spanned;
            item.generics
                .params
                .span()
                .unstable()
                .error("type parameters cannot be lifted to the kind-level")
                .emit();
        }
        #[cfg(not(feature = "unstable"))]
        panic!("type parameters cannot be lifted to the kind-level")
    }

    let mut variants = Vec::new();

    for variant in &item.variants {
        if variant.ident == item.ident {
            #[cfg(feature = "unstable")]
            {
                variant
                    .ident
                    .span()
                    .unstable()
                    .error("name of variant matches name of enum")
                    .emit();
                continue;
            }
            #[cfg(not(feature = "unstable"))]
            panic!("name of variant matches name of enum")
        }
        let mut field_names = Vec::new();
        let mut field_types = Vec::new();
        match &variant.fields {
            Fields::Named(_) => {
                #[cfg(feature = "unstable")]
                {
                    variant
                        .ident
                        .span()
                        .unstable()
                        .error("variant must not have named fields")
                        .emit();
                    continue;
                }
                #[cfg(not(feature = "unstable"))]
                panic!("variant must not have named fields")
            }
            Fields::Unnamed(unnamed) => {
                for (index, field) in unnamed.unnamed.iter().enumerate() {
                    field_names.push(identifier!("T{}", index));
                    field_types.push(&field.ty);
                }
            }
            _ => {}
        }
        variants.push((&variant.ident, field_names, field_types));
    }

    let visibility = &item.vis;
    let kind = &item.ident;
    let clause = item.generics.where_clause;
    let kind_module = identifier!("tylift_enum_{}", kind);
    let sealed_module = identifier!("sealed");
    let sealed_trait = identifier!("Sealed");

    let mut output_stream = quote! { #visibility use #kind_module::*; };
    let mut kind_module_stream = quote! {
        use super::*;
        pub trait #kind: #sealed_module::#sealed_trait #clause {}
    };
    let mut sealed_module_stream = quote! {
        use super::*;
        pub trait #sealed_trait {}
    };
    for (name, field_names, field_types) in &variants {
        let parameters = quote! { <#(#field_names: #field_types),*> };
        let arguments = quote! { <#(#field_names),*> };
        kind_module_stream.extend(quote! {
            pub struct #name #parameters (::std::marker::PhantomData <(#(#field_names),*)>);
            impl #parameters #kind for #name #arguments {}
        });
        sealed_module_stream.extend(quote! {
            impl #parameters #sealed_trait for #name #arguments {}
        });
    }
    kind_module_stream.extend(module(sealed_module, sealed_module_stream));
    output_stream.extend(module(kind_module, kind_module_stream));
    output_stream.into()
}

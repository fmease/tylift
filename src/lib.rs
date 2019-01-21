//! Lift enum variants to the type-level.

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

macro_rules! identifier {
    ($fmt:literal $( $tokens:tt )*) => {
        syn::Ident::new(
            &format!(concat!("__", $fmt) $( $tokens )*),
            proc_macro2::Span::call_site(),
        )
    };
}

fn module(name: syn::Ident, content: TokenStream2) -> TokenStream2 {
    quote! { mod #name { #content } }
}

/// Lift enum variants to the type-level.
#[proc_macro_attribute]
pub fn tylift(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::DeriveInput);

    if !item.generics.params.is_empty() {
        panic!("type parameters cannot be lifted to the kind-level")
    }

    let mut variants = Vec::new();

    match &item.data {
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                use syn::Fields::*;
                if variant.ident == item.ident {
                    panic!("name of variant matches name of enum")
                }
                let mut field_names = Vec::new();
                let mut field_types = Vec::new();
                match &variant.fields {
                    Named(_) => panic!("variants must not have named fields"),
                    Unnamed(unnamed) => {
                        for (index, field) in unnamed.unnamed.iter().enumerate() {
                            field_names.push(identifier!("T{}", index));
                            field_types.push(&field.ty);
                        }
                    }
                    _ => {}
                }
                variants.push((&variant.ident, field_names, field_types));
            }
        }
        _ => panic!("only enums can be lifted"),
    }

    let vis = &item.vis;
    let kind = &item.ident;
    let clause = item.generics.where_clause;
    let scope = identifier!("tylift_enum_{}", kind);
    let sealed_mod = identifier!("sealed");
    let sealed_trait = identifier!("Sealed");

    let mut stream = quote! { #vis use self::#scope::*; };
    let mut scoped_stream = quote! {
        use super::*;
        pub trait #kind: #sealed_mod::#sealed_trait #clause {}
    };
    let mut sealed_mod_stream = quote! {
        use super::*;
        pub trait #sealed_trait {}
    };
    for (name, field_names, field_types) in &variants {
        let params = quote! { <#(#field_names: #field_types),*> };
        let args = quote! { <#(#field_names),*> };
        scoped_stream.extend(quote! {
            pub struct #name #params (!, ::std::marker::PhantomData <(#(#field_names),*)>);
            impl #params #kind for #name #args {}
        });
        sealed_mod_stream.extend(quote! {
            impl #params #sealed_trait for #name #args {}
        });
    }
    scoped_stream.extend(module(sealed_mod, sealed_mod_stream));
    stream.extend(module(scope, scoped_stream));
    stream.into()
}

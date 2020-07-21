#![warn(rust_2018_idioms, unused_must_use)] // never forbid for forward compatibility!

//! This is a libary for making type-level programming more ergonomic.
//! With the attribute `tylift`, one can lift variants of an `enum` to the type-level.
//!
//! ## Optional Features
//!
//! The feature-flag `span_errors` drastically improves error messages by taking
//! advantage of the span information of a token. It uses the experimental feature
//! `proc_macro_diagnostic` and thus requires a nightly `rustc`.

#![cfg_attr(feature = "span_errors", feature(proc_macro_diagnostic))]

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

macro_rules! report {
    ($span:expr, $message:literal $(, $next:tt )?) => {
        #[cfg(feature = "span_errors")]
        {
            $span
                .unstable()
                .error($message)
                .emit();
            $( $next )?
        }
        #[cfg(not(feature = "span_errors"))]
        panic!($message)
    }
}

/// The attribute promotes variants to their own types which will not be namespaced
/// by current design. The enum type becomes a kind emulated by a trait. In the
/// process, the original type gets replaced.
///
/// As of right now, there is no automated way to reify the lifted variants. Variants can hold
/// unnamed fields of types of given kind. Lifted enum types cannot be generic over kinds.
/// The promoted variants inherit the visibility of the lifted enum. Traits representing kinds
/// are sealed, which means nobody is able to add new types to the kind.
///
/// Attributes (notably documentation comments) applied to the item itself and its variants will be
/// preserved. On the other hand, attributes placed in front of fields of a variant
/// (constructor arguments) will not be translated and thus have no effect.
/// Explicit discriminants are ignored, too.
///
/// Example:
///
/// ```
/// use tylift::tylift;
/// use std::marker::PhantomData;
///
/// #[tylift]
/// pub enum Mode {
///     Safe,
///     Fast,
/// }
///
/// pub struct Text<M: Mode> {
///     content: String,
///     _marker: PhantomData<M>,
/// }
///
/// impl<M: Mode> Text<M> {
///     pub fn into_inner(self) -> String {
///         self.content
///     }
/// }
///
/// impl Text<Safe> {
///     pub fn from(content: Vec<u8>) -> Option<Self> {
///         Some(Self {
///             content: String::from_utf8(content).ok()?,
///             _marker: PhantomData,
///         })
///     }
/// }
///
/// impl Text<Fast> {
///     pub fn from(content: Vec<u8>) -> Self {
///         Self {
///             content: unsafe { String::from_utf8_unchecked(content) },
///             _marker: PhantomData,
///         }
///     }
/// }
///
/// fn main() {
///     let safe = Text::<Safe>::from(vec![0x73, 0x61, 0x66, 0x65]);
///     let fast = Text::<Fast>::from(vec![0x66, 0x61, 0x73, 0x74]);
///     assert_eq!(safe.map(Text::into_inner), Some("safe".to_owned()));
///     assert_eq!(fast.into_inner(), "fast".to_owned());
/// }
/// ```
#[proc_macro_attribute]
pub fn tylift(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);

    if !item.generics.params.is_empty() {
        #[allow(unused_imports)]
        use syn::spanned::Spanned;
        report!(
            item.generics.params.span(),
            "type parameters cannot be lifted to the kind-level"
        );
    }

    let mut variants = Vec::new();

    for variant in &item.variants {
        if variant.ident == item.ident {
            report!(
                variant.ident.span(),
                "name of variant matches name of enum",
                continue
            );
        }
        let mut field_names = Vec::new();
        let mut field_types = Vec::new();
        match &variant.fields {
            Fields::Named(_) => {
                report!(
                    variant.ident.span(),
                    "variant must not have named fields",
                    continue
                );
            }
            Fields::Unnamed(unnamed) => {
                for (index, field) in unnamed.unnamed.iter().enumerate() {
                    field_names.push(identifier!("T{}", index));
                    field_types.push(&field.ty);
                }
            }
            _ => {}
        }
        variants.push((&variant.attrs, &variant.ident, field_names, field_types));
    }

    let attributes = &item.attrs;
    let visibility = &item.vis;
    let kind = &item.ident;
    let clause = item.generics.where_clause;
    let kind_module = identifier!("tylift_kind_{}", kind);
    let sealed_module = identifier!("sealed");
    let sealed_trait = identifier!("Sealed");

    let mut output_stream = quote! { #visibility use #kind_module::*; };
    let mut kind_module_stream = quote! {
        use super::*;
        #(#attributes)*
        pub trait #kind: #sealed_module::#sealed_trait #clause {}
    };
    let mut sealed_module_stream = quote! {
        use super::*;
        pub trait #sealed_trait {}
    };
    for (attributes, name, field_names, field_types) in &variants {
        let &attributes = attributes;
        let parameters = quote! { <#(#field_names: #field_types),*> };
        let arguments = quote! { <#(#field_names),*> };
        kind_module_stream.extend(quote! {
            #(#attributes)*
            pub struct #name #parameters (::core::marker::PhantomData <(#(#field_names),*)>);
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

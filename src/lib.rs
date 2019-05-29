#![forbid(bare_trait_objects, unused_must_use)]

//! This is a libary for making type-level programming more ergonomic.
//! With the attribute `tylift`, one can lift variants of an `enum` to the type-level.
//!
//! ## Optional Features
//!
//! The feature-flag `span_errors` drastically improves error messages by taking
//! advantage of the span information of a token. It uses the experimental feature
//! `proc_macro_diagnostic` and thus requires a nightly `rustc`.

#![cfg_attr(feature = "span_errors", feature(proc_macro_diagnostic))]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Fields, Ident, ItemEnum, ItemFn};

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
// @Task disallow explicit discriminants and attributes on the fields of variants
// for forward-compatibility
pub fn tylift(attr: TokenStream, item: TokenStream) -> TokenStream {
    // forward-compatibility
    assert!(attr.is_empty());

    // @Task differenciate between lifting enums vs fns
    // @Beacon @Question does syn provide some way of specifying Either<ItemEnum, ItemFn>
    // or we need to do this manually?
    let item = parse_macro_input!(item as ItemEnum);

    if !item.generics.params.is_empty() {
        #[cfg(feature = "span_errors")]
        {
            use syn::spanned::Spanned;
            item.generics
                .params
                .span()
                .unstable()
                .error("type parameters cannot be lifted to the kind-level")
                .emit();
        }
        #[cfg(not(feature = "span_errors"))]
        panic!("type parameters cannot be lifted to the kind-level")
    }

    let mut variants = Vec::new();

    for variant in &item.variants {
        if variant.ident == item.ident {
            #[cfg(feature = "span_errors")]
            {
                variant
                    .ident
                    .span()
                    .unstable()
                    .error("name of variant matches name of enum")
                    .emit();
                continue;
            }
            #[cfg(not(feature = "span_errors"))]
            panic!("name of variant matches name of enum")
        }
        let mut field_names = Vec::new();
        let mut field_types = Vec::new();
        match &variant.fields {
            Fields::Named(_) => {
                #[cfg(feature = "span_erros")]
                {
                    variant
                        .ident
                        .span()
                        .unstable()
                        .error("variant must not have named fields")
                        .emit();
                    continue;
                }
                #[cfg(not(feature = "span_errors"))]
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
        variants.push((&variant.attrs, &variant.ident, field_names, field_types));
    }

    let attributes = &item.attrs;
    let visibility = &item.vis;
    let kind = &item.ident;
    let clause = item.generics.where_clause;
    let kind_module = identifier!("tylift_kind_{}", kind);
    let dummy = identifier!("Dummy");
    let sealed_module = identifier!("sealed");
    let sealed_trait = identifier!("Sealed");

    let items = std::iter::once(&kind).chain(variants.iter().map(|(_, name, ..)| name));
    let mut output_stream = quote! { #visibility use #kind_module::{#(#items),*}; };
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
            pub struct #name #parameters (::std::marker::PhantomData <(#(#field_names),*)>);
            impl #parameters #kind for #name #arguments {}
        });
        sealed_module_stream.extend(quote! {
            impl #parameters #sealed_trait for #name #arguments {}
        });
    }
    kind_module_stream.extend(quote! {
        #[doc(hidden)]
        pub enum #dummy {}
        impl #kind for #dummy {}
    });
    sealed_module_stream.extend(quote! {
        impl #sealed_trait for #dummy {}
    });
    kind_module_stream.extend(module(sealed_module, sealed_module_stream));
    output_stream.extend(module(kind_module, kind_module_stream));
    output_stream.into()
}

// @Temp
#[cfg(feature = "tyfns")]
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __lift_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    // @Task handle those fields:
    // item.attrs
    // item.vis

    // forward-compatibility
    assert!(item.constness.is_none());
    assert!(item.unsafety.is_none());
    assert!(item.asyncness.is_none());
    assert!(item.abi.is_none());
    assert!(item.decl.variadic.is_none());

    // @Task error message
    assert!(item.decl.generics.params.is_empty());
    assert!(item.decl.generics.where_clause.is_none());

    let function = item.ident;

    // @Note handling function arguments…

    // @Question instead of two Vecs, use a Vec of tuples?
    let mut parameters = Vec::new();
    let mut parameter_kinds = Vec::new();

    // @Temp @Question verify content later?
    for parameter in item.decl.inputs {
        if let syn::FnArg::Captured(syn::ArgCaptured { pat, ty, .. }) = parameter {
            if let syn::Pat::Ident(syn::PatIdent {
                ident,
                by_ref,
                mutability,
                subpat,
            }) = pat
            {
                assert!(by_ref.is_none());
                assert!(mutability.is_none());
                assert!(subpat.is_none());
                parameters.push(ident);
            } else {
                panic!(); // @Temp
            }
            if let syn::Type::Path(ty) = ty {
                assert!(ty.qself.is_none());
                // @Temp
                assert!(ty.path.segments.len() == 1);
                let ty = ty.path.segments.first().unwrap();
                let ty = ty.value();
                assert!(ty.arguments == syn::PathArguments::None);
                parameter_kinds.push(ty.ident.clone());
            } else {
                panic!(); // @Temp
            }
        } else {
            panic!(); // @Temp
        }
    }

    let mut parameters = parameters.into_iter();
    let mut parameter_kinds = parameter_kinds.into_iter();

    // @Task error handling
    let first_parameter = parameters.next().unwrap();
    let first_parameter_kind = parameter_kinds.next().unwrap();

    // @Question maybe add some more checks (restrict it)?
    let result_kind = if let syn::ReturnType::Type(_, type_) = item.decl.output {
        type_
    } else {
        panic!(); // @Temp
    };

    // @Task handle body

    // @Note DRY
    let dummy = identifier!("Dummy");
    let kind_module = identifier!("tylift_kind_{}", first_parameter_kind);
    let dummy = quote! { #kind_module::#dummy };

    // @Temp dummy values
    let variants = vec![
        Ident::new("True", Span::call_site()),
        Ident::new("False", Span::call_site()),
    ];
    let result_types = vec![
        Ident::new("False", Span::call_site()),
        Ident::new("True", Span::call_site()),
    ];

    let function_implementation = identifier!("tylift_tyfn_{}", function);

    let mut implementation_stream = quote! {};

    for (variant, result_type) in variants.into_iter().zip(result_types) {
        implementation_stream.extend(quote! {
            impl #function_implementation for #variant {
                type Result = #result_type;
            }
        });
    }

    let output_stream = quote! {
        // @Note <#first_parameter: #first_parameter_kind, #(#parameters: #parameter_kinds)*>
        type #function<#first_parameter: #first_parameter_kind> = <#first_parameter as #function_implementation>::Result;
        trait #function_implementation: #first_parameter_kind {
            type Result: #result_kind;
        }
        impl<#first_parameter: #first_parameter_kind> #function_implementation for #first_parameter {
            default type Result = #dummy;
        }
        impl #function_implementation for #dummy {
            type Result = #dummy;
        }

        #implementation_stream
    };

    output_stream.into()
}

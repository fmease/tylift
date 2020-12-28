#![warn(rust_2018_idioms, unused_must_use)] // never `forbid` for forward compatibility!

//! This is a libary for making type-level programming more ergonomic.
//! With the attribute `tylift`, one can lift variants of an `enum` to the type-level.
//!
//! ## `cargo` Features
//!
//! The feature-flag `span_errors` drastically improves error messages by taking
//! advantage of the span information of a token. It uses the experimental feature
//! `proc_macro_diagnostic` and thus requires a nightly `rustc`.

#![cfg_attr(feature = "span_errors", feature(proc_macro_diagnostic))]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Fields, Ident, ItemEnum};

fn identifier(identifier: &str) -> Ident {
    Ident::new(identifier, Span::mixed_site())
}

fn module(vis: &syn::Visibility, name: Ident, content: TokenStream2) -> TokenStream2 {
    quote! { #vis mod #name { #content } }
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

/// The attribute promotes enum variants to their own types.
/// The enum type becomes a [_kind_](https://en.wikipedia.org/wiki/Kind_(type_theory))
/// – the type of a type – emulated by a trait, replacing the original type declaration.
/// In Rust, the syntax of trait bounds (`:`) beautifully mirror the syntax of type annotations.
/// Thus, the snippet `B: Bool` can also be read as "type parameter `B` of kind `Bool`".
///
/// Traits representing kinds are _sealed_, which means nobody is able to add new types to
/// the kind. Variants can hold (unnamed) fields of types of a given kind.
/// Attributes (notably documentation comments) applied to the item itself and its variants will
/// be preserved. Expanded code works in `#![no_std]`-environments.
///
/// As of right now, there is no automated way to _reify_ the lifted variants (i.e. map them to
/// their term-level counterpart). Lifted enum types can _not_ be generic over kinds.
/// Attributes placed in front of fields of a variant (constructor arguments) will not be translated
/// and thus have no effect.
///
/// Examples:
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
///     pub unsafe fn from(content: Vec<u8>) -> Self {
///         Self {
///             content: unsafe { String::from_utf8_unchecked(content) },
///             _marker: PhantomData,
///         }
///     }
/// }
///
/// fn main() {
///     let safe = Text::<Safe>::from(vec![0x73, 0x61, 0x66, 0x65]);
///     let fast = unsafe { Text::<Fast>::from(vec![0x66, 0x61, 0x73, 0x74]) };
///     assert_eq!(safe.map(Text::into_inner), Some("safe".to_owned()));
///     assert_eq!(fast.into_inner(), "fast".to_owned());
/// }
///
/// #[tylift]
/// pub enum Bool {
///     False,
///     True,
/// }
///
/// #[tylift]
/// pub(crate) enum Nat {
///     Zero,
///     Succ(Nat),
/// }
///
/// #[tylift]
/// enum BinaryTree {
///     Leaf,
///     Branch(BinaryTree, Nat, BinaryTree),
/// }
///
/// #[tylift(mod)] // put the all 3 items into the module `Power`
/// pub enum Power {
///     On,
///     Off,
/// }
///
/// #[tylift(mod direction)] // put all 3 items into the module `direction`
/// pub(crate) enum Direction {
///     /// Higher and higher!
///     Up,
///     /// Lower and lower...
///     Down,
/// }
/// ```
#[proc_macro_attribute]
pub fn tylift(attr: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = match Arguments::parse(attr) {
        Ok(arguments) => arguments,
        // task: use report macro instead. blocker: Arguments::parse not being compatible
        // with syn/proc_macro2, needs to be adapted first
        Err(_span) => panic!("invalid arguments"),
    };
    let scoped = arguments.scope.is_some();

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
                    field_names.push(identifier(&format!("T{}", index)));
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

    let kind_module = match arguments.scope {
        // note: obviously, we'd like to have `Span::def_site()` to fully hide this module from outsiders
        // but it's still unstable unfortunately :/
        None => identifier(&format!("__kind_{}", kind)),
        Some(None) => kind.clone(),
        // no From impl exists...
        Some(Some(ident)) => identifier(&ident.to_string()),
    };
    let sealed_module = identifier("sealed");
    let sealed_trait = identifier("Sealed");

    let mut output_stream = quote! {};
    if !scoped {
        output_stream.extend(quote! { #visibility use #kind_module::*; });
    };
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
    kind_module_stream.extend(module(
        &syn::Visibility::Inherited,
        sealed_module,
        sealed_module_stream,
    ));
    output_stream.extend(module(visibility, kind_module, kind_module_stream));
    output_stream.into()
}

#[derive(Default)]
struct Arguments {
    /// The module under which the kind and its inhabitants live/are scoped.
    scope: Option<Option<proc_macro::Ident>>,
}

impl Arguments {
    // we should probably use the syn::parse::Parse API
    fn parse(tokens: TokenStream) -> Result<Self, proc_macro::Span> {
        let mut tokens = tokens.into_iter();
        let mut arguments = Self::default();

        if let Some(token) = tokens.next() {
            use proc_macro::TokenTree;

            if let TokenTree::Ident(identifier) = token {
                if identifier.to_string() != "mod" {
                    return Err(identifier.span());
                }
                arguments.scope = Some(None);
            } else {
                return Err(token.span());
            }

            if let Some(token) = tokens.next() {
                if let TokenTree::Ident(identifier) = token {
                    arguments.scope = Some(Some(identifier));
                } else {
                    return Err(token.span());
                }
            }

            if let Some(token) = tokens.next() {
                return Err(token.span());
            }
        }

        Ok(arguments)
    }
}

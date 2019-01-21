extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn tylift(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::DeriveInput);

    if !item.attrs.is_empty() {
        panic!("additional attributes not supported yet")
    }

    if !item.generics.params.is_empty() {
        panic!("type parameters cannot be lifted to kind-level yet");
    }

    let mut variants = Vec::new();

    match &item.data {
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                if variant.ident == item.ident {
                    panic!("name of variant matches name of enum");
                }
                if !variant.attrs.is_empty() {
                    panic!("field attributes not supported yet")
                }
                let mut field_names = Vec::new();
                let mut field_types = Vec::new();
                match &variant.fields {
                    syn::Fields::Named(_) => panic!("variants must not have named fields"),
                    syn::Fields::Unnamed(unnamed) => {
                        for (index, field) in unnamed.unnamed.iter().enumerate() {
                            if !field.attrs.is_empty() {
                                panic!("attributes of fields of fields not supported")
                            }
                            field_names.push(syn::Ident::new(
                                &format!("__T{}", index),
                                proc_macro2::Span::call_site(),
                            ));
                            field_types.push(&field.ty);
                        }
                    }
                    _ => {}
                }
                variants.push((&variant.ident, field_names, field_types));
            }
        }
        _ => panic!("expected enum"),
    }

    let vis = &item.vis;
    let r#trait = &item.ident;
    let r#where = item.generics.where_clause;

    let mut stream = quote! { #vis unsafe trait #r#trait #r#where {} };
    for (name, field_names, field_types) in &variants {
        let params = quote! { <#(#field_names: #field_types),*> };
        stream.extend(quote! {
            #vis struct #name #params (!, ::std::marker::PhantomData <(#(#field_names),*)>);
            unsafe impl #params #r#trait for #name <#(#field_names),*> {}
        });
    }
    stream.into()
}

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields, Ident, ImplItemFn, ItemImpl};

pub fn from_core_value_derive_impl(input: DeriveInput) -> TokenStream {
    let enum_name = input.ident;

    let Data::Enum(data_enum) = input.data else {
        panic!("#[derive(FromVariants)] can only be used on enums");
    };

    let mut from_impls = vec![];

    for variant in data_enum.variants {
        let variant_name = &variant.ident;
        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().unwrap().ty;
                from_impls.push(quote! {
                    impl From<#field_type> for #enum_name {
                        fn from(value: #field_type) -> Self {
                            #enum_name::#variant_name(value)
                        }
                    }
                });
            }
            _ => {
                panic!("Only tuple variants with a single field are supported")
            }
        }
    }

    TokenStream::from(quote! {
        #(#from_impls)*
    })
}

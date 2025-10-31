use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Variant};

pub fn derive_lib_type_string(input: DeriveInput) -> TokenStream {
    let name = input.ident;

    let Data::Enum(DataEnum { variants, .. }) = input.data else {
        panic!("#[derive(LibTypeString)] only works on enums");
    };

    // Create match arms for Display and FromStr
    let mut to_str_arms = Vec::new();
    let mut from_str_arms = Vec::new();

    for Variant { ident, fields, .. } in variants {
        let var_name = ident.to_string().to_lowercase();

        if fields.is_empty() {
            // Simple variant
            to_str_arms.push(quote! {
                #name::#ident => write!(f, "{}", #var_name),
            });
            from_str_arms.push(quote! {
                #var_name => Ok(#name::#ident),
            });
        } else {
            // Variant with data, e.g. Integer(Option<IntegerTypeVariant>)
            to_str_arms.push(quote! {
                #name::#ident(Some(inner)) => write!(f, "{}/{}", #var_name, inner.to_string().to_lowercase()),
            });
            to_str_arms.push(quote! {
                #name::#ident(None) => write!(f, "{}", #var_name),
            });
            from_str_arms.push(quote! {
                s if s.starts_with(concat!(#var_name, "/")) => {
                    let suffix = &s[#var_name.len()+1..];
                    Ok(#name::#ident(Some(suffix.parse().map_err(|_| format!("Invalid {} variant: {}", #var_name, suffix))?)))
                }
            });
            from_str_arms.push(quote! {
                #var_name => Ok(#name::#ident(None)),
            });
        }
    }

    let expanded = quote! {
        impl core::fmt::Display for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    #(#to_str_arms)*
                }
            }
        }

        impl core::str::FromStr for #name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms)*
                    _ => Err(format!("Unknown variant for {}: {}", stringify!(#name), s)),
                }
            }
        }
    };

    expanded
}

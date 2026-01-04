use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::DeriveInput;
/// Unfinished and broken, for serde use only to convert bitfield structs to/from json
pub fn derive_bitfield_serde(input: DeriveInput) -> TokenStream {
    let ident = input.ident;

    let fields = if let syn::Data::Struct(data) = &input.data {
        if let syn::Fields::Named(named) = &data.fields {
            named.named.iter().collect::<Vec<_>>()
        } else {
            core::panic!("#[derive(BitfieldSerde)] requires named fields");
        }
    } else {
        core::panic!("#[derive(BitfieldSerde)] only works on structs");
    };

    // Collect TokenStreams for reuse
    let field_defs: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = &f.ident;
            let ty = &f.ty;
            quote! { #name: #ty }
        })
        .collect();

    let getters: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = &f.ident;
            quote! { #name: self.#name() }
        })
        .collect();

    let setters: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_ident = f.ident.as_ref().unwrap();
            let setter_ident = proc_macro2::Ident::new(
                &format!("with_{}", field_ident),
                Span::call_site(),
            );
            quote! { #setter_ident(helper.#field_ident) }
        })
        .collect();

    // Generate the final impls
    let expanded = quote! {
        impl ::serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: ::serde::Serializer
            {
                #[derive(::serde::Serialize)]
                struct Helper {
                    #( #field_defs, )*
                }

                let helper = Helper {
                    #( #getters, )*
                };

                helper.serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: ::serde::Deserializer<'de>
            {
                #[derive(::serde::Deserialize)]
                struct Helper {
                    #( #field_defs, )*
                }

                let helper = Helper::deserialize(deserializer)?;
                Ok(#ident::new() #( #setters )* )
            }
        }
    };

    expanded
}

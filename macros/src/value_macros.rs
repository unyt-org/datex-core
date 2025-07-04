use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

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
            _ => {}
        }
    }

    quote! {
        #(#from_impls)*
    }
}

/// Derives the `DatexStruct` trait for a struct.
/// This macro generates a method `value_container` that converts the struct
/// into a `ValueContainer` by creating an `Object` and setting its fields.
pub fn derive_datex_struct(input: DeriveInput) -> TokenStream {
    let ident = input.ident;

    let body = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(ref fields) => {
                let setters = fields.named.iter().map(|f| {
                    let fname_ident = f.ident.as_ref().unwrap();
                    let fname_string = fname_ident.to_string();
                    quote! { obj.set(#fname_string, datex_core::values::value_container::ValueContainer::from(self.#fname_ident.clone())); }
                });
                quote! {
                    let mut obj = datex_core::values::core_values::object::Object::new();
                    #(#setters)*
                    datex_core::values::value_container::ValueContainer::from(obj)
                }
            }
            _ => panic!("DxSerialize only supports structs with named fields"),
        },
        _ => panic!("DxSerialize can only be derived for structs"),
    };

    TokenStream::from(quote! {
        impl #ident {
            pub fn value_container(&self) -> datex_core::values::value_container::ValueContainer {
                #body
            }
        }
        impl Into<datex_core::values::value_container::ValueContainer> for #ident {
            fn into(self) -> datex_core::values::value_container::ValueContainer {
                self.value_container()
            }
        }

        // impl From<#ident> for datex_core::values::value_container::ValueContainer {
        //     fn from(value: #ident) -> Self {
        //         value.value_container()
        //     }
        // }
    })
}

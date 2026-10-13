use crate::datex_proxy::data::{EnumVariant, Fields, Structure, StructureData};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::Path;

/// Represents whether the self value is borrowed or moved in the context of generating code for struct or enum variants.
pub enum SelfAccess {
    /// Indicates that the self value is borrowed, and field access should use references.
    Borrowed,
    /// Indicates that the self value is mutably borrowed, and field access should use mutable references.
    BorrowedMut,
    /// Indicates that the self value is borrowed, but accessed via a specific identifier (e.g., `value`), and field access should use references.
    BorrowedIdent { self_value: Ident, self_type: Path },
    /// Indicates that the self value is moved, and field access should take ownership of the fields.
    Moved,
    /// Indicates that the self is passed as a `Box<Self>`
    Boxed,
}

/// Generates a mapping for the fields of a struct or the variants of an enum, depending on the structure type.
/// The `fields_mapping` function is called with the fields of the struct or the fields of each enum variant, and should return a TokenStream representing the mapping for those fields.
pub fn generate_struct_or_enum_variants_fields_mapping(
    structure: &Structure,
    self_access: SelfAccess,
    fields_mapping: impl Fn(&Fields, Option<&String>) -> TokenStream,
) -> TokenStream {
    match structure {
        Structure::Struct(fields) => {
            let field_assignments =
                generate_struct_field_accessors(fields, self_access);
            let mapping = fields_mapping(fields, None);
            quote! {{
                #field_assignments
                #mapping
            }}
        }
        Structure::Enum(variants) => {
            map_enum_variants(variants, self_access, |variant| {
                fields_mapping(&variant.fields, Some(&variant.name))
            })
        }
    }
}

pub fn generate_from_parts_impl(
    structure: &Structure,
    fields_mapper: impl Fn(&Fields, Option<&syn::Ident>) -> TokenStream,
) -> TokenStream {
    match &structure {
        Structure::Struct(fields) => fields_mapper(fields, None),
        Structure::Enum(variants) => {
            generate_enum_match_from_parts(variants, fields_mapper)
        }
    }
}

/// Generates a match expression for the given enum type, mapping each variant to a corresponding match arm body.
/// This provides the identifiers of the fields in the match arm, so that they can be used in the body of the match arm.
pub fn map_enum_variants(
    enum_ty: &[EnumVariant],
    self_access: SelfAccess,
    generate_match_arm_body: impl Fn(&EnumVariant) -> TokenStream,
) -> TokenStream {
    let self_type = match &self_access {
        SelfAccess::BorrowedIdent { self_type, .. } => quote! { #self_type },
        _ => quote! { Self },
    };

    let arms = enum_ty.iter().map(|variant| {
        let variant_ident = Ident::new(&variant.name, Span::call_site());
        let match_arm_body = generate_match_arm_body(variant);
        let field_idents = variant.fields.normalized_field_idents();
        match &variant.fields {
            Fields::Named(_fields) => {
                quote! {
                    #self_type::#variant_ident { #(#field_idents),* } => {
                        #match_arm_body
                    }
                }
            }
            Fields::Unnamed(_fields) => {
                quote! {
                    #self_type::#variant_ident(#(#field_idents),*) => {
                        #match_arm_body
                    }
                }
            }
            Fields::Transparent(_field) => {
                let first_field_ident = field_idents.first().unwrap();
                quote! {
                    #self_type::#variant_ident(#first_field_ident) => {
                        #match_arm_body
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    #self_type::#variant_ident => {
                        #match_arm_body
                    }
                }
            }
        }
    });

    let self_access_pattern = match self_access {
        SelfAccess::Borrowed | SelfAccess::Moved | SelfAccess::BorrowedMut => {
            quote! { self }
        }
        SelfAccess::BorrowedIdent { self_value, .. } => quote! { #self_value },
        SelfAccess::Boxed => quote! { *self },
    };

    quote! {
        match #self_access_pattern {
            #(#arms),*
        }
    }
}

/// Generates the field accessors for a struct, creating let bindings for each field.
pub fn generate_struct_field_accessors(
    fields: &Fields,
    self_access: SelfAccess,
) -> TokenStream {
    let field_assignments = fields
        .field_accessors()
        .iter()
        .zip(fields.normalized_field_idents().iter())
        .map(|(accessor, normalized_ident)| match &self_access {
            SelfAccess::Borrowed => {
                quote! {
                    let #normalized_ident = &self.#accessor;
                }
            }
            SelfAccess::BorrowedMut => {
                quote! {
                    let #normalized_ident = &mut self.#accessor;
                }
            }
            SelfAccess::BorrowedIdent { self_value, .. } => {
                quote! {
                    let #normalized_ident = &#self_value.#accessor;
                }
            }
            SelfAccess::Moved | SelfAccess::Boxed => {
                quote! {
                    let #normalized_ident = self.#accessor;
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#field_assignments)*
    }
}

pub fn generate_enum_match_from_parts(
    variants: &[EnumVariant],
    field_generator: impl Fn(&Fields, Option<&syn::Ident>) -> TokenStream,
) -> TokenStream {
    let variant = variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.name;
            let fields =
                &field_generator(&variant.fields, Some(&variant.ident()));
            quote! {
                Some(#variant_ident) => {
                    #fields
                }
            }
        })
        .collect::<Vec<_>>();
    quote! {{
        match tag {
            #(#variant,)*
            _ => Err(()),
        }
    }}
}

/// Generates a TokenStream that creates a SelfOwnedPointerAddress for the given structure data.
pub fn get_pointer_address(
    structure_data: &StructureData,
) -> TokenStream {
    // FIXME: calculate pointer address statically in macro at compile time
    let unique_name = format!(
        "{}::{}",
        structure_data.namespace.join("::"),
        structure_data.ident
    );
    quote! {
        unsafe {
            SelfOwnedPointerAddress::new_static_from_name(
                #unique_name
            )
        }
    }
}
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    datex_proxy::data::{
        EnumVariant, Field, FieldMapping, Fields, NamedField, Structure,
        StructureData, TypeKind,
    },
    utils::{
        get_datex_core_crate_name_with_options, get_project_relative_file_path,
    },
};
use crate::datex_proxy::generator::helpers::get_pointer_address;

pub fn generate_core_lib_type_id(
    structure_data: &StructureData,
) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics GetCoreLibTypeId for #ident #ty_generics #where_clause {}
    }
}

/// Generates the [GetDatexType] implementation for the given structure data.
/// Returns a TokenStream of the implementation.
pub fn generate_datex_type(structure_data: &StructureData) -> TokenStream {
    let datex_type = generate_type(structure_data);
    let StructureData {
        ident,
        generics,
        attributes,
        ..
    } = structure_data;
    let datex_name = structure_data
        .attributes
        .datex_name
        .clone()
        .unwrap_or(structure_data.ident.to_string());

    let registration = generate_type_registration(structure_data);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    if matches!(attributes.type_kind, TypeKind::Structural { .. }) {
        quote! {
            #[automatically_derived]
            impl #impl_generics GetDatexType for #ident #ty_generics #where_clause {
                fn datex_type(cache: &mut SharedReferencesCache) -> Type {
                    cache.resolve_structural_type::<Self, _>(
                        |cache| (#datex_type).with_name(#datex_name)
                    )
                }
            }

            #registration
        }
    } else {
        quote! {
            #[automatically_derived]
            impl #impl_generics GetDatexType for #ident #ty_generics #where_clause {
                fn datex_type(cache: &mut SharedReferencesCache) -> Type {
                    #datex_type
                }
            }

            #registration
        }
    }
}

fn generate_type_registration(structure_data: &StructureData) -> TokenStream {
    let datex_core_crate_name =
        get_datex_core_crate_name_with_options(structure_data.attributes.force_datex_core_namespace);

    let StructureData {
        ident,
        generics: _,
        attributes,
        ..
    } = structure_data;

    let datex_name = attributes.datex_name.clone().unwrap_or(ident.to_string());

    let docs = match &attributes.docs {
        Some(docs) => quote! {
            Some(#docs)
        },
        None => quote! {
            None
        },
    };

    let namespace = {
        let mut ns = get_project_relative_file_path();
        ns.set_extension("");
        ns.to_str()
            .expect("Failed to convert file path to string")
            .to_string()
    };

    quote! {
        #datex_core_crate_name::inventory::submit! {
            #datex_core_crate_name::datex_registry::DatexTypeRegistration::new_with_cache::<#ident>(
                #datex_core_crate_name::datex_registry::DatexTypeMetadata {
                    name: #datex_name,
                    rust_ident: stringify!(#ident),
                    docs: #docs,
                    export: true, // FIXME
                    namespace: #namespace,
                }
            )
        }
    }
}

/// Generates a type definition for fields. Returns a TokenStream of [TypeDefinition].
fn generate_datex_type_definition(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote! {
            TypeDefinition::UNIT
        },
        Fields::Named(fields) => {
            let field_types = fields
                .iter()
                .map(named_field_to_definition)
                .collect::<Vec<_>>();
            quote! {
                TypeDefinition::Map(MapTypeDefinition(vec![
                    #(#field_types),*
                ]))
            }
        }
        Fields::Unnamed(field) => {
            let field_types = field
                .iter()
                .map(|f| field_to_definition(&f.field))
                .collect::<Vec<_>>();

            quote! {
                TypeDefinition::List(ListTypeDefinition(vec![
                    #(#field_types),*
                ]))
            }
        }
        Fields::Transparent(field) => {
            let first_field = field_to_definition(&field.field);
            quote! {
                #first_field.convert_to_definition()
            }
        }
    }
}

/// Generates a type definition for a single field. Returns a TokenStream of [TypeDefinition].
fn field_to_definition(field: &Field) -> TokenStream {
    let field_type = &field.ty;
    match &field.attributes.field_mapping {
        FieldMapping::Datex => {
            quote! {
                <#field_type as GetDatexType>::datex_type(cache.into())
            }
        }
        // Cannot infer type for serde fields
        FieldMapping::Serde => {
            quote! {
               Type::Definition(TypeDefinition::CoreType(CoreLibTypeId::Base(CoreLibBaseTypeId::Any)).into())
            }
        }
    }
}

/// Generates a type definition for a named field. Returns a TokenStream with a tuple of name and [TypeDefinition].
fn named_field_to_definition(field: &NamedField) -> TokenStream {
    let field_definition = field_to_definition(&field.field);
    let name = field.datex_field_name().to_string();
    quote! {
        (
            Type::Definition(TypeDefinition::Literal(LiteralTypeDefinition::Text(#name.into())).into()),
            #field_definition
        )
    }
}

/// Generates a type definition for an enum. Returns a TokenStream of [TypeDefinition].
fn generate_datex_enum_type(enum_ty: &[EnumVariant]) -> TokenStream {
    let variants_datex_types = enum_ty.iter().map(|variant| {
        let name = &variant.name;
        let type_definition = match variant.fields {
            Fields::Unit => quote! {
                None
            },
            _ => {
                let def = Some(generate_datex_type_definition(&variant.fields));
                quote! {
                    Some(Box::new(Type::Definition(#def.into())))
                }
            }
        };
        quote! {
            Type::Definition(TypeDefinition::TaggedType(TaggedTypeDefinition {
                tag: #name.to_string(),
                ty: #type_definition,
            }).into())
        }
    });

    quote! {
        TypeDefinition::Union(UnionTypeDefinition(vec![
            #(#variants_datex_types),*
        ]))
    }
}

/// Wraps a type definition. Returns a TokenStream of [Type].
fn generate_type(structure_data: &StructureData) -> TokenStream {
    let type_definition = match &structure_data.structure {
        Structure::Enum(enum_val) => generate_datex_enum_type(enum_val),
        Structure::Struct(fields) => generate_datex_type_definition(fields),
    };
    if let TypeKind::Structural { .. } = structure_data.attributes.type_kind {
        quote! {
            Type::Definition(
                #type_definition.into()
            )
        }
    } else {
        let ptr_address_init = get_pointer_address(structure_data);
        
        let name = structure_data
            .attributes
            .datex_name
            .clone()
            .unwrap_or(structure_data.ident.to_string());

        quote! {
            Type::Entity(unsafe {
                cache.init_entity_type::<Self>(#ptr_address_init, |cache| (#name.to_string(), #type_definition))
            })
        }
    }
}

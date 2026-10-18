use crate::datex_proxy::{
    data::{EnumVariant, Fields, Structure, StructureData},
    generator::helpers::{SelfAccess, map_enum_variants},
};
use proc_macro2::TokenStream;
use quote::quote;
use crate::datex_proxy::generator::helpers::get_pointer_address;

/// Generates the [Classification] implementations
pub fn generate_classification(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let _has_classification =
        !structure_data.attributes.type_kind.is_structural();

    let classification_methods =
        generate_classification_methods(structure_data);

    quote! {
        #[automatically_derived]
        impl #impl_generics Classification for #ident #ty_generics #where_clause {
            #classification_methods
        }
    }
}

/// Generates the methods for the [Classification] implementation
/// Adds an entity_type method if the type is an entity, and a tag method if the type is a tagged enum
fn generate_classification_methods(
    structure_data: &StructureData,
) -> TokenStream {
    let tag_method = match &structure_data.structure {
        Structure::Enum(variants) => {
            let enum_tags = generate_enum_tags(variants, SelfAccess::Borrowed);
            Some(quote! {
                 fn tag(&self) -> Option<ValueTag> {
                    #enum_tags
                }
            })
        }
        Structure::Struct(_) => None,
    };
    
    // If the type is an entity, generate the entity_type and entity_type_address methods
    let pointer_address_init = if structure_data.attributes.type_kind.is_entity() {
        let pointer_address_init = get_pointer_address(structure_data);
        quote! { Some(PointerAddress::SelfOwned(#pointer_address_init))}
    }
    else {
        quote! { None }
    };

    let entity_type_init = if structure_data.attributes.type_kind.is_entity() {
        quote! {
            let ty = Self::datex_type(cache);
            match ty {
                Type::Entity(entity_type) => Some(entity_type),
                _ => unreachable!(),
            }
        }
    } else {
        quote! { None }
    };

    let entity_type_method = if structure_data.attributes.type_kind.is_entity()
    {
        Some(quote! {
            fn entity_type(
                &self,
                cache: &mut SharedReferencesCache,
            ) -> Option<EntityType> {
                #entity_type_init
            }

            fn entity_type_address(&self) -> Option<PointerAddress> {
                #pointer_address_init
            }
        })
    } else {
        None
    };

    quote! {
        #entity_type_method
        #tag_method
    }
}

fn generate_enum_tags(
    variants: &[EnumVariant],
    self_access: SelfAccess,
) -> TokenStream {
    map_enum_variants(variants, self_access, |variant| {
        let variant_name = &variant.name;
        let is_empty = matches!(variant.fields, Fields::Unit);
        quote! {
            Some(
                ValueTag {
                    tag: #variant_name.to_string(),
                    is_empty: #is_empty,
                }
            )
        }
    })
}

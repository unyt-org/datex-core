use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::{
    datex_proxy::data::{FieldIdent, Fields, Structure, StructureData},
    generator::helpers::{
        SelfAccess, generate_struct_or_enum_variants_fields_mapping,
    },
};

/// Generates the [IterParts] implementations
pub fn generate_child_iterator(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let iterator_methods =
        generate_child_property_methods(&structure_data.structure);

    quote! {
        #[automatically_derived]
        impl #impl_generics IterParts for #ident #ty_generics #where_clause {
            #iterator_methods
        }
    }
}

fn generate_child_property_methods(structure: &Structure) -> TokenStream {
    let iter_list_parts = generate_struct_or_enum_variants_fields_mapping(
        structure,
        SelfAccess::Borrowed,
        |fields, _| generate_iter_list_parts(fields, false),
    );
    let iter_list_parts_mut = generate_struct_or_enum_variants_fields_mapping(
        structure,
        SelfAccess::BorrowedMut,
        |fields, _| generate_iter_list_parts(fields, true),
    );
    let iter_map_parts = generate_struct_or_enum_variants_fields_mapping(
        structure,
        SelfAccess::Borrowed,
        |fields, _| generate_iter_map_parts(fields, false),
    );
    let iter_map_parts_mut = generate_struct_or_enum_variants_fields_mapping(
        structure,
        SelfAccess::BorrowedMut,
        |fields, _| generate_iter_map_parts(fields, true),
    );
    quote! {
        fn iter_list_parts<'a>(
            &'a self,
        ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainer<'a>> + 'a>> {
            #iter_list_parts
        }

        fn iter_list_parts_mut<'a>(
            &'a mut self,
        ) -> Option<Box<dyn Iterator<Item = BorrowedValueContainerMut<'a>> + 'a>>
        {
            #iter_list_parts_mut
        }

        fn iter_map_parts<'a>(
            &'a self,
        ) -> Option<
            Box<
                dyn Iterator<
                        Item = (
                            BorrowedValueContainer<'a>,
                            BorrowedValueContainer<'a>,
                        ),
                    > + 'a,
            >,
        > {
            #iter_map_parts
        }

        fn iter_map_parts_mut<'a>(
            &'a mut self,
        ) -> Option<
            Box<
                dyn Iterator<
                        Item = (
                            BorrowedValueContainer<'a>,
                            BorrowedValueContainerMut<'a>,
                        ),
                    > + 'a,
            >,
        > {
            #iter_map_parts_mut
        }
    }
}

fn generate_iter_list_parts(fields: &Fields, is_mut: bool) -> TokenStream {
    let iter_list_parts = if is_mut {
        Ident::new("iter_list_parts_mut", Span::call_site())
    } else {
        Ident::new("iter_list_parts", Span::call_site())
    };
    let as_borrowed_value_container_with_mutability = borrowed_name(is_mut);
    match fields {
        Fields::Unnamed(unnnamed) => {
            let fields = unnnamed
                .iter()
                .map(|field| {
                    let ident = field.normalized_ident();
                    quote! {
                        yield #ident.#as_borrowed_value_container_with_mutability();
                    }
                })
                .collect::<Vec<_>>();
            quote! {
                Some(Box::new(gen move {
                    #(#fields)*
                }))
            }
        }
        Fields::Transparent(inner) => {
            let inner = inner.normalized_ident();
            quote! {
                #inner.#iter_list_parts()
            }
        }
        Fields::Unit | Fields::Named(_) => {
            quote! {
                None
            }
        }
    }
}

fn borrowed_name(is_mut: bool) -> Ident {
    if is_mut {
        Ident::new("as_borrowed_value_container_mut", Span::call_site())
    } else {
        Ident::new("as_borrowed_value_container", Span::call_site())
    }
}

fn generate_iter_map_parts(fields: &Fields, is_mut: bool) -> TokenStream {
    let iter_map_parts = if is_mut {
        Ident::new("iter_map_parts_mut", Span::call_site())
    } else {
        Ident::new("iter_map_parts", Span::call_site())
    };

    let as_borrowed_value_container_with_mutability = borrowed_name(is_mut);
    match fields {
        Fields::Named(named) => {
            let fields_with_names = named
                .iter()
                // FIXME: skips serde fields for now (cannot be borrowed as value containers)
                .filter(|field| field.field.attributes.field_mapping.is_datex())
                .map(|field| {
                    let name = field.datex_field_name();
                    let ident = field.normalized_ident();
                    quote! {
                        yield (
                            BorrowedValueContainer::from(#name),
                            #ident.#as_borrowed_value_container_with_mutability()
                        );
                    }
                })
                .collect::<Vec<_>>();
            quote! {
                Some(Box::new(gen move {
                    #(#fields_with_names)*
                }))
            }
        }
        Fields::Transparent(inner) => {
            let inner = inner.normalized_ident();
            quote! {
                #inner.#iter_map_parts()
            }
        }
        Fields::Unit | Fields::Unnamed(_) => {
            quote! {
                None
            }
        }
    }
}

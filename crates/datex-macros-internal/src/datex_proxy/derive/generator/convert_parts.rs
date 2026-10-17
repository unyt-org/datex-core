use crate::{
    datex_proxy::{
        data::{FieldIdent, FieldMapping, Fields, StructureData},
        generator::helpers::{
            SelfAccess, generate_struct_or_enum_variants_fields_mapping,
        },
    },
    generator::helpers::generate_from_parts_impl,
};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generates the [FromParts] and [IntoParts] implementations
pub fn generate_convert_parts(structure_data: &StructureData) -> TokenStream {
    let StructureData { .. } = structure_data;

    let into_parts_impl = generate_into_parts(structure_data);
    let from_parts_impl = generate_from_parts(structure_data);

    quote! {
        #into_parts_impl
        #from_parts_impl
    }
}

fn generate_into_parts(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let parts_kind = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Borrowed,
        get_fields_parts_kind,
    );

    let into_map_parts_impl = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Boxed,
        generate_into_map_parts_for_fields,
    );

    let into_list_parts_impl = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Boxed,
        generate_into_list_parts_for_fields,
    );

    let into_single_value_impl =
        generate_struct_or_enum_variants_fields_mapping(
            &structure_data.structure,
            SelfAccess::Boxed,
            generate_into_single_value_for_fields,
        );

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics HasPartsKind for #ident #ty_generics #where_clause {
            fn parts_kind(&self) -> PartsKind {
                #parts_kind
            }
        }

        #[automatically_derived]
        impl #impl_generics IntoParts for #ident #ty_generics #where_clause {

            fn try_into_map_parts<'a>(
                self: Box<Self>,
            ) -> Result<Map, ()>
            where
                Self: 'a,
            {
                #into_map_parts_impl
            }

            fn try_into_list_parts<'a>(
                self: Box<Self>,
            ) -> Result<List, ()>
            where
                Self: 'a,
            {
                #into_list_parts_impl
            }

            fn try_into_single_value<'a>(
                self: Box<Self>,
            ) -> Result<ValueContainer, ()>
            where
                Self: 'a,
            {
                #into_single_value_impl
            }
        }
    }
}

/// Determines the [PartsKind] based on the fields of the structure
fn get_fields_parts_kind(
    fields: &Fields,
    _tag: Option<&String>,
) -> TokenStream {
    match fields {
        Fields::Named(_) => quote! { PartsKind::Map },
        Fields::Unnamed(_) => quote! { PartsKind::List },
        Fields::Unit => quote! { PartsKind::SingleValue }, // unit structs can be represented as a single value (null)
        Fields::Transparent(field) => {
            let accessor = field.normalized_ident();
            quote! { #accessor.parts_kind() } // retrieve the parts kind of the single field
        }
    }
}

fn generate_into_map_parts_for_fields(
    fields: &Fields,
    _tag: Option<&String>,
) -> TokenStream {
    match fields {
        Fields::Named(fields) => {
            let yields = fields.iter().map(|field| {
                let field_name = &field.name;
                let field_value_container = value_container_from_field(
                    field.normalized_ident(),
                    &field.field.attributes.field_mapping,
                );
                quote! {
                    yield (
                        ValueContainer::from(#field_name),
                        #field_value_container,
                    );
                }
            });
            quote! {
                // gen block that yields (ValueContainer, ValueContainer) pairs for each field in the struct
                let map = gen move {
                    #(#yields)*
                };
                Ok(map.collect::<Map>())
            }
        }
        Fields::Unit | Fields::Unnamed(_) => quote! { Err(()) }, // cannot be converted into map parts
        Fields::Transparent(field) => {
            let accessor = field.normalized_ident();
            quote! { Box::new(#accessor).try_into_map_parts() } // delegate to the single field's implementation
        }
    }
}

fn generate_into_list_parts_for_fields(
    fields: &Fields,
    _tag: Option<&String>,
) -> TokenStream {
    match fields {
        Fields::Unnamed(fields) => {
            let yields = fields.iter().map(|field| {
                let field_value_container = value_container_from_field(
                    field.normalized_ident(),
                    &field.field.attributes.field_mapping,
                );
                quote! {
                    yield #field_value_container;
                }
            });
            quote! {
                // gen block that yields ValueContainer for each field in the tuple struct
                let list = gen move {
                    #(#yields)*
                };
                Ok(list.collect::<List>())
            }
        }
        Fields::Unit | Fields::Named(_) => quote! { Err(()) }, // cannot be converted into list parts
        Fields::Transparent(field) => {
            let accessor = field.normalized_ident();
            quote! { Box::new(#accessor).try_into_list_parts() } // delegate to the single field's implementation
        }
    }
}

fn generate_into_single_value_for_fields(
    fields: &Fields,
    _tag: Option<&String>,
) -> TokenStream {
    match fields {
        Fields::Transparent(field) => {
            let accessor = field.normalized_ident();
            quote! { Box::new(#accessor).try_into_single_value() } // delegate to the single field's implementation
        }
        Fields::Unit => quote! { Ok(ValueContainer::Local(Value::null())) }, // unit structs can be represented as null
        Fields::Named(_) | Fields::Unnamed(_) => quote! { Err(()) }, // only transparent structs can be converted into a single value
    }
}

/// Generates the appropriate ValueContainer conversion based on the field's mapping
/// For fields with serde mapping, it uses `serde_to_value_container`, otherwise it uses `ValueContainer::from`.
fn value_container_from_field(
    accessor: Ident,
    field_mapping: &FieldMapping,
) -> TokenStream {
    if field_mapping.is_serde() {
        quote! { serde_to_value_container(#accessor) }
    } else {
        quote! { ValueContainer::from(#accessor) }
    }
}

/// Generates the appropriate ValueContainer conversion from a ValueContainer based on the field's mapping
/// For fields with serde mapping, it uses `try_serde_from_value_container`, otherwise it uses `try_into_value`.
fn field_from_value_container(
    accessor_result: Ident,
    field_mapping: &FieldMapping,
    unwrap_default: bool,
) -> TokenStream {
    let access = if field_mapping.is_serde() {
        quote! {{
            match #accessor_result {
                Ok(v) => Ok(try_serde_from_value_container(v).map_err(|_| ())?),
                Err(e) => Err(e),
            }
        }}
    } else {
        quote! {{
            match #accessor_result {
                Ok(v) => Ok(v.try_cast_into_value().map_err(|_| ())?),
                Err(e) => Err(e),
            }
        }}
    };

    if unwrap_default {
        quote! { #access.unwrap_or_default() }
    } else {
        quote! { #access.map_err(|_| ())? }
    }
}

fn generate_from_parts(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident,
        generics,
        attributes,
        ..
    } = structure_data;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let from_map_parts_impl = if attributes.no_deserialize {
        quote! { Err(()) }
    } else {
        generate_from_parts_impl(
            &structure_data.structure,
            generate_from_map_parts_for_fields,
        )
    };

    let from_list_parts_impl = if attributes.no_deserialize {
        quote! { Err(()) }
    } else {
        generate_from_parts_impl(
            &structure_data.structure,
            generate_from_list_parts_for_fields,
        )
    };

    let from_single_value_impl = if attributes.no_deserialize {
        quote! { Err(()) }
    } else {
        generate_from_parts_impl(
            &structure_data.structure,
            generate_from_single_value_for_fields,
        )
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics FromParts for #ident #ty_generics #where_clause {
            fn try_from_map_parts_with_tag(mut parts: Map, tag: Option<&str>) -> Result<Self, ()>
            where
                Self: Sized,
            {
                #from_map_parts_impl
            }

            fn try_from_list_parts_with_tag(mut parts: List, tag: Option<&str>) -> Result<Self, ()>
            where
                Self: Sized,
            {
                #from_list_parts_impl
            }

            fn try_from_single_value_with_tag(value: ValueContainer, tag: Option<&str>) -> Result<Self, ()>
            where
                Self: Sized,
            {
                #from_single_value_impl
            }
        }
    }
}

fn generate_from_map_parts_for_fields(
    fields: &Fields,
    variant_ident: Option<&syn::Ident>,
) -> TokenStream {
    let variant_ident = variant_ident
        .map(|ident| quote! { Self::#ident })
        .unwrap_or(quote! { Self });
    match fields {
        Fields::Named(fields) => {
            let field_conversions = fields.iter().map(|field| {
                let field_name = &field.name;
                let field_ident = field.ident_accessor();
                let accessor = field.normalized_ident();

                let from_value_container = field_from_value_container(
                    accessor.clone(),
                    &field.field.attributes.field_mapping,
                    field.attributes.skip_with_default,
                );

                quote! {
                    #field_ident: {
                        /// SAFETY: parts is not used afterward
                        let #accessor = unsafe { parts.try_delete_unchecked(&ValueContainer::from(#field_name)) };
                        println!("try to get field: {:?} from parts: {:?}: {:?}", ValueContainer::from(#field_name), parts, #accessor);

                        #from_value_container
                    },
                }
            });
            quote! {
                Ok(#variant_ident {
                    #(#field_conversions)*
                })
            }
        }
        Fields::Unit | Fields::Unnamed(_) => quote! { Err(()) }, // unit structs cannot be converted from map parts
        Fields::Transparent(field) => {
            let ty = &field.field.ty;
            quote! {
                <#ty>::try_from_map_parts_with_tag(parts, tag).map(#variant_ident)
            } // delegate to the single field's implementation
        }
    }
}

fn generate_from_list_parts_for_fields(
    fields: &Fields,
    variant_ident: Option<&syn::Ident>,
) -> TokenStream {
    let variant_ident = variant_ident
        .map(|ident| quote! { Self::#ident })
        .unwrap_or(quote! { Self });
    match fields {
        Fields::Unnamed(fields) => {
            let (mut field_pops, field_idents) = fields
                .iter()
                .map(|field| {
                    let ident = field.normalized_ident();
                    let from_value_container = field_from_value_container(
                        ident.clone(),
                        &field.field.attributes.field_mapping,
                        false, // TODO
                    );
                    (
                        quote! {
                            let #ident = parts.pop().ok_or(());
                        },
                        from_value_container,
                    )
                })
                .collect::<(Vec<_>, Vec<_>)>();
            field_pops.reverse();
            quote! {
                #(#field_pops)*
                Ok(#variant_ident(
                    #(#field_idents),*
                ))
            }
        }
        Fields::Unit | Fields::Named(_) => quote! { Err(()) }, // unit structs cannot be converted from list parts
        Fields::Transparent(field) => {
            let ty = &field.field.ty;
            quote! {
                <#ty>::try_from_list_parts_with_tag(parts, tag).map(#variant_ident)
            } // delegate to the single field's implementation
        }
    }
}

fn generate_from_single_value_for_fields(
    fields: &Fields,
    variant_ident: Option<&syn::Ident>,
) -> TokenStream {
    let variant_ident = variant_ident
        .map(|ident| quote! { Self::#ident })
        .unwrap_or(quote! { Self });
    match fields {
        Fields::Transparent(field) => {
            let ty = &field.field.ty;
            quote! {
                <#ty>::try_from_single_value_with_tag(value, tag).map(#variant_ident)
            } // delegate to the single field's implementation
        }
        Fields::Unit => {
            // if value is null, we can construct the unit struct, otherwise we cannot
            quote! {
                if value.is_null() {
                    Ok(#variant_ident)
                } else {
                    Err(())
                }
            }
        }
        Fields::Named(_) | Fields::Unnamed(_) => quote! { Err(()) }, // only transparent structs can be converted from a single value
    }
}

use crate::datex_proxy::{
    data::{FieldIdent, FieldMapping, Fields, NamedField, StructureData},
    generator::helpers::{
        SelfAccess, generate_struct_or_enum_variants_fields_mapping,
    },
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

/// Creates the implementation of the [ToDatexExpressionData] trait for the given structure data.
/// Returns a TokenStream of the implementation.
pub fn generate_datex_expression_data(
    structure_data: &StructureData,
) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let datex_expression_data = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Borrowed,
        generate_datex_expression_data_fields,
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ToDatexExpressionData for #ident #ty_generics #where_clause {
            fn to_datex_expression_data(&self) -> DatexExpressionData {
                #datex_expression_data
            }
        }
    }
}

/// Generates the datex expression data for the given fields. Returns a TokenStream of [DatexExpressionData].
fn generate_datex_expression_data_fields(
    fields: &Fields,
    tag: Option<&String>,
) -> TokenStream {
    let inner = match fields {
        Fields::Unit => None,
        Fields::Named(fields) => {
            let field_expressions = fields
                .iter()
                .map(named_field_to_expression_data)
                .collect::<Vec<_>>();
            Some(quote! {
                DatexExpressionData::Map(ast::expressions::Map::new(
                    vec![
                        #(#field_expressions),*
                    ]
                ))
            })
        }
        Fields::Unnamed(field) => {
            let field_expressions = field
                .iter()
                .map(|field| {
                    field_to_expression_data(
                        field.normalized_ident().into_token_stream(),
                        &field.field.attributes.field_mapping,
                    )
                })
                .collect::<Vec<_>>();

            Some(quote! {
                DatexExpressionData::List(ast::expressions::List::new(
                    vec![
                        #(#field_expressions),*
                    ]
                ))
            })
        }
        Fields::Transparent(field) => {
            let first_field = field_to_expression_data(
                field.normalized_ident().into_token_stream(),
                &field.field.attributes.field_mapping,
            );
            Some(quote! {
                {
                    *(#first_field.data)
                }
            })
        }
    };

    match tag {
        Some(tag) => {
            let expression = match inner {
                Some(inner) => quote! {
                    Some(#inner.with_default_span())
                },
                None => quote! { None },
            };

            quote! {
                DatexExpressionData::Tag(TagExpression {
                    tag: #tag.to_string(),
                    expression: #expression
                })
            }
        },
        None => inner.unwrap_or(quote! {
            DatexExpressionData::Statements(Statements::empty())
        }),
    }
}

/// Generates a type definition for a single field. Returns a TokenStream of [TypeDefinition].
fn field_to_expression_data(
    accessor: TokenStream,
    field_mapping: &FieldMapping,
) -> TokenStream {
    match field_mapping {
        FieldMapping::Datex => {
            quote! {
                #accessor.to_datex_expression_data().with_default_span()
            }
        }
        FieldMapping::Serde => {
            quote! {
                serde_to_value_container(#accessor).to_datex_expression_data().with_default_span()
            }
        }
    }
}

/// Generates a type definition for a named field. Returns a TokenStream with a tuple of name and [TypeDefinition].
fn named_field_to_expression_data(field: &NamedField) -> TokenStream {
    let id = field.normalized_ident().into_token_stream();
    let expression_data =
        field_to_expression_data(id, &field.field.attributes.field_mapping);
    let name = field.datex_field_name().to_string();
    quote! {
        (
            DatexExpressionData::Text(Text::String(#name.to_string())).with_default_span(),
            #expression_data,
        )
    }
}

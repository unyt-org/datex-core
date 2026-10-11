use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::datex_proxy::{
    data::{
        FieldIdent, FieldMapping, Fields, IndexedField, NamedField,
        StructureData,
    },
    generator::helpers::{
        SelfAccess, generate_struct_or_enum_variants_fields_mapping,
    },
};

/// Generates the implementation of the [ToInstructions] trait for the given structure data.
/// Returns a [TokenStream] containing the generated implementation.
pub fn generate_to_instructions(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Borrowed,
        generate_to_instructions_for_fields,
    );

    quote! {
        impl #impl_generics ToInstructions for #ident #ty_generics #where_clause {
            fn to_instructions<'ctx, 'a>(
                &'a self,
                ctx: &'a mut dyn ValueVisitor<'ctx>,
            ) -> Box<dyn Iterator<Item = Instruction> + 'a>
            where
                'ctx: 'a,
            {
                Box::new(gen move {
                    #body
                })
            }
        }
    }
}

/// Generates the instructions for the given fields, handling different field types.
fn generate_to_instructions_for_fields(
    fields: &Fields,
    tag: Option<&String>,
) -> TokenStream {
    let tag = if let Some(tag) = tag {
        let is_empty = matches!(fields, Fields::Unit);
        quote! {
            yield RegularInstruction::tagged_value(#tag.to_string(), #is_empty).into();
        }
    } else {
        quote! {}
    };

    let fields = match fields {
        Fields::Unit => {
            quote! {}
        }
        Fields::Named(fields) => generate_named_fields_to_instructions(fields),
        Fields::Unnamed(fields) => {
            generate_unnamed_fields_to_instructions(fields)
        }
        Fields::Transparent(field) => {
            generate_transparent_field_to_instructions(field)
        }
    };

    quote! {{
        #tag
        #fields
    }}
}

/// Generates the instructions for named fields within a struct.
fn generate_named_fields_to_instructions(fields: &[NamedField]) -> TokenStream {
    let field_instructions = fields.iter().map(|field| {
        let accessor = field.ident_accessor().to_token_stream();
        let name = field.name.to_string();
        let mapping = &field.field.attributes.field_mapping;
        let value_instructions = field_to_instructions(accessor, mapping);

        quote! {
            yield RegularInstruction::key_value_short_text(#name.to_string()).into();
            #value_instructions
        }
    });

    let field_count = fields.len() as u32;
    quote! {
        yield RegularInstruction::map(#field_count).into();
        #(
            #field_instructions
        )*
    }
}

/// Generates the instructions for unnamed fields within a struct.
fn generate_unnamed_fields_to_instructions(
    fields: &[IndexedField],
) -> TokenStream {
    let field_instructions = fields.iter().map(|field| {
        let accessor = field.normalized_ident().to_token_stream();
        let mapping = &field.field.attributes.field_mapping;
        field_to_instructions(accessor, mapping)
    });

    let field_count = fields.len() as u32;
    quote! {
        yield RegularInstruction::list(#field_count).into();
        #(
            #field_instructions
        )*
    }
}

/// Generates the instructions for a transparent field within a struct.
fn generate_transparent_field_to_instructions(
    field: &IndexedField,
) -> TokenStream {
    let accessor = field.normalized_ident().to_token_stream();
    let mapping = &field.field.attributes.field_mapping;
    field_to_instructions(accessor, mapping)
}

/// Generates the instructions for a single field based on its mapping.
fn field_to_instructions(
    accessor: TokenStream,
    field_mapping: &FieldMapping,
) -> TokenStream {
    match field_mapping {
        FieldMapping::Datex => {
            quote! {
                for i in (#accessor).to_instructions(ctx) {
                    yield i;
                }
            }
        }
        FieldMapping::Serde => {
            // we have to collect, as serde_to_value_container gives an owned container,
            // and this leads to lifetime issues if we don't collect the instructions first
            // but try to yield on the fly
            quote! {
                let value = serde_to_value_container(&(#accessor)).to_instructions(ctx).collect::<Vec<_>>();
                for i in value {
                    yield i;
                }
            }
        }
    }
}

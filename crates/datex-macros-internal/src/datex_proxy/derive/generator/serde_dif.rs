use crate::datex_proxy::{
    data::{
        AnyField, EnumVariant, FieldIdent, FieldMapping, FieldType, Fields,
        IndexedField, NamedField, Structure, StructureData,
    },
    generator::helpers::{
        SelfAccess, generate_enum_match_from_parts, generate_from_parts_impl,
        generate_struct_field_accessors,
        generate_struct_or_enum_variants_fields_mapping, map_enum_variants,
    },
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::parse_quote;

/// Creates the implementation of the [SerializeWithSerdeContext] and [DeserializeWithSerdeContext] trait for the given structure data.
/// Returns a TokenStream of the implementation.
pub fn generate_serde_dif(structure_data: &StructureData) -> TokenStream {
    let serialize_impl = generate_serde_serialize(structure_data);

    let deserialize_impl = if !structure_data.attributes.no_deserialize {
        generate_serde_deserialize(structure_data)
    } else {
        generate_unimplemented_serde_deserialize(structure_data)
    };

    quote! {
        use serde::{
            Deserializer, Serializer,
            de::{DeserializeSeed, MapAccess, SeqAccess, Error, Visitor},
            ser::{SerializeMap, SerializeSeq, SerializeTuple},
        };

        #serialize_impl
        #deserialize_impl
    }
}

fn generate_serde_serialize(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let serialize = generate_struct_or_enum_variants_fields_mapping(
        &structure_data.structure,
        SelfAccess::Borrowed,
        generate_dif_serialize_for_fields,
    );

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl SerializeWithSerdeContext for #ident #ty_generics #where_clause {
            fn serialize_with_ctx<S>(
                &self,
                ctx: &SerdeContext<'_>,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                #serialize
            }
        }
    }
}

fn generate_serde_deserialize(structure_data: &StructureData) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let (deserialize_impl, visitor_impl) = match &structure_data.structure {
        Structure::Struct(fields) => generate_dif_deserialize_for_struct(
            structure_data,
            fields,
            &Ident::new(
                &format!("{}DeserializeCollector", ident),
                Span::call_site(),
            ),
        ),
        Structure::Enum(variants) => {
            generate_dif_deserialize_for_enum(structure_data, variants)
        }
    };

    generate_serde_deserialize_skeleton(
        structure_data,
        deserialize_impl,
        visitor_impl,
    )
}

/// Generates a deserialization implementation that returns an error indicating that deserialization is not implemented for the given structure.
fn generate_unimplemented_serde_deserialize(
    struct_data: &StructureData,
) -> TokenStream {
    let deserialize_impl = quote! {
        Err(D::Error::custom("Deserialization is not implemented for this type"))
    };

    generate_serde_deserialize_skeleton(
        struct_data,
        deserialize_impl,
        quote! {},
    )
}

/// Generates the skeleton of the deserialization implementation and Visitor for a struct
fn generate_serde_deserialize_skeleton(
    structure_data: &StructureData,
    deserialize_impl: TokenStream,
    visitor_impl: TokenStream,
) -> TokenStream {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl<'de> DeserializeWithSerdeContext<'de> for #ident #ty_generics #where_clause {
            fn deserialize_with_ctx<D>(
                ctx: &SerdeContext<'_>,
                deserializer: D,
            ) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                #deserialize_impl
            }
        }

        #visitor_impl
    }
}

fn generate_dif_serialize_for_fields(
    fields: &Fields,
    tag: Option<&String>,
) -> TokenStream {
    match fields {
        Fields::Named(named_fields) => {
            let serialize_fields = named_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                let field_name = field.datex_field_name();

                let val_init = match field.field.attributes.field_mapping {
                    FieldMapping::Datex => quote! {},
                    FieldMapping::Serde => quote! {
                        let #field_ident = serde_to_value_container(#field_ident);
                    },
                };

                quote! {
                    #val_init
                    map.serialize_entry(
                        #field_name,
                        &ValueWithSerdeContext::new(&#field_ident, ctx),
                    )?;
                }
            });

            let len = named_fields.len();

            quote! {
                let mut map = serializer.serialize_map(Some(#len))?;
                #(#serialize_fields)*
                map.end()
            }
        }
        Fields::Unnamed(unnamed_fields) => {
            let serialize_fields = unnamed_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();

                let val_init = match field.field.attributes.field_mapping {
                    FieldMapping::Datex => quote! {},
                    FieldMapping::Serde => quote! {
                        let #field_ident = &serde_to_value_container(#field_ident);
                    }
                };

                quote! {
                    #val_init
                    seq.serialize_element(&ValueWithSerdeContext::new(
                        #field_ident,
                        ctx,
                    ))?;
                }
            });
            let len = unnamed_fields.len();
            quote! {
                let mut seq = serializer.serialize_seq(Some(#len))?;
                #(#serialize_fields)*
                seq.end()
            }
        }
        Fields::Unit => {
            quote! {
                todo!()
            }
        }
        Fields::Transparent(_) => {
            quote! {
                todo!()
            }
        }
    }
}

fn generate_dif_deserialize_for_enum(
    structure_data: &StructureData,
    variants: &[EnumVariant],
) -> (TokenStream, TokenStream) {
    let StructureData { ident, .. } = structure_data;

    let collector_ident = Ident::new(
        &format!("{}EnumDeserializeCollector", ident),
        Span::call_site(),
    );

    let parent_visitor = quote! {

        struct #collector_ident<'a, 'ctx> {
            pub __ctx: &'a SerdeContext<'ctx>,
        }

        impl<'a, 'ctx> #collector_ident<'a, 'ctx> {
            fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
                #collector_ident { __ctx: ctx }
            }
        }

        impl<'de, 'a, 'ctx> Visitor<'de> for #collector_ident<'a, 'ctx> {
            type Value = #ident;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "either an object with string keys or a sequence of [key, value] entries",
                )
            }

            fn visit_map<_MapAccess>(self, mut map: _MapAccess) -> Result<Self::Value, _MapAccess::Error>
             where
                _MapAccess: MapAccess<'de> {
                // expect "t" and "v" keys
                // tag is the value of "t"
                let tag: String = if let Some(key) = map.next_key::<String>()? {
                    if key == "t" {
                        map.next_value()?
                    } else {
                        return Err(_MapAccess::Error::custom(format!("Expected 't' key, found '{}'", key)));
                    }
                } else {
                    return Err(_MapAccess::Error::custom("Expected 't' key, found none"));
                };

                panic!("handle deserialize for tag: {}", tag);
            }
        }
    };

    let deserialize_impl = quote! {
        deserializer.deserialize_any(#collector_ident::new(ctx))
    };

    (deserialize_impl, parent_visitor)
}

/// Generates the deserialization implementation and Visitor for a struct.
fn generate_dif_deserialize_for_struct(
    structure_data: &StructureData,
    fields: &Fields,
    collector_ident: &Ident,
) -> (TokenStream, TokenStream) {
    let StructureData {
        ident, generics, ..
    } = structure_data;

    let (properties, property_defaults, property_collapses) =
        generate_property_mappings(fields);

    let collapse_init =
        fields.wrap_in_initializer(property_collapses.into_iter());

    let deserialize_impl = quote! {
        deserializer.deserialize_any(#collector_ident::new(ctx))
    };

    let visitor_methods = match fields {
        Fields::Named(named) => {
            generate_visitor_methods_for_named_fields(named)
        }
        Fields::Unnamed(unnamed) => {
            generate_visitor_methods_for_unnamed_fields(unnamed)
        }
        Fields::Transparent(_) => {
            // TODO:
            quote! {}
        }
        Fields::Unit => {
            // TODO:
            quote! {}
        }
    };

    let visitor_impl = quote! {

        struct #collector_ident<'a, 'ctx> {
            pub __ctx: &'a SerdeContext<'ctx>,
            #(#properties),*
        }

        impl<'a, 'ctx> #collector_ident<'a, 'ctx> {
            fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
                #collector_ident::from(ctx)
            }
            fn collapse(self) -> Result<#ident, &'static str> {
                Ok(#ident #collapse_init)
            }
        }

        impl<'a, 'ctx> From<&'a SerdeContext<'ctx>> for #collector_ident<'a, 'ctx> {
            fn from(ctx: &'a SerdeContext<'ctx>) -> Self {
                #collector_ident {
                    __ctx: ctx,
                    #(#property_defaults),*
                }
            }
        }

        impl<'a, 'ctx, T> From<&'a DeserializeSerdeContext<'a, 'ctx, T>> for #collector_ident<'a, 'ctx> {
            fn from(value: &'a DeserializeSerdeContext<'a, 'ctx, T>) -> Self {
                #collector_ident::from(value.ctx)
            }
        }

        impl<'a, 'ctx, T> From<&'a #collector_ident<'a, 'ctx>> for DeserializeSerdeContext<'a, 'ctx, T> {
            fn from(value: &'a #collector_ident<'a, 'ctx>) -> Self {
                DeserializeSerdeContext::new(value.__ctx)
            }
        }

        impl<'de, 'a, 'ctx> Visitor<'de> for #collector_ident<'a, 'ctx> {
            type Value = #ident;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "either an object with string keys or a sequence of [key, value] entries",
                )
            }

            #visitor_methods
        }
    };

    (deserialize_impl, visitor_impl)
}

fn generate_visitor_methods_for_named_fields(
    fields: &[NamedField],
) -> TokenStream {
    let map_key_deserializers = fields.iter().map(|field| {
        let field_name = field.datex_field_name();
        let normalized_ident = field.normalized_ident();
        let ty = field.ty();

        match field.field.attributes.field_mapping {
            FieldMapping::Datex => quote! {
                #field_name => {
                    let value: #ty = map.next_value_seed(DeserializeSerdeContext::<#ty>::new(self.__ctx))?;
                    self.#normalized_ident = Some(value);
                }
            },
            FieldMapping::Serde => quote! {
                #field_name => {
                    let value_container = map.next_value_seed(DeserializeSerdeContext::<ValueContainer>::new(self.__ctx))?;
                    let value = try_serde_from_value_container(value_container).map_err(|_err| _MapAccess::Error::custom(format!("Failed to deserialize field {}", #field_name)))?;
                    self.#normalized_ident = Some(value);
                }
            }
        }
    });

    quote! {
        fn visit_map<_MapAccess>(mut self, mut map: _MapAccess) -> Result<Self::Value, _MapAccess::Error>
        where
            _MapAccess: MapAccess<'de>,
        {
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    #(#map_key_deserializers),*
                    _ => {
                        // Ignore unknown keys
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
            }

            self.collapse().map_err(|err| _MapAccess::Error::missing_field(err))
        }

        fn visit_seq<_SeqAccess>(mut self, mut seq: _SeqAccess) -> Result<Self::Value, _SeqAccess::Error>
        where
            _SeqAccess: SeqAccess<'de>,
        {
            self.collapse().map_err(|err| _SeqAccess::Error::missing_field(err))
        }
    }
}

fn generate_visitor_methods_for_unnamed_fields(
    fields: &[IndexedField],
) -> TokenStream {
    quote! {}
}

fn generate_property_mappings(
    fields: &Fields,
) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
    fields
        .iter()
        .map(|field| {
            let field_ty = field.ty();
            let normalized_ident = field.normalized_ident();
            let field_name = field.original_name();

            let field_init = quote! {
                pub #normalized_ident: Option<#field_ty>
            };

            let field_default = quote! {
                #normalized_ident: None
            };

            let field_collapse = quote! {
                self.#normalized_ident.ok_or_else(|| #field_name)?
            };

            (field_init, field_default, field_collapse)
        })
        .collect()
}

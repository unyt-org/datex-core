use crate::datex_proxy::{
    data::{
        AnyField, EnumVariant, FieldIdent, FieldMapping, FieldType, Fields,
        IndexedField, NamedField, Structure, StructureData,
    },
    generator::helpers::{
        SelfAccess, generate_struct_or_enum_variants_fields_mapping,
    },
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

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

    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
    let StructureData { ident, .. } = structure_data;

    let (deserialize_impl, visitor_impl) = match &structure_data.structure {
        Structure::Struct(fields) => generate_dif_deserialize_for_struct(
            structure_data,
            fields,
            &Ident::new(
                &format!("{}InnerDeserializeCollector", ident),
                Span::call_site(),
            ),
            &Ident::new(
                &format!("{}OuterDeserializeCollector", ident),
                Span::call_site(),
            ),
            &Ident::new(&format!("{}InnerWrapper", ident), Span::call_site()),
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

    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
    if let Fields::Transparent(inner) = fields {
        let field_ident = inner.normalized_ident();

        let value = match inner.field.attributes.field_mapping {
            FieldMapping::Datex => quote! {
                ValueWithSerdeContext::new(#field_ident, ctx)
            },
            FieldMapping::Serde => quote! {
                {
                    let value = serde_to_value_container(#field_ident);
                    ValueWithSerdeContext::new(&value, ctx)
                }
            },
        };

        return match tag {
            Some(tag) => quote! {
                use serde::ser::SerializeMap;

                let value = #value;
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("_", "t")?;
                map.serialize_entry("t", #tag)?;
                map.serialize_entry("v", &value)?;
                map.end()
            },
            None => quote! {
                let value = #value;
                serde::Serialize::serialize(&value, serializer)
            },
        };
    }

    let value = match fields {
        Fields::Named(named_fields) => {
            let field_definitions = named_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                let field_ty = field.ty();
                quote! {
                    #field_ident: &'a #field_ty
                }
            });

            let field_initializers = named_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                quote! {
                    #field_ident: #field_ident
                }
            });

            let serialize_fields = named_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                let field_name = field.datex_field_name();

                match field.field.attributes.field_mapping {
                    FieldMapping::Datex => quote! {
                        map.serialize_entry(
                            #field_name,
                            &ValueWithSerdeContext::new(
                                self.#field_ident,
                                self.ctx,
                            ),
                        )?;
                    },
                    FieldMapping::Serde => quote! {
                        let value = serde_to_value_container(self.#field_ident);
                        map.serialize_entry(
                            #field_name,
                            &ValueWithSerdeContext::new(&value, self.ctx),
                        )?;
                    },
                }
            });

            let len = named_fields.len();

            quote! {{
                struct __DatexFieldsMap<'a, 'ctx> {
                    ctx: &'a SerdeContext<'ctx>,
                    #(#field_definitions,)*
                }

                impl serde::Serialize for __DatexFieldsMap<'_, '_> {
                    fn serialize<S>(
                        &self,
                        serializer: S,
                    ) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        use serde::ser::SerializeMap;

                        let mut map = serializer.serialize_map(Some(#len))?;
                        #(#serialize_fields)*
                        map.end()
                    }
                }

                __DatexFieldsMap {
                    ctx,
                    #(#field_initializers,)*
                }
            }}
        }

        Fields::Unnamed(unnamed_fields) => {
            let field_definitions = unnamed_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                let field_ty = field.ty();
                quote! { #field_ident: &'a #field_ty }
            });

            let field_initializers = unnamed_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();
                quote! { #field_ident: #field_ident }
            });

            let serialize_fields = unnamed_fields.iter().map(|field| {
                let field_ident = field.normalized_ident();

                match field.field.attributes.field_mapping {
                    FieldMapping::Datex => quote! {
                        seq.serialize_element(
                            &ValueWithSerdeContext::new(
                                self.#field_ident,
                                self.ctx,
                            ),
                        )?;
                    },
                    FieldMapping::Serde => quote! {
                        let value = serde_to_value_container(self.#field_ident);
                        seq.serialize_element(
                            &ValueWithSerdeContext::new(&value, self.ctx),
                        )?;
                    },
                }
            });

            let len = unnamed_fields.len();

            quote! {{
                struct __DatexFieldsSeq<'a, 'ctx> {
                    ctx: &'a SerdeContext<'ctx>,
                    #(#field_definitions,)*
                }

                impl serde::Serialize for __DatexFieldsSeq<'_, '_> {
                    fn serialize<S>(
                        &self,
                        serializer: S,
                    ) -> Result<S::Ok, S::Error>
                    where
                        S: serde::Serializer,
                    {
                        use serde::ser::SerializeSeq;

                        let mut seq = serializer.serialize_seq(Some(#len))?;
                        #(#serialize_fields)*
                        seq.end()
                    }
                }

                __DatexFieldsSeq {
                    ctx,
                    #(#field_initializers,)*
                }
            }}
        }
        Fields::Unit => quote! { () },
        Fields::Transparent(_) => unreachable!(),
    };

    match tag {
        Some(tag) => quote! {{
            use serde::ser::SerializeMap;

            struct __DatexEnumValue<'a, T: serde::Serialize + ?Sized> {
                tag: &'a str,
                value: &'a T,
            }

            impl<T: serde::Serialize + ?Sized> serde::Serialize
                for __DatexEnumValue<'_, T>
            {
                fn serialize<S>(
                    &self,
                    serializer: S,
                ) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let mut map = serializer.serialize_map(Some(3))?;
                    map.serialize_entry("_", "t")?;
                    map.serialize_entry("t", self.tag)?;
                    map.serialize_entry("v", self.value)?;
                    map.end()
                }
            }

            let value = #value;
            serde::Serialize::serialize(
                &__DatexEnumValue {
                    tag: #tag,
                    value: &value,
                },
                serializer,
            )
        }},

        None => match fields {
            Fields::Named(_) => quote! {{
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(
                    &CoreLibIdIndex::from(CoreLibBaseTypeId::Map),
                )?;
                seq.serialize_element(&#value)?;
                seq.end()
            }},

            Fields::Unnamed(_) => quote! {{
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(
                    &CoreLibIdIndex::from(CoreLibBaseTypeId::List),
                )?;
                seq.serialize_element(&#value)?;
                seq.end()
            }},

            Fields::Transparent(_) | Fields::Unit => quote! {
                serde::Serialize::serialize(&#value, serializer)
            },
        },
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
    let variants_construct = variants.iter().map(|variant| {
        let variant_ident = variant.ident();
        let tag = variant_ident.to_string();
        quote! {
            #tag => {
                // TODO handle variant deserialization
                todo!("handle deserialization for variant: {}", #tag);
            }
        }
    });
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

            fn visit_map<_MapAccess>(self, mut map: _MapAccess) -> Result<Self::Value, _MapAccess::Error> where _MapAccess: MapAccess<'de> {
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
                let value: Value = if let Some(key) = map.next_key::<String>()? {
                    if key == "v" {
                        map.next_value()?
                    } else {
                        return Err(_MapAccess::Error::custom(format!("Expected 'v' key, found '{}'", key)));
                    }
                } else {
                    return Err(_MapAccess::Error::custom("Expected 'v' key, found none"));
                };
                match tag.as_str() {
                    #(#variants_construct),*
                    _ => {
                        return Err(_MapAccess::Error::custom(format!("Unknown variant tag: {}", tag)));
                    }
                }
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
    inner_collector: &Ident,
    outer_collector: &Ident,
    inner_wrapper: &Ident,
) -> (TokenStream, TokenStream) {
    let StructureData { ident, .. } = structure_data;

    let (properties, property_defaults, property_collapses) =
        generate_property_mappings(fields);

    let collapse_init =
        fields.wrap_in_initializer(property_collapses.into_iter());

    let outer_deserialize_impl = quote! {
        deserializer.deserialize_any(#outer_collector::new(ctx))
    };

    let (outer_visitor_methods, inner_visitor_methods) = match fields {
        Fields::Named(named) => (
            generate_outer_visit_seq(
                inner_wrapper,
                &Ident::new("Map", Span::call_site()),
            ),
            Some(generate_inner_visitor_methods_for_named_fields(named)),
        ),
        Fields::Unnamed(unnamed) => (
            generate_outer_visit_seq(
                inner_wrapper,
                &Ident::new("List", Span::call_site()),
            ),
            Some(generate_inner_visitor_methods_for_unnamed_fields(unnamed)),
        ),
        Fields::Transparent(inner) => {
            // does not need the custom visitor, just pass through to the inner type's deserialization
            return (
                generate_deserialize_impl_for_transparent_struct(ident, inner),
                quote! {},
            );
        }
        Fields::Unit => (generate_inner_visitor_methods_for_unit(ident), None),
    };

    let inner_visitor_impl = if let Some(inner_visitor_methods) =
        inner_visitor_methods
    {
        let inner_wrapper_impl = quote! {
            struct #inner_wrapper<'a, 'ctx> {
                pub __ctx: &'a SerdeContext<'ctx>,
            }

            impl<'a, 'ctx> #inner_wrapper<'a, 'ctx> {
                fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
                    #inner_wrapper { __ctx: ctx }
                }
            }

            impl<'de, 'a, 'ctx> DeserializeSeed<'de> for #inner_wrapper<'a, 'ctx> {
                type Value = #ident;

                fn deserialize<D>(self, deserializer: D, ) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_any(#inner_collector::new(self.__ctx))
                }
            }
        };

        quote! {
            #inner_wrapper_impl

            struct #inner_collector<'a, 'ctx> {
                pub __ctx: &'a SerdeContext<'ctx>,
                #(#properties),*
            }

            impl<'a, 'ctx> #inner_collector<'a, 'ctx> {
                fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
                    #inner_collector::from(ctx)
                }
                fn collapse(self) -> Result<#ident, &'static str> {
                    Ok(#ident #collapse_init)
                }
            }

            impl<'a, 'ctx> From<&'a SerdeContext<'ctx>> for #inner_collector<'a, 'ctx> {
                fn from(ctx: &'a SerdeContext<'ctx>) -> Self {
                    #inner_collector {
                        __ctx: ctx,
                        #(#property_defaults),*
                    }
                }
            }

            impl<'de, 'a, 'ctx> Visitor<'de> for #inner_collector<'a, 'ctx> {
                type Value = #ident;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str(
                        "the inner value serialization",
                    )
                }

                #inner_visitor_methods
            }

        }
    } else {
        quote! {}
    };

    let outer_visitor_impl = quote! {
        struct #outer_collector<'a, 'ctx> {
            pub __ctx: &'a SerdeContext<'ctx>,
        }

        impl<'a, 'ctx> #outer_collector<'a, 'ctx> {
            fn new(ctx: &'a SerdeContext<'ctx>) -> Self {
                #outer_collector { __ctx: ctx }
            }
        }

        impl<'de, 'a, 'ctx> Visitor<'de> for #outer_collector<'a, 'ctx> {
            type Value = #ident;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "the form [key, value] to mark [CoreLibId, Value] pair",
                )
            }

            #outer_visitor_methods
        }
    };

    (
        outer_deserialize_impl,
        quote! {
            #inner_visitor_impl
            #outer_visitor_impl
        },
    )
}

fn generate_inner_visitor_methods_for_named_fields(
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
    }
}

fn generate_outer_visit_seq(
    inner_wrapper: &Ident,
    core_lib_name: &Ident,
) -> TokenStream {
    quote! {
        fn visit_seq<_SeqAccess>(
            self,
            mut seq: _SeqAccess,
        ) -> Result<Self::Value, _SeqAccess::Error>
        where
            _SeqAccess: SeqAccess<'de>,
        {
            let core_lib_index: CoreLibIdIndex = seq
                .next_element()?
                .ok_or_else(|| {
                    _SeqAccess::Error::invalid_length(
                        0,
                        &"expect two element seq",
                    )
                })?;

            if core_lib_index != CoreLibIdIndex::from(CoreLibBaseTypeId::#core_lib_name) {
                return Err(_SeqAccess::Error::custom(
                    "Invalid core lib id",
                ));
            }
            seq
                .next_element_seed(
                    #inner_wrapper::new(self.__ctx)
                )?
                .ok_or_else(|| {
                    _SeqAccess::Error::invalid_length(
                        1,
                        &"expect two element seq",
                    )
                })
        }
    }
}

fn generate_inner_visitor_methods_for_unnamed_fields(
    fields: &[IndexedField],
) -> TokenStream {
    let list_key_deserializers = fields.iter().map(|field| {
        let field_index = field.accessor();
        let normalized_ident = field.normalized_ident();
        let ty = field.ty();

        match field.field.attributes.field_mapping {
            FieldMapping::Datex => quote! {
                let value = seq.next_element_seed(DeserializeSerdeContext::<#ty>::new(self.__ctx))?;
                self.#normalized_ident = value;
            },
            FieldMapping::Serde => quote! {
                let value_container = seq.next_element_seed(DeserializeSerdeContext::<ValueContainer>::new(self.__ctx))?;
                let value = try_serde_from_value_container(value_container).map_err(|_err| _SeqAccess::Error::custom(format!("Failed to deserialize index {}", #field_index)))?;
                self.#normalized_ident = value;
            }
        }
    });

    quote! {
        fn visit_seq<_SeqAccess>(mut self, mut seq: _SeqAccess) -> Result<Self::Value, _SeqAccess::Error>
        where
            _SeqAccess: SeqAccess<'de>,
        {
            #(#list_key_deserializers)*

            self.collapse().map_err(|err| _SeqAccess::Error::missing_field(err))
        }
    }
}

fn generate_inner_visitor_methods_for_unit(ident: &syn::Ident) -> TokenStream {
    quote! {
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(#ident)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(#ident)
        }
    }
}

fn generate_deserialize_impl_for_transparent_struct(
    ident: &syn::Ident,
    field: &IndexedField,
) -> TokenStream {
    let inner_ty = field.ty();
    quote! {
        let value: #inner_ty = DeserializeSerdeContext::<#inner_ty>::new(ctx).deserialize(deserializer)?;
        Ok(#ident(value))
    }
}

fn generate_property_mappings(
    fields: &Fields,
) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {
    fields
        .iter()
        .map(|field: &dyn AnyField| {
            let field_ty = field.ty();
            let normalized_ident = field.normalized_ident();
            let field_name = field.original_name();

            let field_init = quote! {
                pub #normalized_ident: Option<#field_ty>
            };

            let field_default = quote! {
                #normalized_ident: None
            };

            // special case: null values and omitted properties are treated
            // the same for structural types, so a missing property for an Option field is set to None per default
            let is_option_field = is_option(field_ty);

            // also fall back to default value if not set if datex(default) is set
            let skip_with_default = field
                .try_as_named_field()
                .map(|named_field| named_field.attributes.skip_with_default)
                .unwrap_or(false);

            let field_collapse = if skip_with_default || is_option_field {
                quote! {
                    self.#normalized_ident.unwrap_or_default()
                }
            } else {
                quote! {
                    self.#normalized_ident.ok_or_else(|| #field_name)?
                }
            };

            (field_init, field_default, field_collapse)
        })
        .collect()
}

fn is_option(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            type_path.path.segments.last().is_some_and(|segment| {
                segment.ident == "Option"
                    && matches!(
                        &segment.arguments,
                        PathArguments::AngleBracketed(args)
                            if args.args.len() == 1
                                && matches!(
                                    args.args.first(),
                                    Some(GenericArgument::Type(_))
                                )
                    )
            })
        }
        _ => false,
    }
}

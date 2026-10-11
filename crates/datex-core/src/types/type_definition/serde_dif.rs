use core::{fmt, ops::Deref};

use crate::{
    dif::serde_context::SerdeContext,
    libs::core::core_lib_id::{CoreLibId, CoreLibIdIndex},
    prelude::*,
    shared_values::SharedContainer,
    types::{
        shared_container_containing_type::SharedContainerContainingType,
        r#type::Type,
        type_definition::{
            TypeDefinition, callable::CallableTypeDefinition,
            collection::CollectionTypeDefinition,
            impl_type::ImplMarkers,
            intersection::IntersectionTypeDefinition, list::ListTypeDefinition,
            map::MapTypeDefinition, range::RangeTypeDefinition,
            tagged_type::TaggedTypeDefinition, union::UnionTypeDefinition,
        },
    },
    dif::serialize_with_serde_context::SerializeWithSerdeContext,
};
use serde::{
    Deserializer, Serializer,
    de::{self, DeserializeSeed, Visitor},
    ser::SerializeMap,
};
use crate::dif::deserialize_serde_context::DeserializeSerdeContext;
use crate::dif::value_with_serde_context::ValueWithSerdeContext;
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;

impl SerializeWithSerdeContext for TypeDefinition {

    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TypeDefinition::CoreType(core) => serializer
                .serialize_u16(CoreLibIdIndex::from(CoreLibId::Type(*core)).0),
            _ => {
                let mut outer = serializer.serialize_map(Some(1))?;
                outer.serialize_key(self.as_ref())?;
                match self {
                    TypeDefinition::Literal(literal) => {
                        outer.serialize_value(&literal)?;
                    }
                    TypeDefinition::List(list_def) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            list_def,
                            ctx,
                        ))?
                    }
                    TypeDefinition::Map(map_def) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            map_def,
                            ctx
                        ))?
                    }
                    TypeDefinition::Range(range) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            range,
                            ctx,
                        ))?
                    }
                    TypeDefinition::Collection(collection_type_definition) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            collection_type_definition,
                            ctx,
                        ))?
                    }
                    TypeDefinition::Shared(
                        shared_container_containing_type,
                    ) => outer.serialize_value(&ValueWithSerdeContext::new(
                        shared_container_containing_type.deref(),
                        ctx,
                    ))?,
                    TypeDefinition::Box(nested) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            nested as &Type,
                            ctx,
                        ))?
                    }
                    TypeDefinition::Callable(callable_signature) => outer
                        .serialize_value(&ValueWithSerdeContext::new(
                            callable_signature,
                            ctx,
                        ))?,
                    TypeDefinition::ImplMarkers(def) => {
                        outer.serialize_value(&ValueWithSerdeContext::new(
                            def,
                            ctx,
                        ))?
                    }
                    TypeDefinition::Intersection(type_intersection) => outer
                        .serialize_value(&ValueWithSerdeContext::new(
                            type_intersection,
                            ctx,
                        ))?,
                    TypeDefinition::Union(type_union) => outer
                        .serialize_value(&ValueWithSerdeContext::new(
                            type_union,
                            ctx,
                        ))?,
                    TypeDefinition::TaggedType(tagged_type) => outer
                        .serialize_value(&ValueWithSerdeContext::new(
                            tagged_type,
                            ctx,
                        ))?,
                    TypeDefinition::CoreType(_) => unreachable!(), // already handled above
                }
                outer.end()
            }
        }
    }
}

/// Deserialization for [TypeDefinition]
impl<'de> DeserializeWithSerdeContext<'de> for TypeDefinition {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        d: D,
    ) -> Result<TypeDefinition, D::Error> {
        d.deserialize_any(DeserializeSerdeContext::<TypeDefinition>::new(ctx))
    }
}
impl<'de, 'ctx> DeserializeSerdeContext<'de, 'ctx, TypeDefinition> {
    fn deserialize_core_lib_id(
        &self,
        value: u64,
    ) -> Result<TypeDefinition, String> {
        let index = u16::try_from(value).map_err(|_| {
            format!(
                "CoreLibId index out of range for TypeDefinition: {}",
                value
            )
        })?;

        match CoreLibId::try_from(CoreLibIdIndex(index)) {
            Ok(CoreLibId::Type(core_type_id)) => {
                Ok(TypeDefinition::CoreType(core_type_id))
            }
            _ => Err(format!(
                "Invalid CoreLibId for TypeDefinition: {:?}",
                value
            )),
        }
    }
}
impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, TypeDefinition> {
    type Value = TypeDefinition;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a valid TypeDefinition")
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let key: String = map.next_key()?.ok_or_else(|| {
            de::Error::custom("expected TypeDefinition map with one key")
        })?;

        let value = match key.as_str() {
            "literal" => {
                let literal = map.next_value()?;
                TypeDefinition::Literal(literal)
            }

            "list" => {
                let list =
                    map.next_value_seed(self.cast::<ListTypeDefinition>())?;
                TypeDefinition::List(list)
            }

            "map" => {
                let map_def =
                    map.next_value_seed(self.cast::<MapTypeDefinition>())?;
                TypeDefinition::Map(map_def)
            }

            "range" => {
                let range =
                    map.next_value_seed(self.cast::<RangeTypeDefinition>())?;
                TypeDefinition::Range(range)
            }

            "collection" => {
                let collection = map
                    .next_value_seed(self.cast::<CollectionTypeDefinition>())?;
                TypeDefinition::Collection(collection)
            }

            "nested" => {
                let ty = map.next_value_seed(self.cast::<Type>())?;
                TypeDefinition::Box(Box::new(ty))
            }

            "callable" => {
                let callable =
                    map.next_value_seed(self.cast::<CallableTypeDefinition>())?;
                TypeDefinition::Callable(callable)
            }

            "impl_markers" => {
                let def =
                    map.next_value_seed(self.cast::<ImplMarkers>())?;
                TypeDefinition::ImplMarkers(def)
            }

            "intersection" => {
                let intersection = map.next_value_seed(
                    self.cast::<IntersectionTypeDefinition>(),
                )?;
                TypeDefinition::Intersection(intersection)
            }

            "union" => {
                let union =
                    map.next_value_seed(self.cast::<UnionTypeDefinition>())?;
                TypeDefinition::Union(union)
            }

            "tagged_type" => {
                let tagged =
                    map.next_value_seed(self.cast::<TaggedTypeDefinition>())?;
                TypeDefinition::TaggedType(tagged)
            }

            "shared" => {
                let shared_container =
                    map.next_value_seed(self.cast::<SharedContainer>())?;
                SharedContainerContainingType::try_from(shared_container)
                    .map(TypeDefinition::Shared)
                    .map_err(|_| {
                        de::Error::custom("Failed to convert shared container to SharedContainerContainingType".to_string())
                    })?
            }

            other => {
                return Err(de::Error::unknown_variant(
                    other,
                    &[
                        "literal",
                        "list",
                        "map",
                        "range",
                        "collection",
                        "nested",
                        "callable",
                        "impl_markers",
                        "intersection",
                        "union",
                        "tagged_type",
                        "shared",
                    ],
                ));
            }
        };

        if let Some(extra_key) = map.next_key::<String>()? {
            return Err(de::Error::custom(format!(
                "expected TypeDefinition map with exactly one key, found extra key `{}`",
                extra_key
            )));
        }

        Ok(value)
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.deserialize_core_lib_id(value as u64)
            .map_err(E::custom)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.deserialize_core_lib_id(value).map_err(E::custom)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value < 0 {
            return Err(E::custom(format!(
                "Invalid negative CoreLibId for TypeDefinition: {}",
                value
            )));
        }

        self.deserialize_core_lib_id(value as u64)
            .map_err(E::custom)
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;
    use crate::{
        dif::serde_context::SerdeContext,
        libs::core::{
            core_lib_id::CoreLibIdIndex,
            type_id::{CoreLibTypeId, CoreLibBaseTypeId, CoreLibVariantTypeId},
        },
        prelude::*,
        types::type_definition::TypeDefinition,
        values::core_values::integer::typed_integer::IntegerTypeVariant,
    };

    fn to_json(value: &TypeDefinition) -> String {
        SerdeContext::new(&RefCell::new(SharedValuesCache::default()))
            .serialize_to_json(value)
    }
    use crate::runtime::cache::{
        shared_values_cache::SharedValuesCache,
    };
    use test_case::test_case;

    #[test_case(CoreLibTypeId::Base(CoreLibBaseTypeId::Text) ; "Text")]
    #[test_case(CoreLibTypeId::Variant(CoreLibVariantTypeId::Integer(IntegerTypeVariant::U8)) ; "integer/u8")]
    fn core_library_type_definition(id: CoreLibTypeId) {
        let type_def = TypeDefinition::CoreType(id);
        // Serialize the TypeDefinition to JSON
        let serialized = to_json(&type_def);

        // Assert that the serialized JSON is just the CoreLibId index as a number
        assert_eq!(serialized, format!(r#"{}"#, CoreLibIdIndex::from(id)));

        // Deserialize the JSON back to a TypeDefinition
        let deserialized: TypeDefinition =
            SerdeContext::new(&RefCell::new(SharedValuesCache::default()))
                .try_deserialize_from_json(&serialized)
                .unwrap();
        assert_eq!(type_def, deserialized);
    }
}

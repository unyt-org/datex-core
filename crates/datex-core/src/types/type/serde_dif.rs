use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
    },
    libs::core::{core_lib_id::CoreLibIdIndex, type_id::CoreLibTypeId},
    prelude::*,
    shared_values::SharedContainer,
    types::{
        entity_type::EntityType,
        r#type::Type,
        type_definition::TypeDefinition,
        type_definition_with_metadata::{
            TypeDefinitionWithMetadata, TypeMetadata,
        },
    },
};
use core::ops::Deref;
use num::ToPrimitive;
use serde::{
    Serializer,
    de::{IntoDeserializer, Visitor},
};

impl SerializeWithSerdeContext for Type {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Type::Definition(type_definition) => match type_definition
                .definition
            {
                TypeDefinition::CoreType(_core)
                    if type_definition.metadata == TypeMetadata::default() =>
                {
                    type_definition
                        .definition
                        .serialize_with_ctx(ctx, serializer)
                }
                _ => type_definition.serialize_with_ctx(ctx, serializer),
            },
            Type::Entity(shared_container_containing_nominal_type) => {
                shared_container_containing_nominal_type
                    .deref()
                    .serialize_with_ctx(ctx, serializer)
            }
        }
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Type {
    fn deserialize_with_ctx<D: serde::de::Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DeserializeSerdeContext::<Type>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Type> {
    type Value = Type;
    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str("a type definition")
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }

    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v.to_u16().ok_or_else(|| {
            serde::de::Error::custom(format!("core type id out of range: {v}"))
        })?)
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Type::Definition(
            TypeDefinition::core(
                CoreLibTypeId::try_from(CoreLibIdIndex(v)).map_err(|_| {
                    serde::de::Error::custom(format!(
                        "invalid core type id: {v}"
                    ))
                })?,
            )
            .into(),
        ))
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v as u16)
    }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u16(v.to_u16().ok_or_else(|| {
            serde::de::Error::custom(format!("core type id out of range: {v}"))
        })?)
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Type::Entity(unsafe {
            EntityType::new_unchecked(SharedContainer::deserialize_with_ctx(
                self.ctx,
                v.into_deserializer(),
            )?)
        }))
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(&v)
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let metadata = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("missing metadata"))?;
        let definition = seq
            .next_element_seed(self.cast::<TypeDefinition>())?
            .ok_or_else(|| serde::de::Error::custom("missing definition"))?;
        Ok(Type::Definition(TypeDefinitionWithMetadata::new(
            definition, metadata,
        )))
    }
}

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    types::{
        r#type::Type,
        type_definition::callable::{CallableKind, CallableTypeDefinition},
    },
};
use core::ops::Deref;
use serde::{
    Deserializer, Serializer,
    de::{MapAccess, Visitor},
    ser::{SerializeMap, SerializeSeq, SerializeTuple},
};

impl SerializeWithSerdeContext for CallableTypeDefinition {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut obj = serializer.serialize_map(Some(1))?;
        obj.serialize_key("kind")?;
        obj.serialize_value(&self.kind)?;

        obj.serialize_key("requires_async")?;
        obj.serialize_value(&self.requires_async)?;

        obj.serialize_key("parameters")?;
        obj.serialize_value(&ValueWithSerdeContext::new(
            &self.parameters,
            ctx,
        ))?;
        obj.serialize_key("rest_parameter")?;
        match &self.rest_parameter {
            Some((name, ty)) => {
                obj.serialize_value(&ValueWithSerdeContext::new(
                    &(name.clone(), ty.deref().clone()),
                    ctx,
                ))?;
            }
            None => {
                obj.serialize_value(&())?;
            }
        }

        obj.serialize_key("return_type")?;
        match &self.return_type {
            Some(return_type) => {
                obj.serialize_value(&ValueWithSerdeContext::new(
                    return_type.deref(),
                    ctx,
                ))?;
            }
            None => {
                obj.serialize_value(&())?;
            }
        }

        obj.serialize_key("yeet_type")?;
        match &self.yeet_type {
            Some(yeet_type) => {
                obj.serialize_value(&ValueWithSerdeContext::new(
                    yeet_type.deref(),
                    ctx,
                ))?;
            }
            None => {
                obj.serialize_value(&())?;
            }
        }

        obj.end()
    }
}

impl SerializeWithSerdeContext for (Option<String>, Type) {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.0)?;
        tuple.serialize_element(&ValueWithSerdeContext::new(&self.1, ctx))?;
        tuple.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for CallableTypeDefinition {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DeserializeSerdeContext::<
            CallableTypeDefinition,
        >::new(ctx))
    }
}
impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, CallableTypeDefinition>
{
    type Value = CallableTypeDefinition;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str("a callable type definition")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut kind: Option<CallableKind> = None;
        let mut requires_async: Option<bool> = None;
        let mut parameters: Option<Vec<(Option<String>, Type)>> = None;
        let mut rest_parameter: Option<(Option<String>, Box<Type>)> = None;
        let mut return_type: Option<Type> = None;
        let mut yeet_type: Option<Type> = None;

        let mut map = map;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "kind" => {
                    kind = Some(map.next_value()?);
                }
                "requires_async" => {
                    requires_async = Some(map.next_value()?);
                }
                "parameters" => {
                    parameters = Some(map.next_value_seed(
                        self.cast::<Vec<(Option<String>, Type)>>(),
                    )?);
                }
                "rest_parameter" => {
                    rest_parameter = map
                        .next_value_seed(
                            self.cast::<Option<(Option<String>, Type)>>(),
                        )?
                        .map(|(name, ty)| (name, Box::new(ty)));
                }
                "return_type" => {
                    return_type =
                        map.next_value_seed(self.cast::<Option<Type>>())?;
                }
                "yeet_type" => {
                    yeet_type =
                        map.next_value_seed(self.cast::<Option<Type>>())?;
                }
                _ => {
                    // Ignore unknown keys
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let kind =
            kind.ok_or_else(|| serde::de::Error::missing_field("kind"))?;
        let parameters = parameters
            .ok_or_else(|| serde::de::Error::missing_field("parameters"))?;

        Ok(CallableTypeDefinition {
            kind,
            requires_async: requires_async.unwrap_or(false),
            parameters,
            rest_parameter,
            return_type: return_type.map(Box::new),
            yeet_type: yeet_type.map(Box::new),
        })
    }
}

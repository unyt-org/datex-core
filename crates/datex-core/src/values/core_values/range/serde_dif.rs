use core::fmt;

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    values::{core_values::range::Range, value_container::ValueContainer},
};
use serde::{
    Serializer,
    de::{Error, IgnoredAny, MapAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct, SerializeTuple},
};

impl SerializeWithSerdeContext for Range {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_tuple(2)?;
        state.serialize_element(&ValueWithSerdeContext::new(
            &*self.start,
            ctx,
        ))?;
        state
            .serialize_element(&ValueWithSerdeContext::new(&*self.end, ctx))?;
        state.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Range {
    fn deserialize_with_ctx<D>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Range, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(DeserializeSerdeContext::<Range>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Range> {
    type Value = Range;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a range")
    }

    /// We can have the range represented as a sequence of two elements (start and end)
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let start = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| A::Error::custom("missing start element"))?;
        let end = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| A::Error::custom("missing end element"))?;

        Ok(Range {
            start: Box::new(start),
            end: Box::new(end),
        })
    }

    /// We expect the range to be represented as a map with "start" and "end" keys, each containing a ValueContainer.
    /// TODO: Add inclusive / exclusive marker here
    fn visit_map<A>(self, mut map: A) -> Result<Range, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut start: Option<ValueContainer> = None;
        let mut end: Option<ValueContainer> = None;
        while let Some(field) = map.next_key::<String>()? {
            match field.as_str() {
                "start" => {
                    start = Some(
                        map.next_value_seed(self.cast::<ValueContainer>())?,
                    );
                }

                "end" => {
                    end = Some(
                        map.next_value_seed(self.cast::<ValueContainer>())?,
                    );
                }
                _ => {
                    let _: IgnoredAny = map.next_value()?;
                }
            }
        }

        let start =
            start.ok_or_else(|| A::Error::custom("missing start field"))?;
        let end = end.ok_or_else(|| A::Error::custom("missing end field"))?;

        Ok(Range {
            start: Box::new(start),
            end: Box::new(end),
        })
    }
}

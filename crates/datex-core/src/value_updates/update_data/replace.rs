use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        serde_context::SerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    values::value_container::ValueContainer,
};
use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ReplaceUpdateData {
    pub value: ValueContainer,
}
impl ReplaceUpdateData {
    pub fn new(value: ValueContainer) -> Self {
        ReplaceUpdateData { value }
    }
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn serialize_replace_fields<S: SerializeSeq>(
        &self,
        value: &ReplaceUpdateData,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        seq.serialize_element(&ValueWithSerdeContext::new(&value.value, self))?;

        Ok(())
    }
}
impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ReplaceUpdateData>
{
    type Value = ReplaceUpdateData;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "a replace update data sequence with 1 element")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let value = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        Ok(ReplaceUpdateData { value })
    }
}

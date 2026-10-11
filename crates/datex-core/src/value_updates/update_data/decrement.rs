use crate::values::value_container::ValueContainer;
use core::fmt;

use crate::dif::{
    deserialize_serde_context::DeserializeSerdeContext,
    serde_context::SerdeContext,
    value_with_serde_context::ValueWithSerdeContext,
};
use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct DecrementUpdateData {
    pub value: ValueContainer,
}
impl DecrementUpdateData {
    pub fn new(value: ValueContainer) -> Self {
        DecrementUpdateData { value }
    }
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn serialize_decrement_fields<S: SerializeSeq>(
        &self,
        value: &DecrementUpdateData,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        seq.serialize_element(&ValueWithSerdeContext::new(&value.value, self))?;
        Ok(())
    }
}
impl<'de> Visitor<'de>
    for DeserializeSerdeContext<'de, '_, DecrementUpdateData>
{
    type Value = DecrementUpdateData;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "a delete entry update data sequence with 1 element (key)"
        )
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let key = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        Ok(DecrementUpdateData { value: key })
    }
}

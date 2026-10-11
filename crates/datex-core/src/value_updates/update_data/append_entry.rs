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
pub struct AppendEntryUpdateData {
    pub value: ValueContainer,
}
impl AppendEntryUpdateData {
    pub fn new(value: ValueContainer) -> Self {
        AppendEntryUpdateData { value }
    }
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn serialize_append_entry_fields<S: SerializeSeq>(
        &self,
        value: &AppendEntryUpdateData,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        seq.serialize_element(&ValueWithSerdeContext::new(&value.value, self))?;

        Ok(())
    }
}
impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, AppendEntryUpdateData>
{
    type Value = AppendEntryUpdateData;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an append entry update data sequence with 1 element")
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let value = seq
            .next_element_seed(self.cast::<ValueContainer>())?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        Ok(AppendEntryUpdateData { value })
    }
}

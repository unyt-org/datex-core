use crate::values::value_container::value_key::ValueKey;
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
pub struct DeleteEntryUpdateData {
    pub key: ValueKey,
}
impl DeleteEntryUpdateData {
    pub fn new(key: ValueKey) -> Self {
        DeleteEntryUpdateData { key }
    }
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn serialize_delete_entry_fields<S: SerializeSeq>(
        &self,
        value: &DeleteEntryUpdateData,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        seq.serialize_element(&ValueWithSerdeContext::new(&value.key, self))?;
        Ok(())
    }
}
impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, DeleteEntryUpdateData>
{
    type Value = DeleteEntryUpdateData;

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
            .next_element_seed(self.cast::<ValueKey>())?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        Ok(DeleteEntryUpdateData { key })
    }
}

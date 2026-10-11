use core::fmt;

use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        serde_context::SerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    values::value_container::ValueContainer,
};
use serde::{
    de::{self, Visitor},
    ser::SerializeSeq,
};

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct ListSpliceUpdateData {
    pub start: u32,
    pub delete_count: u32,
    pub items: Vec<ValueContainer>,
}
impl ListSpliceUpdateData {
    pub fn new(
        start: u32,
        delete_count: u32,
        items: Vec<ValueContainer>,
    ) -> Self {
        ListSpliceUpdateData {
            start,
            delete_count,
            items,
        }
    }
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn serialize_list_splice_fields<S: SerializeSeq>(
        &self,
        value: &ListSpliceUpdateData,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        seq.serialize_element(&value.start)?;
        seq.serialize_element(&value.delete_count)?;

        seq.serialize_element(&ValueWithSerdeContext::new(&value.items, self))?;

        Ok(())
    }
}
impl<'de, 'a, 'ctx> Visitor<'de>
    for DeserializeSerdeContext<'a, 'ctx, ListSpliceUpdateData>
{
    type Value = ListSpliceUpdateData;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "a list splice update data sequence with 3 elements (start, delete_count, items)"
        )
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let start = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let delete_count = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let items = seq
            .next_element_seed(self.cast::<Vec<ValueContainer>>())?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;

        Ok(ListSpliceUpdateData {
            start,
            delete_count,
            items,
        })
    }
}

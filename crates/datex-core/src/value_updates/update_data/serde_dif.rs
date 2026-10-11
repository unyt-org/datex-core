use crate::{
    dif::{
        deserialize_serde_context::DeserializeSerdeContext,
        deserialize_with_serde_context::DeserializeWithSerdeContext,
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    prelude::*,
    shared_values::base_shared_value_container::observers::TransceiverId,
    value_updates::update_data::{
        AppendEntryUpdateData, DeleteEntryUpdateData, ListSpliceUpdateData,
        ReplaceUpdateData, SetEntryUpdateData, Update, UpdateData,
        UpdateOperation,
    },
    values::value_container::value_key::ValueKey,
};
use core::fmt;
use serde::{
    Deserializer, Serializer,
    de::{self, SeqAccess, Visitor},
    ser::SerializeSeq,
};

impl SerializeWithSerdeContext for Update {
    fn serialize_with_ctx<S: Serializer>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        // serialize the transceiver id and the operation name as a string
        seq.serialize_element(self.source_id())?;

        // serialize path, if the path is empty, we pack an empty vec, otherwise we pack the path as a sequence of ValueKeys
        {
            let val = self.path();
            let path_val = ValueWithSerdeContext::new(&val, ctx);
            seq.serialize_element(&path_val)?;
        }

        // this serializes the name of the operation (e.g. "set_entry", "replace", etc.) as a string
        {
            seq.serialize_element(&&self.operation().as_ref().to_string())?;
            match self.operation() {
                UpdateOperation::SetEntry(data) => {
                    ctx.serialize_set_entry_fields(data, &mut seq)?
                }
                UpdateOperation::Replace(data) => {
                    ctx.serialize_replace_fields(data, &mut seq)?
                }
                UpdateOperation::DeleteEntry(data) => {
                    ctx.serialize_delete_entry_fields(data, &mut seq)?
                }
                UpdateOperation::Clear => {}
                UpdateOperation::AppendEntry(data) => {
                    ctx.serialize_append_entry_fields(data, &mut seq)?
                }
                UpdateOperation::ListSplice(data) => {
                    ctx.serialize_list_splice_fields(data, &mut seq)?
                }
                UpdateOperation::Increment(data) => {
                    ctx.serialize_increment_fields(data, &mut seq)?
                }
                UpdateOperation::Decrement(data) => {
                    ctx.serialize_decrement_fields(data, &mut seq)?
                }
            };
        }
        seq.end()
    }
}

impl<'de> DeserializeWithSerdeContext<'de> for Update {
    fn deserialize_with_ctx<D: Deserializer<'de>>(
        ctx: &SerdeContext<'_>,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer
            .deserialize_seq(DeserializeSerdeContext::<Update>::new(ctx))
    }
}

impl<'de, 'a, 'ctx> Visitor<'de> for DeserializeSerdeContext<'a, 'ctx, Update> {
    type Value = Update;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an update-data sequence starting with a variant hint")
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        let transceiver_id: TransceiverId = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        let path = seq
            .next_element_seed(self.cast::<Vec<ValueKey>>())?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;

        let kind: String = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        let operation = match kind.as_str() {
            "replace" => UpdateOperation::Replace(Box::new(
                self.cast::<ReplaceUpdateData>().visit_seq(&mut seq)?,
            )),

            "set_entry" => UpdateOperation::SetEntry(Box::new(
                self.cast::<SetEntryUpdateData>().visit_seq(&mut seq)?,
            )),

            "delete_entry" => UpdateOperation::DeleteEntry(Box::new(
                self.cast::<DeleteEntryUpdateData>().visit_seq(&mut seq)?,
            )),

            "clear" => UpdateOperation::Clear,

            "append_entry" => UpdateOperation::AppendEntry(Box::new(
                self.cast::<AppendEntryUpdateData>().visit_seq(&mut seq)?,
            )),

            "list_splice" => UpdateOperation::ListSplice(Box::new(
                self.cast::<ListSpliceUpdateData>().visit_seq(&mut seq)?,
            )),

            other => {
                return Err(de::Error::unknown_variant(
                    other,
                    &[
                        "replace",
                        "set_entry",
                        "delete_entry",
                        "clear",
                        "append_entry",
                        "list_splice",
                    ],
                ));
            }
        };

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            return Err(de::Error::custom(format!(
                "unexpected trailing value after `{kind}` update payload"
            )));
        }

        Ok(Update::new(
            transceiver_id,
            UpdateData::new_with_path(operation, path),
        ))
    }
}

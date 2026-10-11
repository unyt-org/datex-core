use crate::{
    dif::{
        serde_context::SerdeContext,
        serialize_with_serde_context::SerializeWithSerdeContext,
        value_with_serde_context::ValueWithSerdeContext,
    },
    types::type_definition_with_metadata::TypeDefinitionWithMetadata,
};
use serde::{Serializer, ser::SerializeSeq};

impl SerializeWithSerdeContext for TypeDefinitionWithMetadata {
    fn serialize_with_ctx<S>(
        &self,
        ctx: &SerdeContext<'_>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.metadata)?;
        seq.serialize_element(&ValueWithSerdeContext::new(
            &self.definition,
            ctx,
        ))?;
        seq.end()
    }
}

use crate::runtime::cache::shared_values_cache::SharedValuesCache;
use core::cell::RefCell;

#[cfg(test)]
use crate::dif::deserialize_with_serde_context::DeserializeWithSerdeContext;
#[cfg(test)]
use crate::dif::serialize_with_serde_context::SerializeWithSerdeContext;
#[cfg(test)]
use crate::prelude::*;

#[derive(Debug)]
pub struct SerdeContext<'ctx> {
    pub shared_container_cache: &'ctx RefCell<SharedValuesCache>,
}

impl<'ctx> SerdeContext<'ctx> {
    pub fn new(
        shared_container_cache: &'ctx RefCell<SharedValuesCache>,
    ) -> Self {
        Self {
            shared_container_cache,
        }
    }

    /// Try to deserialize a JSON string to a DATEX value using the provided context
    #[cfg(test)]
    pub fn try_deserialize_from_json<T>(
        &self,
        json_string: &'ctx str,
    ) -> Result<T, serde_json::Error>
    where
        T: DeserializeWithSerdeContext<'ctx>,
    {
        DeserializeWithSerdeContext::deserialize_with_ctx(
            self,
            &mut serde_json::Deserializer::from_str(json_string),
        )
    }

    /// Convert a serializable DATEX value to a JSON string
    #[cfg(test)]
    pub fn serialize_to_json<T>(&mut self, value: &T) -> String
    where
        T: SerializeWithSerdeContext,
    {
        use crate::prelude::*;
        let mut serializer = serde_json::Serializer::new(Vec::new());
        value.serialize_with_ctx(self, &mut serializer).unwrap();
        let bytes = serializer.into_inner();
        String::from_utf8(bytes).unwrap()
    }
}

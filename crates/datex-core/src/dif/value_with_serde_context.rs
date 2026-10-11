use crate::dif::{
    serde_context::SerdeContext,
    serialize_with_serde_context::SerializeWithSerdeContext,
};
use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct ValueWithSerdeContext<'a, 'ctx, V: ?Sized> {
    pub(crate) value: &'a V,
    pub(crate) ctx: &'a SerdeContext<'ctx>,
}

impl<'a, 'ctx, V: ?Sized + SerializeWithSerdeContext>
    ValueWithSerdeContext<'a, 'ctx, V>
{
    pub fn new(value: &'a V, ctx: &'a SerdeContext<'ctx>) -> Self {
        Self { value, ctx }
    }
}

impl<V: ?Sized + SerializeWithSerdeContext> Serialize
    for ValueWithSerdeContext<'_, '_, V>
{
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        self.value.serialize_with_ctx(self.ctx, serializer)
    }
}

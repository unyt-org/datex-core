use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use crate::dif::serde_context::SerdeContext;
use crate::dif::serialize_with_serde_context::SerializeWithSerdeContext;
use crate::values::core_values::native::NativeCoreValue;

impl SerializeWithSerdeContext for &NativeCoreValue {
    fn serialize_with_ctx<S: Serializer>(&self, ctx: &SerdeContext<'_>, serializer: S) -> Result<S::Ok, S::Error> {

        // serialize as {_: "c", c: Classification, v: Value} if classification is present, otherwise just serialize the value
        let classification = self.unresolved_classification();
        if classification.is_unclassified() {
            self.with_ctx(ctx).serialize(serializer)
        }
        else {
            let mut map_serializer = serializer.serialize_map(Some(3))?;
            // "_" key to indicate that this is a classified value
            map_serializer.serialize_entry("_", "c")?;
            // "c" key for classification
            map_serializer.serialize_entry("c", &classification)?;
            // "v" key for value
            map_serializer.serialize_entry("v", &self.with_ctx(ctx))?;
            map_serializer.end()
        }
    }
}
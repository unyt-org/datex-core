use crate::traits::convert_parts::{FromParts, HasPartsKind, IntoParts, PartsKind};
use crate::values::value::Value;
use crate::prelude::*;

pub trait ConvertValue {
    fn to_value(self) -> Value;

    fn try_from_value(value: Value) -> Result<Self, Value>
    where
        Self: Sized;

    fn try_borrow_from_value(value: &Value) -> Result<&Self, ()>;

    fn try_borrow_mut_from_value(value: &mut Value) -> Result<&mut Self, ()>;

    /// Tries to cast a [Value] into the specified type [T].
    /// First, it attempts a direct conversion (downcast) of the [Value] into the type [T].
    /// If that fails, it tries to convert the [Value] into map or list parts and then into the type [T].
    fn try_cast_from_value(
        value: Value,
    ) -> Result<Self, ()>
    where
        Self: FromParts + Sized,
    {
        // first try to convert (downcast) the value container directly into the type T
        let val = value.try_into_value::<Self>();
        match val {
            Ok(value) => Ok(value),
            // otherwise, try to convert the value container into map or list parts and then into the type T
            Err(value) => {
                let classification =
                    value.unresolved_classification();
                let tag = classification.tag_str();

                match value.parts_kind() {
                    PartsKind::Map => {
                        let map_parts =
                            Box::new(value).try_into_map_parts();
                        if let Ok(map) = map_parts {
                            Self::try_from_map_parts_with_tag(map, tag)
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::List => {
                        let list_parts =
                            Box::new(value).try_into_list_parts();
                        if let Ok(list) = list_parts {
                            Self::try_from_list_parts_with_tag(list, tag)
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::SingleValue => {
                        let single_value_parts =
                            Box::new(value).try_into_single_value();
                        if let Ok(single_value) = single_value_parts {
                            Self::try_from_single_value_with_tag(
                                single_value,
                                tag,
                            )
                        } else {
                            Err(())
                        }
                    }
                    PartsKind::None => Err(()),
                }
            }
        }
    }
}

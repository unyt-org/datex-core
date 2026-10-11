use crate::{
    traits::try_clone::TryClone,
    values::{
        core_value::CoreValue,
        core_values::integer::typed_integer::TypedInteger, value::Value,
    },
};

macro try_clone_integer($t:ty, $variant:ident) {
    impl TryClone for $t {
        fn try_clone(&self) -> Result<Value, ()> {
            Ok(
                CoreValue::TypedInteger(TypedInteger::$variant(self.clone()))
                    .into(),
            )
        }
    }
}

try_clone_integer!(i8, I8);
try_clone_integer!(i16, I16);
try_clone_integer!(i32, I32);
try_clone_integer!(i64, I64);
try_clone_integer!(i128, I128);

try_clone_integer!(u8, U8);
try_clone_integer!(u16, U16);
try_clone_integer!(u32, U32);
try_clone_integer!(u64, U64);
try_clone_integer!(u128, U128);

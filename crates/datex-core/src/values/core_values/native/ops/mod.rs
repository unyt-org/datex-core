use core::ops::Add;

use crate::values::core_values::native::NativeCoreValue;
mod add;
pub use add::*;

#[cfg(test)]
mod tests {
    use core::ops::Add;

    use crate::values::core_values::{
        integer::typed_integer::TypedInteger, native::NativeCoreValue,
    };

    #[test]
    fn add() {
        let a = NativeCoreValue::new(5u32);
        let b = NativeCoreValue::new(3u32);
        let result = (&a).add(&b).expect("Addition failed");
        assert_eq!(
            result.try_as::<u32>().expect("Failed to get result as u32"),
            NativeCoreValue::new(8u32)
                .try_as::<u32>()
                .expect("Failed to get expected value as u32")
        );

        let a = NativeCoreValue::new(TypedInteger::U32(10));
        let b = NativeCoreValue::new(TypedInteger::U32(10));
        let result = (&a).add(&b).expect("Addition failed");
        assert_eq!(
            result
                .try_as::<TypedInteger>()
                .expect("Failed to get result as u32"),
            &TypedInteger::U32(20)
        );
    }
}

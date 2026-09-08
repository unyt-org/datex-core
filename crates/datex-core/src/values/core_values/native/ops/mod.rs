use core::ops::Add;

use crate::values::core_values::native::NativeCoreValue;

impl Add for &NativeCoreValue {
    type Output = Option<NativeCoreValue>;

    fn add(self, rhs: Self) -> Self::Output {
        println!("rhs type_id: {:?}", rhs.as_any().type_id());

        println!("i8 type_id: {:?}", core::any::TypeId::of::<i8>());

        println!(
            "NativeCoreValue type_id: {:?}",
            core::any::TypeId::of::<NativeCoreValue>()
        );

        let value = self.value.add_native(&*rhs.value)?;
        Some(NativeCoreValue { value })
    }
}

impl Add for NativeCoreValue {
    type Output = Option<NativeCoreValue>;

    fn add(self, rhs: Self) -> Self::Output {
        (&self) + (&rhs)
    }
}

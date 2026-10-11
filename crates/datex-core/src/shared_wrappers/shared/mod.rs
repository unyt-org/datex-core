pub mod convert_value_container;
pub mod get_datex_type;
pub mod to_shared;

use crate::{
    prelude::*,
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        SharedContainer, SharedContainerMutability,
        traits::SharedContainerCommon,
    },
    values::{
        core_values::native::DatexNative, value::Value,
        value_container::ValueContainer,
    },
};
use core::{
    cell::{Ref, RefMut},
    ops::Deref,
};

pub struct Shared<T: DatexNative + ?Sized> {
    container: SharedContainer,
    _phantom_t: core::marker::PhantomData<T>,
}

impl<T: DatexNative> Shared<T> {
    pub fn to_container(self) -> SharedContainer {
        self.container
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        Ref::map(self.container.value_container(), |value_container| {
            match value_container {
                ValueContainer::Local(value) => {
                    // inner should always contain value castable to T
                    value.downcast_native_ref::<T>().unwrap()
                }
                // inner should never contain a shared value
                ValueContainer::Shared(_) => {
                    unreachable!()
                }
            }
        })
    }

    pub fn borrow_mut(&mut self) -> RefMut<'_, T> {
        RefMut::map(self.container.value_container_mut(), |value_container| {
            match value_container {
                ValueContainer::Local(value) => {
                    // inner should always contain value castable to T
                    value.downcast_native_mut::<T>().unwrap()
                }
                // inner should never contain a shared value
                ValueContainer::Shared(_) => {
                    unreachable!()
                }
            }
        })
    }
}

impl<T: DatexNative + 'static> Shared<T> {
    pub fn try_new(
        value: Box<T>,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Result<Self, ()> {
        let value_container = ValueContainer::from(Value::native(*value));
        Ok(Self {
            container: SharedContainer::new_owned_with_inferred_allowed_type(
                value_container,
                SharedContainerMutability::Mutable,
                address_provider,
            ),
            _phantom_t: core::marker::PhantomData,
        })
    }
}

impl<T: DatexNative + 'static> Shared<T> {
    pub fn new(
        value: T,
        address_provider: &mut SelfOwnedPointerAddressProvider,
    ) -> Self {
        let value_container = ValueContainer::from(Value::native(value));
        Self {
            container: SharedContainer::new_owned_with_inferred_allowed_type(
                value_container,
                SharedContainerMutability::Mutable,
                address_provider,
            ),
            _phantom_t: core::marker::PhantomData,
        }
    }
}

impl<T: DatexNative + 'static> TryFrom<SharedContainer> for Shared<T> {
    type Error = ();
    fn try_from(container: SharedContainer) -> Result<Self, Self::Error> {
        // TODO: also check if the type only allows values that are of type T.
        // check if the container contains a local value of type T
        match container.value_container().deref() {
            ValueContainer::Local(value) => {
                if value.downcast_native_ref::<T>().is_none() {
                    return Err(());
                }
            }
            ValueContainer::Shared(_) => {
                return Err(());
            }
        }
        Ok(Self {
            container,
            _phantom_t: core::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
        shared_values::{SharedContainer, SharedContainerMutability},
        shared_wrappers::shared::Shared,
    };

    use crate::{
        prelude::*,
        values::{core_value::CoreValue, value::Value},
    };

    #[test]
    fn string_shared() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        // TODO: also allow conversion to Shared<String> from CoreValue::Text
        let shared_container =
            SharedContainer::new_owned_with_inferred_allowed_type(
                Value::native("Hello DATEX".to_string()),
                SharedContainerMutability::Mutable,
                address_provider,
            );

        let shared_string: Shared<String> =
            Shared::try_from(shared_container).unwrap();
        assert_eq!(shared_string.borrow().as_str(), "Hello DATEX");
    }
}

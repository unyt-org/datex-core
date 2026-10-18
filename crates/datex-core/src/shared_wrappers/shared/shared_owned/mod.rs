mod get_datex_type;
mod convert_value_container;

use crate::{
    prelude::*,
    runtime::pointer_address_provider::SelfOwnedPointerAddressProvider,
    shared_values::{
        SharedContainerMutability,
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
use crate::shared_values::OwnedSharedContainer;

#[derive(Debug, PartialEq, Eq)]
pub struct Shared<T: DatexNative + ?Sized> {
    container: OwnedSharedContainer,
    _phantom_t: core::marker::PhantomData<T>,
}

impl<T: DatexNative> Shared<T> {
    pub fn into_container(self) -> OwnedSharedContainer {
        self.container
    }

    pub fn container(&self) -> &OwnedSharedContainer {
        &self.container
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        Ref::map(self.container.value_container(), |value_container| {
            // assumes T can always be borrowed from the value container, otherwise it will panic
            T::try_borrow_from_value_container(value_container).unwrap()
        })
    }

    pub fn borrow_mut(&mut self) -> RefMut<'_, T> {
        RefMut::map(self.container.value_container_mut(), |value_container| {
            // assumes T can always be borrowed from the value container, otherwise it will panic
            T::try_borrow_mut_from_value_container(value_container).unwrap()
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
            container: OwnedSharedContainer::new_with_inferred_allowed_type(
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
            container: OwnedSharedContainer::new_with_inferred_allowed_type(
                value_container,
                SharedContainerMutability::Mutable,
                address_provider,
            ),
            _phantom_t: core::marker::PhantomData,
        }
    }
}

impl<T: DatexNative + 'static> TryFrom<OwnedSharedContainer> for Shared<T> {
    type Error = OwnedSharedContainer;
    fn try_from(container: OwnedSharedContainer) -> Result<Self, Self::Error> {
        // TODO: also check if the type only allows values that are of type T.
        // check if the container contains a local value of type T
        if T::try_borrow_from_value_container(container.value_container().deref()).is_err() {
            return Err(container);
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
        shared_values::{SharedContainerMutability},
    };

    use crate::{
        prelude::*,
        values::{value::Value},
    };
    use crate::shared_values::OwnedSharedContainer;
    use crate::shared_wrappers::shared::shared_owned::Shared;

    #[test]
    fn string_shared() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let shared_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                Value::from("Hello DATEX".to_string()),
                SharedContainerMutability::Mutable,
                address_provider,
            );

        let mut shared_string: Shared<String> =
            Shared::try_from(shared_container).unwrap();
        assert_eq!(shared_string.borrow().as_str(), "Hello DATEX");

        *shared_string.borrow_mut() = "Hello DATEX!".to_string();
        assert_eq!(shared_string.borrow().as_str(), "Hello DATEX!");
    }
}
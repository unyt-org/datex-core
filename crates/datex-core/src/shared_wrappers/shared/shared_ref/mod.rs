mod get_datex_type;
mod convert_value_container;

use core::cell::{Ref, RefMut};
use core::ops::Deref;
use crate::runtime::pointer_address_provider::SelfOwnedPointerAddressProvider;
use crate::shared_values::{OwnedSharedContainer, ReferencedSharedContainer, SharedContainerMutability};
use crate::shared_values::traits::SharedContainerCommon;
use crate::values::core_values::native::DatexNative;
use crate::values::value::Value;
use crate::values::value_container::ValueContainer;

#[derive(Debug, PartialEq, Eq)]
pub struct SharedRef<T: DatexNative + ?Sized> {
    container: ReferencedSharedContainer,
    _phantom_t: core::marker::PhantomData<T>,
}

impl<T: DatexNative> SharedRef<T> {
    pub fn into_container(self) -> ReferencedSharedContainer {
        self.container
    }

    pub fn container(&self) -> &ReferencedSharedContainer {
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

impl<T: DatexNative + 'static> SharedRef<T> {
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
            ).derive_with_max_mutability(),
            _phantom_t: core::marker::PhantomData,
        })
    }
}

impl<T: DatexNative + 'static> SharedRef<T> {
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
            ).derive_with_max_mutability(),
            _phantom_t: core::marker::PhantomData,
        }
    }
}

impl<T: DatexNative + 'static> TryFrom<ReferencedSharedContainer> for SharedRef<T> {
    type Error = ReferencedSharedContainer;
    fn try_from(container: ReferencedSharedContainer) -> Result<Self, Self::Error> {
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


impl<T: DatexNative + 'static> TryFrom<OwnedSharedContainer> for SharedRef<T> {
    type Error = OwnedSharedContainer;
    fn try_from(container: OwnedSharedContainer) -> Result<Self, Self::Error> {
        // TODO: also check if the type only allows values that are of type T.
        // check if the container contains a local value of type T
        if T::try_borrow_from_value_container(container.value_container().deref()).is_err() {
            return Err(container);
        }

        Ok(Self {
            container: container.derive_with_max_mutability(),
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
    use crate::shared_wrappers::shared::shared_ref::SharedRef;

    #[test]
    fn string_shared() {
        let address_provider = &mut SelfOwnedPointerAddressProvider::default();

        let shared_container =
            OwnedSharedContainer::new_with_inferred_allowed_type(
                Value::from("Hello DATEX".to_string()),
                SharedContainerMutability::Mutable,
                address_provider,
            );

        let mut shared_string: SharedRef<String> =
            SharedRef::try_from(shared_container).unwrap();
        assert_eq!(shared_string.borrow().as_str(), "Hello DATEX");

        *shared_string.borrow_mut() = "Hello DATEX!".to_string();
        assert_eq!(shared_string.borrow().as_str(), "Hello DATEX!");
    }
}
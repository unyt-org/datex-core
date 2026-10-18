use crate::{
    prelude::*, runtime::Runtime, traits::get_datex_type::GetDatexType,
    values::value_container::ValueContainer,
};
use datex_macros_internal::{Datex, datex};

#[datex(name = "inspector")]
mod datex_inspector {
    use super::*;
    use crate::shared_wrappers::shared::{to_shared::ToShared};
    use crate::shared_wrappers::shared::shared_ref::SharedRef;

    #[derive(Datex, Debug, Clone, PartialEq, Eq, Hash)]
    #[datex(no_deserialize)]
    pub struct Inspector {
        // TODO: #[datex(readonly)]
        pub name: String, // TODO: distinguish between pub and private entity properties
    }

    #[datex]
    impl Inspector {
        pub fn name(&self) -> String {
            self.name.clone()
        }

        // // TODO
        // pub fn method_for_shared_instance(self: Shared<Self>) -> String {
        //     self.borrow().name.clone()
        // }
    }

    /// Creates a new [Inspector] instance.
    #[datex]
    pub fn create(name: String, #[runtime] runtime: &Runtime) -> SharedRef<Inspector> {
        Inspector { name }.shared_ref(runtime)
    }

    pub async fn async_test(a: String) -> String {
        format!("a = {}", a)
    }
}

/// Registers the `inspector` namespace in the runtime, allowing users to create Inspector instances.
pub fn register_inspector_namespace(runtime: &Runtime) {
    let mut memory = runtime.shared_references_cache_refcell().borrow_mut();
    let inspector_type = ValueContainer::from(
        datex_inspector::Inspector::datex_type(&mut memory),
    );

    runtime
        .endpoint_properties_mut()
        .insert("Inspector".to_string(), inspector_type);
}

#[cfg(test)]
mod tests {
    use core::ops::{DerefMut};
    use crate::{
        traits::apply::Apply, types::type_definition::callable::CallableKind,
        values::core_values::callable::native_sync_callable,
    };
    use crate::inspector::datex_inspector::Inspector;
    use crate::traits::apply::ApplyArgument;
    use crate::traits::convert_value_container::ConvertValueContainer;
    use crate::traits::value_access::ValueAccess;
    use crate::values::core_values::callable::Callable;

    #[test]
    fn ty() {
        let runtime = Runtime::stub();
        let inspector_type = Inspector::datex_type(runtime.shared_references_cache_mut().deref_mut());

        // create a new inspector instance via inspector.create()
        let inspector = datex_inspector::create("Test Inspector".to_string(), &runtime);
        let inspector_container = inspector.to_value_container();
        let inspector_container_clone = inspector_container.clone();

        // call inspector->name()
        let name_fn = inspector_type.try_get_property(&ValueContainer::from("name")).unwrap().try_as::<Callable>().unwrap();
        let (result, mut borrows) = name_fn.try_apply_sync_checked(&runtime, vec![ApplyArgument::referenced(inspector_container)]).unwrap();

        // return value should be "Test Inspector"
        assert_eq!(result.unwrap().try_as_str().unwrap(), "Test Inspector");
        // borrowed inspector should be given back to the caller
        assert_eq!(borrows.remove(0), inspector_container_clone);
    }

    use super::*;
    #[test]
    fn test_function() {
        let runtime = Runtime::stub();
        let memory = runtime.shared_references_cache_refcell();
        // 1 arg
        let func = |x: u8| x + 1;
        let dx_func_1 = ValueContainer::from(native_sync_callable(
            func,
            None,
            CallableKind::Function,
            &mut memory.borrow_mut(),
        ));
        let res = dx_func_1
            .try_apply_sync(&runtime, vec![4u8.into()])
            .unwrap()
            .0
            .expect("Function should return a value");
        assert_eq!(res.try_as::<u8>(), Some(&5u8));

        // 2 args
        let func_2 = |x: u8, y: u8| x + y;
        let dx_func_2 = ValueContainer::from(native_sync_callable(
            func_2,
            None,
            CallableKind::Function,
            &mut memory.borrow_mut(),
        ));
        let res_2 = dx_func_2
            .try_apply_sync(&runtime, vec![3u8.into(), 4u8.into()])
            .unwrap()
            .0
            .expect("Function should return a value");
        assert_eq!(res_2.try_as::<u8>(), Some(&7u8));
    }
}

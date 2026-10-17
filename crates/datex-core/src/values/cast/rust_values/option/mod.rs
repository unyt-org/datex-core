pub mod classification;
pub mod convert_parts;
mod convert_value;
mod datex_hash;
pub mod datex_native;
pub mod get_core_lib_type_id;
pub mod get_datex_type;
mod iter_parts;
mod serde_dif;
#[cfg(feature = "ast")]
mod to_datex_expression_data;
mod to_instructions;
mod value_access;

#[cfg(test)]
mod tests {
    use crate::{
        runtime::cache::shared_references_cache::SharedReferencesCache,
        traits::get_datex_type::GetDatexType,
        types::type_definition::TypeDefinition,
        values::{
            core_values::{
                endpoint::Endpoint, integer::Integer, native::NativeCoreValue,
            },
            value::Value,
        },
    };
    use crate::traits::try_clone::TryClone;

    #[test]
    fn to_value() {
        let some_option: Option<Integer> = Some(Integer::new(1));
        let none_option: Option<Integer> = None;

        let some_value: Value = Value::native(some_option);
        let none_value: Value = Value::native(none_option);

        // Option<T> is represented as T | Null, not as Option<T> itself,
        // so it cannot be borrowed as &Option<T>.
        assert!(some_value.try_as::<Option<Integer>>().is_none());
        assert!(none_value.try_as::<Option<Integer>>().is_none());
        assert!(some_value.try_as::<Integer>().is_some());
        assert!(none_value.try_as::<Integer>().is_none());
    }

    #[test]
    fn from_value() {
        // Convert native Integer to Some(Integer) using try_cast_into_value
        let normal_value = Value::native(Integer::new(1));
        let normal_some_option =
            normal_value.try_cast_into_value::<Option<Integer>>().unwrap();
        assert_eq!(normal_some_option, Some(Integer::new(1)));

        // Convert null to None using try_cast_into_value
        let null_value = Value::null();
        let null_none_option =
            null_value.try_cast_into_value::<Option<Integer>>().unwrap();
        assert_eq!(null_none_option, None);

        // Convert Some(Integer) to Some(Integer) using try_into_value
        let some_value: Value = Value::native(Some(Integer::new(1)));
        let some_option =
            some_value.try_into_value::<Option<Integer>>().unwrap();
        assert_eq!(some_option, Some(Integer::new(1)));

        // Convert None to None using try_into_value
        let none_value: Value = Value::native(None::<Integer>);
        let none_option =
            none_value.try_into_value::<Option<Integer>>().unwrap();
        assert_eq!(none_option, None);
    }
    #[test]
    fn datex_type() {
        let option_type = Option::<Integer>::datex_type(
            &mut SharedReferencesCache::default(),
        );
        option_type.with_collapsed_type_definition(|td| {
            assert!(matches!(td, TypeDefinition::Box(_)));
        });
    }

    #[test]
    fn endpoint_in_option() {
        let endpoint = Endpoint::new("@jonas");
        let boxed_endpoint = Some(endpoint.clone());
        let value: Value = Value::native(boxed_endpoint);
        assert_eq!(
            value,
            Value::Native(NativeCoreValue::new(Some(endpoint.clone())))
        );
        assert_eq!(
            value
                .try_clone()
                .unwrap()
                .try_cast_into_value::<Option<Endpoint>>()
                .expect("Expected Option<Endpoint>"),
            Some(endpoint.clone())
        );

        assert_eq!(
            value.try_cast_into_value::<Endpoint>().expect("Expected Endpoint"),
            endpoint
        );
    }
}

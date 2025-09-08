use std::assert_matches::assert_matches;

use datex_core::values::{
    core_value::CoreValue, value::Value, value_container::ValueContainer,
};
use datex_macros::DatexStruct;

#[derive(DatexStruct)]
pub struct SimpleTestStruct {
    pub my_bool: bool,
    pub my_number: i32,
    pub my_string: String,
}

#[test]
fn datex_struct_simple() {
    let my_struct = SimpleTestStruct {
        my_bool: true,
        my_number: 42,
        my_string: "Hello, Datex!".to_string(),
    };

    let value_container = my_struct.value_container();
    assert_matches!(
        value_container,
        ValueContainer::Value(Value {
            inner: CoreValue::Map(_),
            ..
        })
    );
    let value_container: ValueContainer = my_struct.into();
    assert_matches!(
        value_container,
        ValueContainer::Value(Value {
            inner: CoreValue::Map(_),
            ..
        })
    );
}

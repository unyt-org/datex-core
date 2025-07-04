use datex_core::values::value_container::ValueContainer;
use datex_macros::DatexStruct;

#[derive(DatexStruct)]
pub struct SimpleTestStruct {
    pub my_bool: bool,
    pub my_number: i32,
    pub my_string: String,
}

#[test]
fn serialize_simple() {
    let my_struct = SimpleTestStruct {
        my_bool: true,
        my_number: 42,
        my_string: "Hello, Datex!".to_string(),
    };

    let value_container = my_struct.value_container();
    let value_container: ValueContainer = my_struct.into();
    println!("Serialized Test: {:?}", value_container);
}

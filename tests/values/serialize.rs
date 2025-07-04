use datex_macros::DXSerialize;

#[derive(DXSerialize)]
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
    println!("Serialized Test: {:?}", value_container);
}

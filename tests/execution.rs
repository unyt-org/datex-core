/// End-to-end execution tests, including compiling values
/// and executing them, both locally and remotely.
use datex_core::compile;
use datex_core::runtime::execution::{execute_dxb_sync, ExecutionInput, ExecutionOptions};
use datex_core::values::core_values::integer::integer::Integer;
use datex_core::values::core_values::list::List;
use datex_core::values::core_values::map::Map;
use datex_core::values::value_container::ValueContainer;

fn compile_and_execute(input: ValueContainer) -> ValueContainer {
    let (dxb, _) = compile!("?", input.clone()).unwrap();

    execute_dxb_sync(ExecutionInput::new_with_dxb_and_options(
        &dxb,
        ExecutionOptions { verbose: true },
    )).unwrap().unwrap()
}

#[test]
fn test_compile_and_execute_integer() {
    let input = ValueContainer::from(Integer::from(42));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

// FIXME: preserve typed integer type in dxb
#[test]
fn test_compile_and_execute_typed_integer() {
    let input = ValueContainer::from(42u8);
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}


#[test]
fn test_compile_and_execute_string() {
    let input = ValueContainer::from("Hello, World!");
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

#[test]
fn test_compile_and_execute_bool() {
    let input = ValueContainer::from(true);
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

#[test]
fn test_compile_and_execute_list() {
    let input = ValueContainer::from(List::new(vec![
        ValueContainer::from(Integer::from(1)),
        ValueContainer::from(Integer::from(2)),
        ValueContainer::from(Integer::from(3)),
    ]));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

#[test]
fn test_compile_and_execute_map() {
    let input = ValueContainer::from(Map::from(vec![
        ("key1".to_string(), ValueContainer::from(Integer::from(1))),
        ("key2".to_string(), ValueContainer::from("value")),
        ("key3".to_string(), ValueContainer::from(true)),
    ]));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}
/// Runtime execution tests that validate that values are consistent through
/// the compile and execution process.
/// Any value passed as input should be returned exactly as it was passed in after compilation and execution.
use datex_core::compile;
use datex_core::runtime::execution::{
    ExecutionInput, ExecutionOptions, execute_dxb_sync,
};
use datex_core::values::core_values::decimal::Decimal;
use datex_core::values::core_values::decimal::typed_decimal::TypedDecimal;
use datex_core::values::core_values::integer::Integer;
use datex_core::values::core_values::integer::typed_integer::TypedInteger;
use datex_core::values::core_values::list::List;
use datex_core::values::core_values::map::Map;
use datex_core::values::value_container::ValueContainer;

fn compile_and_execute(input: ValueContainer) -> ValueContainer {
    let (dxb, _) = compile!("?", input.clone()).unwrap();

    execute_dxb_sync(ExecutionInput::new(
        &dxb,
        ExecutionOptions { verbose: true },
        None
    ))
    .unwrap()
    .unwrap()
}

#[test]
fn test_compile_and_execute_integer() {
    let input = ValueContainer::from(Integer::from(42));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

#[test]
fn test_compile_and_execute_typed_integer() {
    let input = ValueContainer::from(TypedInteger::U8(42));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);
}

#[test]
fn test_compile_and_execute_typed_decimals() {
    let input = ValueContainer::from(TypedDecimal::F32(42f32.into()));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);

    let input = ValueContainer::from(TypedDecimal::F32(f32::INFINITY.into()));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);

    let input = ValueContainer::from(TypedDecimal::F32(f32::NAN.into()));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);

    let input = ValueContainer::from(TypedDecimal::Decimal(Decimal::Infinity));
    let result = compile_and_execute(input.clone());
    assert_eq!(result, input);

    let input = ValueContainer::from(TypedDecimal::Decimal(Decimal::NaN));
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

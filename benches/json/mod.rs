use datex_core::ast::DatexScriptParser;
use datex_core::compiler::{
    CompileOptions, StaticValueOrDXB, compile_script,
    compile_script_or_return_static_value,
    extract_static_value_from_script,
};
use datex_core::decompiler::{DecompileOptions, decompile_body};
use datex_core::runtime::execution::{
    ExecutionInput, ExecutionOptions, execute_dxb_sync,
};
use datex_core::values::value_container::ValueContainer;
use json_syntax::Parse;
use serde_json::Value;
use std::io::Read;
use datex_core::core_compiler::value_compiler::compile_value_container;

pub fn get_json_test_string(file_path: &str) -> String {
    // read json from test file
    let file_path = format!("benches/json/{file_path}");
    let file_path = std::path::Path::new(&file_path);
    let file =
        std::fs::File::open(file_path).expect("Failed to open test.json");
    let mut reader = std::io::BufReader::new(file);
    let mut json_string = String::new();
    reader
        .read_to_string(&mut json_string)
        .expect("Failed to read test.json");
    json_string
}

pub fn json_to_serde_value(json: &str) -> Value {
    serde_json::from_str::<Value>(json).expect("Failed to parse JSON string")
}

pub fn json_to_json_syntax_value(json: &str) -> json_syntax::Value {
    let (json_value, _) =
        json_syntax::Value::parse_str(json).expect("Failed to parse JSON");
    json_value
}

pub fn json_to_datex_value(json: &str) -> ValueContainer {
    let (dxb, _) = compile_script(json, CompileOptions::default())
        .expect("Failed to parse JSON string");
    let exec_input = ExecutionInput::new_with_dxb_and_options(
        &dxb,
        ExecutionOptions::default(),
    );
    execute_dxb_sync(exec_input).unwrap().unwrap()
}

// json -> value
pub fn json_to_runtime_value_baseline_serde(json: &str) {
    let json_value = serde_json::from_str::<Value>(json)
        .expect("Failed to parse JSON string");
    assert!(json_value.is_object(), "Expected JSON to be an object");
}

pub fn json_to_runtime_value_baseline_json_syntax(json: &str) {
    let (json_value, _) =
        json_syntax::Value::parse_str(json).expect("Failed to parse JSON");
    assert!(json_value.is_object(), "Expected JSON to be an object");
}

pub fn json_to_runtime_value_datex<'a>(
    json: &'a str,
    parser: Option<&'a DatexScriptParser<'a>>,
) {
    let (dxb, _) = compile_script(
        json,
        CompileOptions {
            parser,
            ..CompileOptions::default()
        },
    )
    .expect("Failed to parse JSON string");
    let exec_input = ExecutionInput::new_with_dxb_and_options(
        &dxb,
        ExecutionOptions::default(),
    );
    let val = execute_dxb_sync(exec_input).unwrap().unwrap();
    assert!(val.to_value().borrow().is_map());
}

pub fn json_to_runtime_value_datex_auto_static_detection<'a>(
    json: &'a str,
    parser: Option<&'a DatexScriptParser<'a>>,
) -> ValueContainer {
    let (dxb, _) = compile_script_or_return_static_value(
        json,
        CompileOptions {
            parser,
            ..CompileOptions::default()
        },
    )
    .unwrap();
    if let StaticValueOrDXB::StaticValue(value) = dxb {
        value.expect("Static Value should not be empty")
    } else {
        core::panic!("Expected static value, but got DXB");
    }
}

pub fn json_to_runtime_value_datex_force_static_value(
    json: &str,
) -> ValueContainer {
    let dxb = extract_static_value_from_script(json).unwrap();
    dxb.expect("Static Value should not be empty")
}

pub fn json_to_dxb<'a>(
    json: &'a str,
    parser: Option<&'a DatexScriptParser<'a>>,
) {
    let (dxb, _) = compile_script(
        json,
        CompileOptions {
            parser,
            ..CompileOptions::default()
        },
    )
    .expect("Failed to parse JSON string");
    assert!(!dxb.is_empty(), "Expected DXB to be non-empty");
}

// DXB -> value
pub fn dxb_to_runtime_value(dxb: &[u8]) {
    let exec_input = ExecutionInput::new_with_dxb_and_options(
        dxb,
        ExecutionOptions::default(),
    );
    let json_value = execute_dxb_sync(exec_input).unwrap().unwrap();
    assert!(json_value.to_value().borrow().is_map());
}

// value -> JSON
pub fn runtime_value_to_json_baseline_serde_json(value: &Value) {
    let string =
        serde_json::to_string(value).expect("Failed to convert value to JSON");
    assert!(!string.is_empty(), "Expected JSON string to be non-empty");
}

pub fn runtime_value_to_json_baseline_json_syntax(value: &json_syntax::Value) {
    let string = value.to_string();
    assert!(
        !string.is_empty(),
        "Expected JSON syntax string to be non-empty"
    );
}

pub fn runtime_value_to_json_datex(value: &ValueContainer) {
    let dxb = compile_value_container(value);
    let string = decompile_body(&dxb, DecompileOptions::json()).unwrap();
    assert!(!string.is_empty(), "Expected DATEX string to be non-empty");
}

pub fn runtime_value_to_dxb(value: &ValueContainer) {
    let dxb = compile_value_container(value);
    assert!(!dxb.is_empty(), "Expected DXB to be non-empty");
}

pub fn dxb_to_json(dxb: &[u8]) {
    let string = decompile_body(dxb, DecompileOptions::json()).unwrap();
    assert!(!string.is_empty(), "Expected DATEX string to be non-empty");
}

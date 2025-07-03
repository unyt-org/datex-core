use datex_core::compiler::{compile_script, CompileOptions};
use datex_core::datex_values::core_value::CoreValue;
use datex_core::datex_values::core_values::decimal::decimal::Decimal;
use datex_core::datex_values::core_values::integer::integer::Integer;
use datex_core::datex_values::core_values::object::Object;
use datex_core::datex_values::value::Value;
use datex_core::datex_values::value_container::ValueContainer;
use datex_core::decompiler::{decompile_body, DecompileOptions};
use datex_core::runtime::execution::{
    execute_dxb, ExecutionInput, ExecutionOptions,
};
use itertools::Itertools;
use json_syntax::Parse;
use std::path::PathBuf;

fn json_value_to_datex_value(json: &json_syntax::Value) -> Value {
    match json {
        json_syntax::Value::Null => Value::null(),
        json_syntax::Value::String(s) => {
            Value::from(CoreValue::Text(s.to_string().into()))
        }
        json_syntax::Value::Number(n) => {
            let num_str = n.as_str();
            // num string only contains +, - and digits
            let is_integer = num_str
                .chars()
                .all(|c| c.is_ascii_digit() || c == '+' || c == '-');
            if is_integer {
                // Parse as integer
                let int_value = num_str.parse::<i128>().unwrap();
                Value::from(Integer::from(int_value))
            } else {
                // Parse as big decimal
                Value::from(Decimal::from_string(num_str))
            }
        }
        json_syntax::Value::Boolean(b) => Value::from(*b),
        json_syntax::Value::Array(arr) => {
            let mut vec = Vec::new();
            for value in arr {
                vec.push(json_value_to_datex_value(value));
            }
            Value::from(vec)
        }
        json_syntax::Value::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            for entry in obj {
                map.insert(
                    entry.key.to_string(),
                    ValueContainer::from(json_value_to_datex_value(
                        &entry.value.clone(),
                    )),
                );
            }
            Value::from(Object::from(map))
        }
    }
}

fn compare_datex_result_with_json(json_string: &str) {
    println!(" JSON String: {json_string}");
    let json_value = json_syntax::Value::parse_str(json_string).unwrap().0;
    let (dxb, _) =
        compile_script(json_string, CompileOptions::default()).unwrap();
    let exec_input = ExecutionInput::new_with_dxb_and_options(
        &dxb,
        ExecutionOptions {
            verbose: false,
            ..ExecutionOptions::default()
        },
    );
    let datex_value = execute_dxb(exec_input).unwrap().0.unwrap();
    let json_value_converted = json_value_to_datex_value(&json_value);

    println!(" JSON Value: {json_value}");
    println!(" DATEX Value: {datex_value}");
    println!(" Converted JSON Value: {json_value_converted}");

    assert_eq!(json_value_converted, *datex_value.to_value().borrow());
}

fn get_datex_decompiled_from_json(json_string: &str) -> String {
    let (dxb, _) =
        compile_script(json_string, CompileOptions::default()).unwrap();
    let decompiled = decompile_body(
        &dxb,
        DecompileOptions {
            json_compat: true,
            formatted: true,
            colorized: false,
            ..DecompileOptions::default()
        },
    )
    .unwrap();
    // try to parse JSON, if failed, panic
    let parsed_json = json_syntax::Value::parse_str(&decompiled);
    if parsed_json.is_err() {
        panic!("Decompiled JSON is not valid: {decompiled}");
    }
    decompiled
}

fn compare_datex_result_with_expected(
    json_string: &str,
    expected: &str,
    path: PathBuf,
) {
    let datex_decompiled = get_datex_decompiled_from_json(json_string);

    // println!(" Expected: {expected}");
    // println!(" Decompiled: {datex_decompiled}");
    assert_eq!(
        datex_decompiled,
        expected,
        "Decompiled output does not match expected output for file: {}",
        path.display()
    );
}

fn iterate_test_cases<'a>() -> impl Iterator<Item = (PathBuf, PathBuf)> + 'a {
    std::iter::from_coroutine(
        #[coroutine]
        move || {
            // read test cases from directory ./test_cases/<filename>.json
            let test_dir = std::path::Path::new("tests/json/test_cases");
            // go through directory files in alphabetical order
            for entry in std::fs::read_dir(test_dir)
                .unwrap()
                .map(|e| e.unwrap())
                .sorted_by_key(|e| e.path())
            {
                if entry.file_type().unwrap().is_file()
                    && entry.path().extension().unwrap() == "json"
                {
                    let input_path = entry.path();
                    // output path is ./expected_results/<filename>.json
                    let output_path =
                        std::path::PathBuf::from("tests/json/expected_results")
                            .join(entry.file_name());
                    yield (input_path, output_path);
                }
            }
        },
    )
}

#[test]
fn test_basic_json() {
    compare_datex_result_with_json("1");
    compare_datex_result_with_json("-42");
    compare_datex_result_with_json("true");
    compare_datex_result_with_json("false");
    compare_datex_result_with_json("null");
    compare_datex_result_with_json(r#""Hello World""#);
    compare_datex_result_with_json(r#""ölz1中文""#);
    compare_datex_result_with_json("1.23456789");
    compare_datex_result_with_json("5e3");
    compare_datex_result_with_json("-5e-3");
    compare_datex_result_with_json("1234567890");
    compare_datex_result_with_json("[]");
    compare_datex_result_with_json("{}");
    compare_datex_result_with_json(
        r#"{"key": "value", "number": 123, "boolean": true, "null_value": null}"#,
    );
    compare_datex_result_with_json(
        r#"{"array": [1, 2, 3], "object": {"key": "value"}}"#,
    );
}

#[test]
fn test_json_test_cases() {
    for (path, _) in iterate_test_cases() {
        println!("Testing JSON file: {}", path.display());
        let file_content = std::fs::read_to_string(path).unwrap();
        compare_datex_result_with_json(&file_content);
    }
}

#[test]
fn test_compare_with_expected() {
    for (input_path, output_path) in iterate_test_cases() {
        println!("Testing JSON file: {}", input_path.display());
        let file_content = std::fs::read_to_string(input_path.clone()).unwrap();
        let expected_content = std::fs::read_to_string(output_path).unwrap();
        compare_datex_result_with_expected(
            &file_content,
            &expected_content,
            input_path,
        );
    }
}

#[test]
#[ignore]
/// This test is used to update the expected results for the JSON test cases.
/// It will overwrite the expected results with the current decompiled output.
fn update_expected() {
    for (input_path, output_path) in iterate_test_cases() {
        // only update if output_path does not exist
        if output_path.exists() {
            println!(
                "Expected results already exist for: {}. Skipping update.",
                input_path.display()
            );
            continue;
        }
        println!("Updating expected results for: {}", input_path.display());
        let file_content = std::fs::read_to_string(input_path).unwrap();
        let decompiled = get_datex_decompiled_from_json(&file_content);
        std::fs::write(output_path.clone(), decompiled).unwrap();
        println!("Updated expected results for: {}", output_path.display());
    }
}

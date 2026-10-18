use datex_core::{
    core_compiler::{
        core_compilation_context::{CompileInput, CoreCompilationContext},
        to_instructions::ToInstructions,
    },
    disassembler::assertions::{
        assert_instruction_lists_eq, assert_instructions_equal, instructions,
    },
    instruction::{
        Instruction, instruction_data::TaggedValue,
        regular_instruction::RegularInstruction,
    },
    runtime::pointer_availability_lookup::PointerAvailabilityLookup,
    values::value::Value,
};
use datex_macros_internal::Datex;
#[derive(Datex, Debug)]
#[datex(structural)]
struct ExampleStruct {
    a: u8,
    b: String,
}
fn to_instructions<T: ToInstructions>(structure: &T) -> Vec<Instruction> {
    let pointer_lookup = PointerAvailabilityLookup::default();
    let mut context = CoreCompilationContext::new(
        CompileInput::new(&pointer_lookup, &[]),
    );
    structure.to_instructions(context.shared_value_tracking.get_mut()).collect::<Vec<_>>()
}

#[test]
fn example_struct_to_instructions() {
    let structure = ExampleStruct {
        a: 42u8,
        b: "Test".to_string(),
    };
    assert_instruction_lists_eq!(
        to_instructions(&structure),
        (RegularInstruction::map(2).with_children(instructions!(
            // a
            RegularInstruction::key_value_short_text("a".to_string()),
            RegularInstruction::uint8(42),
            // b
            RegularInstruction::key_value_short_text("b".to_string()),
            RegularInstruction::text("Test".to_string()),
        )))
    )
}

#[derive(Datex, Debug)]
#[datex(structural)]
struct StructWithValue {
    a: u8,
    value: Value,
}

#[test]
fn struct_with_value_to_instructions() {
    let structure = StructWithValue {
        a: 42u8,
        value: Value::from(123u8),
    };
    assert_instruction_lists_eq!(
        to_instructions(&structure),
        (RegularInstruction::map(2).with_children(instructions!(
            // a
            RegularInstruction::key_value_short_text("a".to_string()),
            RegularInstruction::uint8(42),
            // value
            RegularInstruction::key_value_short_text("value".to_string()),
            RegularInstruction::uint8(123),
        )))
    );
}

#[derive(Datex, Debug)]
#[datex(structural)]
enum ExampleEnum {
    Variant1(u8),
    Variant2 { x: u8, y: String },
    Variant3(u8, String),
    Variant4,
}

#[test]
fn example_enum_variant_1_to_instructions() {
    let variant1 = ExampleEnum::Variant1(42);
    assert_instruction_lists_eq!(
        to_instructions(&variant1),
        (RegularInstruction::tagged_value("Variant1".to_string(), false)
            .with_children(instructions!(RegularInstruction::uint8(42),)))
    );
}

#[test]
fn example_enum_variant_2_to_instructions() {
    let variant2 = ExampleEnum::Variant2 {
        x: 42,
        y: "Test".to_string(),
    };
    assert_instruction_lists_eq!(
        to_instructions(&variant2),
        (RegularInstruction::tagged_value("Variant2".to_string(), false)
            .with_children(instructions!(
                RegularInstruction::map(2).with_children(instructions!(
                    // x
                    RegularInstruction::key_value_short_text("x".to_string()),
                    RegularInstruction::uint8(42),
                    // y
                    RegularInstruction::key_value_short_text("y".to_string()),
                    RegularInstruction::text("Test".to_string()),
                ))
            )))
    );
}

#[test]
fn example_enum_variant_3_to_instructions() {
    let variant3 = ExampleEnum::Variant3(42, "Test".to_string());
    assert_instruction_lists_eq!(
        to_instructions(&variant3),
        (RegularInstruction::tagged_value("Variant3".to_string(), false)
            .with_children(instructions!(
                RegularInstruction::list_with_children(instructions!(
                    RegularInstruction::uint8(42),
                    RegularInstruction::text("Test".to_string()),
                ))
            )))
    );
}

#[test]
fn example_enum_variant_4_to_instructions() {
    let variant4 = ExampleEnum::Variant4;
    assert_instruction_lists_eq!(
        to_instructions(&variant4),
        vec![RegularInstruction::tagged_value(
            "Variant4".to_string(),
            true
        )]
    );
}

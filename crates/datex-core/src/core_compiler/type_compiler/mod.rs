use crate::{
    core_compiler::core_compilation_context::ByteCursor,
    instruction::type_instruction::TypeInstruction,
};
use binrw::BinWrite;

pub mod type_to_instructions;

pub mod type_definition_to_instructions;

#[deprecated(note = "Use ToInstructions trait instead")]
pub fn append_type_instruction(
    cursor: &mut ByteCursor,
    instruction: TypeInstruction,
) {
    // add instruction
    instruction.write(cursor).unwrap();
}

#[cfg(test)]
#[cfg(feature = "disassembler")]
mod tests {
    use binrw::BinRead;

    use crate::{
        core_compiler::{
            core_compilation_context::{
                ByteCursor, default_compile_input,
                default_core_compilation_context,
            },
            shared_value_tracking::SharedValueTracking,
            to_instructions::ToInstructions,
            value_compiler::compile_value,
        },
        disassembler::assertions::assert_instructions_equal,
        instruction::{
            Instruction, regular_instruction::RegularInstruction,
            type_instruction::TypeInstruction,
        },
        libs::core::type_id::{CoreLibBaseTypeId, CoreLibTypeId},
        prelude::*,
        types::{
            r#type::Type,
            type_definition::TypeDefinition,
            type_definition_with_metadata::{
                LocalMutability, LocalOwnership, TypeDefinitionWithMetadata,
                TypeMetadata,
            },
        },
        values::{core_value::CoreValue, value::Value},
    };

    fn assert_type_instructions(
        ty: Type,
        expected_instruction: Vec<TypeInstruction>,
    ) {
        let compile_input = unsafe { default_compile_input() };
        let vec =
            vec![Instruction::Regular(RegularInstruction::type_expression())]
                .into_iter()
                .chain(expected_instruction.into_iter().map(Instruction::Type))
                .collect::<Vec<_>>();

        let compiled =
            compile_value(Value::new(CoreValue::Type(ty), None), compile_input);
        assert_eq!(compiled.shared_values.len(), 0);
        assert_instructions_equal!(&compiled.dxb, vec)
    }

    fn assert_regular_instructions_equal(
        val: Value,
        expected_instructions: Vec<RegularInstruction>,
    ) {
        let compile_input = unsafe { default_compile_input() };
        let compiled = compile_value(val, compile_input);
        let mut cursor = ByteCursor::new(compiled.dxb.to_vec());
        for expected in expected_instructions {
            let instruction = RegularInstruction::read(&mut cursor)
                .expect("Failed to read instruction from compiled bytecode");
            assert_eq!(instruction, expected);
        }
    }

    #[test]
    fn type_definition_with_metadata() {
        let ty = Type::Definition(TypeDefinitionWithMetadata::new(
            TypeDefinition::CoreType(CoreLibTypeId::Base(
                CoreLibBaseTypeId::Boolean,
            )),
            TypeMetadata::Local {
                mutability: LocalMutability::Mutable,
                ownership: LocalOwnership::Owned,
            },
        ));
        assert_type_instructions(
            ty,
            vec![
                TypeInstruction::DefinitionWithMetadata(TypeMetadata::Local {
                    mutability: LocalMutability::Mutable,
                    ownership: LocalOwnership::Owned,
                }),
                TypeInstruction::CoreType(CoreLibTypeId::Base(
                    CoreLibBaseTypeId::Boolean,
                )),
            ],
        );
    }

    #[test]
    fn core_type() {
        // We shortcut the type compilation for aliased core types, that don't have any metadata or a custom type
        // to just directly return the core lib value instruction, since the execution will treat them as the same type anyway
        let ty = Type::Definition(
            TypeDefinition::CoreType(CoreLibTypeId::Base(
                CoreLibBaseTypeId::Boolean,
            ))
            .into(),
        );
        assert_regular_instructions_equal(
            Value::new(CoreValue::Type(ty), None),
            vec![RegularInstruction::GetCoreLibValue(
                CoreLibTypeId::Base(CoreLibBaseTypeId::Boolean).into(),
            )],
        );
    }
}

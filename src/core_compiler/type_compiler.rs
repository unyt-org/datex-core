use datex_core::global::type_instruction_codes::TypeSpaceInstructionCode;
use datex_core::types::definition::TypeDefinition;
use crate::core_compiler::value_compiler::append_get_ref;
use crate::global::type_instruction_codes::TypeMutabilityCode;
use crate::values::core_values::r#type::Type;
use crate::stdlib::vec::Vec;
use crate::utils::buffers::append_u8;

/// Compiles a given type container to a DXB body
pub fn compile_type(ty: &Type) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(256);
    append_type(&mut buffer, ty);

    buffer
}

fn append_type(buffer: &mut Vec<u8>, ty: &Type) {

    // append instruction code
    let instruction_code = TypeSpaceInstructionCode::from(&ty.type_definition);
    append_type_space_instruction_code(buffer, instruction_code);

    // append mutability
    let mutability_code = TypeMutabilityCode::from(&ty.reference_mutability);
    append_type_mutability_code(buffer, mutability_code);

    // append type definition
    append_type_definition(buffer, &ty.type_definition);
}

fn append_type_definition(buffer: &mut Vec<u8>, type_definition: &TypeDefinition) {
    match type_definition {
        TypeDefinition::ImplType(ty, impls) => {
            // Append the number of impls
            let impl_count = impls.len() as u8;
            append_u8(buffer, impl_count);

            // Append each impl address
            for impl_type in impls {
                append_get_ref(buffer, impl_type);
            }

            // Append the base type
            append_type(buffer, ty);
        }
        TypeDefinition::Reference(type_ref) => {
            // TODO: ensure pointer_address exists here
            let type_ref = type_ref.borrow();
            let pointer_address = type_ref.pointer_address.as_ref().expect("Type reference must have a pointer address");
            append_get_ref(buffer, pointer_address);
        }
        _ => todo!("Type definition compilation not implemented yet"),
    };
}


pub fn append_type_space_instruction_code(buffer: &mut Vec<u8>, code: TypeSpaceInstructionCode) {
    append_u8(buffer, code as u8);
}

pub fn append_type_mutability_code(buffer: &mut Vec<u8>, code: TypeMutabilityCode) {
    append_u8(buffer, code as u8);
}
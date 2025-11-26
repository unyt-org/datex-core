use crate::values::core_values::r#type::Type;
/// Compiles a given type container to a DXB body
pub fn compile_type_container(ty: &Type) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(256);
    append_type_container(&mut buffer, ty);

    buffer
}

/***

$123123123
"txt"
$x = ()

 */

pub fn append_type_container(
    buffer: &mut Vec<u8>,
    ty: &Type,
) {
    match ty {
        // TypeContainer::Type(ty) => append_type(buffer, ty),
        // // nominal type (e.g. integer, User)
        // TypeContainer::TypeReference(reference) => {
            // // add CREATE_REF/CREATE_REF_MUT instruction
            // if reference.mutability() == ReferenceMutability::Mutable {
            //     append_instruction_code(
            //         buffer,
            //         InstructionCode::CREATE_REF_MUT,
            //     );
            // } else {
            //     append_instruction_code(buffer, InstructionCode::CREATE_REF);
            // }
            // // insert pointer id + value or only id
            // // add pointer to memory if not there yet
            // append_value(buffer, &reference.collapse_to_value().borrow())
        // }
        _ => unreachable!()
    }
}

pub fn append_type(buffer: &mut Vec<u8>, ty: &Type) {

    // INSTRUCTION
    // &mut (integer + js.undefed)
    // function (x: null) {
    //
    // }
    //
    // match &ty.type_definition {
    //     TypeDefinition::MarkedType(ty) => {
    //         // ...
    //     }
    //     _ => todo!()
    // }
}

// pub fn get_type_definition_
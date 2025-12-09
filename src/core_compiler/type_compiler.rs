use crate::values::core_values::r#type::Type;
/// Compiles a given type container to a DXB body
pub fn compile_type(ty: &Type) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(256);
    append_type(&mut buffer, ty);

    buffer
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
    todo!()
}

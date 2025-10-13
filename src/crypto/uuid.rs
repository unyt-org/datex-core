use crate::runtime::global_context::get_global_context;

pub fn generate_uuid() -> String {
    let crypto = get_global_context().crypto;
    crypto.create_uuid()
}

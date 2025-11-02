use crate::runtime::global_context::get_global_context;
use crate::stdlib::string::String;

pub fn generate_uuid() -> String {
    let crypto = get_global_context().crypto;
    crypto.create_uuid()
}

use super::{Value, ValueResult};
use crate::stdlib::fmt;
use crate::{
    global::binary_codes::BinaryCode, utils::buffers::buffer_to_hex_advanced,
};

pub struct Pointer {
    pub id_formatted: String,
}

impl Pointer {
    pub const MAX_POINTER_ID_SIZE: usize = 26;
    pub const STATIC_POINTER_SIZE: usize = 18;

    pub fn from_id(id: Vec<u8>) -> Pointer {
        Pointer {
            id_formatted: buffer_to_hex_advanced(id, "", 0, true),
        }
    }
}

impl Value for Pointer {
    fn to_string(&self) -> String {
        format!("${}", self.id_formatted)
    }

    fn binary_operation(
        &self,
        _code: BinaryCode,
        _other: Box<dyn Value>,
    ) -> ValueResult {
        todo!()
    }

    fn cast(&self, _dx_type: super::Type) -> ValueResult {
        todo!()
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}

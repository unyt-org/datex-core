use std::fmt;

use crate::global::binary_codes::BinaryCode;

use super::{Value, ValueResult};

pub struct Type {
    pub namespace: String,
    pub name: String,
    pub variation: Option<String>,
}

impl Value for Type {
    fn to_string(&self) -> String {
        if self.namespace.len() == 0 || self.namespace == "std" {
            if self.variation.is_some() {
                return format!("<{}/{}>", self.name, self.variation.as_ref().unwrap());
            } else {
                return format!("<{}>", self.name);
            }
        } else {
            if self.variation.is_some() {
                return format!(
                    "<{}:{}/{}>",
                    self.namespace,
                    self.name,
                    self.variation.as_ref().unwrap()
                );
            } else {
                return format!("<{}:{}>", self.namespace, self.name);
            }
        }
    }

    fn binary_operation(&self, _code: BinaryCode, _other: Box<dyn Value>) -> ValueResult {
        todo!()
    }

    fn cast(&self, _dx_type: Type) -> ValueResult {
        todo!()
    }
}

pub mod std_types {
    use super::Type;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref SET: Type = Type {
            namespace: "".to_string(),
            name: "Set".to_string(),
            variation: None
        };
        pub static ref MAP: Type = Type {
            namespace: "".to_string(),
            name: "Map".to_string(),
            variation: None
        };
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Value::to_string(self))
    }
}

pub trait DatexTypedStruct {
    fn get_type() -> String;
}

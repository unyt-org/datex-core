use core::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Primitive {
    I8(i8),
    U8(u8),
}

impl Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Primitive::I8(v) => write!(f, "{}", v),
            Primitive::U8(v) => write!(f, "{}", v),
        }
    }
}

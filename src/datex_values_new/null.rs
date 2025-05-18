use core::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub struct Null;

impl Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "null")
    }
}

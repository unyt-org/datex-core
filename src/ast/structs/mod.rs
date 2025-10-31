use core::fmt::Display;

use crate::values::pointer::PointerAddress;

pub mod expression;
pub mod operator;
pub mod r#type;

pub type VariableId = usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedVariable {
    VariableId(usize),
    PointerAddress(PointerAddress),
}

impl Display for ResolvedVariable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResolvedVariable::VariableId(id) => write!(f, "#{}", id),
            ResolvedVariable::PointerAddress(addr) => write!(f, "{}", addr),
        }
    }
}

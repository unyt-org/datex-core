use core::fmt::Display;
use crate::runtime::execution::ExecutionError;
use crate::traits::apply::Apply;
use crate::traits::structural_eq::StructuralEq;
use crate::values::core_values::r#type::Type;
use crate::values::value_container::ValueContainer;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallableKind {
    // A pure function
    Function,
    // A procedure that may have side effects
    Procedure
}

impl Display for CallableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallableKind::Function => write!(f, "function"),
            CallableKind::Procedure => write!(f, "procedure"),
        }
    }
}

pub type NativeCallable = fn(&[ValueContainer]) -> Result<Option<ValueContainer>, ExecutionError>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallableBody {
    Native(NativeCallable),
    DatexBytecode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallableSignature {
    pub kind: CallableKind,
    pub parameter_types: Vec<(Option<String>, Type)>,
    pub rest_parameter_type: Option<(Option<String>, Box<Type>)>,
    pub return_type: Option<Box<Type>>,
    pub yeet_type: Option<Box<Type>>,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Callable {
    pub name: Option<String>,
    pub signature: CallableSignature,
    pub body: CallableBody,
}

impl Callable {
    pub fn call(&self, args: &[ValueContainer]) -> Result<Option<ValueContainer>, ExecutionError> {
        match &self.body {
            CallableBody::Native(func) => func(args),
            CallableBody::DatexBytecode => {
                todo!("Calling Datex bytecode is not yet implemented")
            }
        }
    }
}

impl Apply for Callable {
    fn apply(&self, args: &[ValueContainer]) -> Result<Option<ValueContainer>, ExecutionError> {
        self.call(args)
    }
    fn apply_single(&self, arg: &ValueContainer) -> Result<Option<ValueContainer>, ExecutionError> {
        self.call(&[arg.clone()])
    }
}

impl StructuralEq for Callable {
    fn structural_eq(&self, other: &Self) -> bool {
        self == other
    }
}

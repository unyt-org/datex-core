use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum UnaryOperator {
    Reference(ReferenceUnaryOperator),
    Arithmetic(ArithmeticUnaryOperator),
    Bitwise(BitwiseUnaryOperator),
    Logical(LogicalUnaryOperator),
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            UnaryOperator::Reference(op) => write!(f, "{}", op),
            UnaryOperator::Arithmetic(op) => write!(f, "{}", op),
            UnaryOperator::Bitwise(op) => write!(f, "{}", op),
            UnaryOperator::Logical(op) => write!(f, "{}", op),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ReferenceUnaryOperator {
    CreateRef,      // &
    CreateRefMut,   // &mut
    CreateRefFinal, // &final
    Deref,          // *
}

impl Display for ReferenceUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ReferenceUnaryOperator::CreateRef => write!(f, "&"),
            ReferenceUnaryOperator::CreateRefMut => write!(f, "&mut"),
            ReferenceUnaryOperator::CreateRefFinal => write!(f, "&final"),
            ReferenceUnaryOperator::Deref => write!(f, "*"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ArithmeticUnaryOperator {
    Increment, // ++
    Decrement, // --
    Plus,      // +
    Minus,     // -
}

impl Display for ArithmeticUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ArithmeticUnaryOperator::Increment => write!(f, "++"),
            ArithmeticUnaryOperator::Decrement => write!(f, "--"),
            ArithmeticUnaryOperator::Plus => write!(f, "+"),
            ArithmeticUnaryOperator::Minus => write!(f, "-"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum BitwiseUnaryOperator {
    Negation, // ~
}

impl Display for BitwiseUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            BitwiseUnaryOperator::Negation => write!(f, "~"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum LogicalUnaryOperator {
    Not, // !
}
impl Display for LogicalUnaryOperator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            LogicalUnaryOperator::Not => write!(f, "!"),
        }
    }
}

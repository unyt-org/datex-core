pub use crate::global::operators::assignment::AssignmentOperator;

pub mod binary;
pub use binary::BinaryOperator;

pub mod comparison;
pub use comparison::ComparisonOperator;

pub mod assignment;
pub mod unary;

pub use unary::{
    ArithmeticUnaryOperator, BitwiseUnaryOperator, LogicalUnaryOperator,
    ReferenceUnaryOperator, UnaryOperator,
};

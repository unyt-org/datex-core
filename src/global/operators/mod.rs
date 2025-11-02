pub use crate::global::operators::assignment::AssignmentOperator;

pub mod binary;
pub use binary::BinaryOperator;

pub mod apply;
pub use apply::ApplyOperation;

pub mod comparison;
pub use comparison::ComparisonOperator;

pub mod unary;
pub mod assignment;

pub use unary::{
    ArithmeticUnaryOperator, BitwiseUnaryOperator, LogicalUnaryOperator,
    ReferenceUnaryOperator, UnaryOperator,
};

use crate::ast::TokenInput;
use crate::ast::utils::whitespace;
use crate::compiler::lexer::Token;
use crate::global::binary_codes::InstructionCode;
use chumsky::extra::Err;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum AssignmentOperator {
    Assign,          // =
    AddAssign,       // +=
    SubstractAssign, // -=
    MultiplyAssign,  // *=
    DivideAssign,    // /=
}

impl From<&AssignmentOperator> for InstructionCode {
    fn from(op: &AssignmentOperator) -> Self {
        match op {
            AssignmentOperator::Assign => InstructionCode::ASSIGN,
            AssignmentOperator::AddAssign => InstructionCode::ADD_ASSIGN,
            AssignmentOperator::SubstractAssign => {
                InstructionCode::SUBTRACT_ASSIGN
            }
            AssignmentOperator::MultiplyAssign => {
                InstructionCode::MULTIPLY_ASSIGN
            }
            AssignmentOperator::DivideAssign => InstructionCode::DIVIDE_ASSIGN,
            operator => todo!(
                "Assignment operator {:?} not implemented for InstructionCode",
                operator
            ),
        }
    }
}

pub fn assignment_operation<'a>()
-> impl Parser<'a, TokenInput<'a>, AssignmentOperator, Err<Cheap>> + Clone + 'a
{
    select! {
        Token::Assign      => AssignmentOperator::Assign,
        Token::AddAssign   => AssignmentOperator::AddAssign,
        Token::SubAssign   => AssignmentOperator::SubstractAssign,
        Token::MulAssign   => AssignmentOperator::MultiplyAssign,
        Token::DivAssign   => AssignmentOperator::DivideAssign,
    }
    .padded_by(whitespace())
}

use crate::ast::DatexParserTrait;
use crate::ast::lexer::Token;
use crate::global::operators::AssignmentOperator;
use chumsky::prelude::*;

pub fn assignment_operation<'a>()
-> impl DatexParserTrait<'a, AssignmentOperator> {
    select! {
        Token::Assign      => AssignmentOperator::Assign,
        Token::AddAssign   => AssignmentOperator::AddAssign,
        Token::SubAssign   => AssignmentOperator::SubtractAssign,
        Token::MulAssign   => AssignmentOperator::MultiplyAssign,
        Token::DivAssign   => AssignmentOperator::DivideAssign,
        Token::ModAssign   => AssignmentOperator::ModuloAssign,
    }
}

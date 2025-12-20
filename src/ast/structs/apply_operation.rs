use crate::ast::structs::expression::DatexExpression;

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyOperation {
    /// Apply a function to an argument
    FunctionCallSingleArgument(DatexExpression),

    // TODO #356: Implement MultiFunctionCall(Vec<DatexExpression>),
    /// Apply a property access to an argument
    PropertyAccess(DatexExpression),

    /// Generic property access, e.g. `a<b>`
    GenericAccess(DatexExpression),
}

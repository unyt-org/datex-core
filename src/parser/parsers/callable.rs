use crate::ast::expressions::{
    CallableDeclaration, CallableKind, DatexExpression, DatexExpressionData,
};
use crate::ast::spanned::Spanned;
use crate::ast::type_expressions::TypeExpression;
use crate::parser::lexer::Token;
use crate::parser::{Parser, SpannedParserError};

impl Parser {
    pub(crate) fn parse_callable_definition(
        &mut self,
    ) -> Result<DatexExpression, SpannedParserError> {
        let start_pos = self.get_current_source_position();

        // first token must be Function or Procedure
        let kind = match self.advance()?.token {
            Token::Function => CallableKind::Function,
            Token::Procedure => CallableKind::Procedure,
            _ => unreachable!(),
        };

        // next token must be identifier
        let (name, _) = self.expect_identifier()?;

        // parse parameters
        let parameters = self.parse_callable_parameters()?;

        // parse return type if next token is "->"
        let return_type = if let Ok(token) = self.peek()
            && token.token == Token::Arrow
        {
            self.advance()?;
            Some(self.parse_type_expression(0)?)
        } else {
            None
        };

        // TODO: add yeets

        // parse function body
        let body = self.parse_parenthesized_statements()?;
        Ok(
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name,
                kind,
                parameters,
                return_type,
                body: Box::new(body),
            })
            .with_span(start_pos..self.get_current_source_position()),
        )
    }

    fn parse_callable_parameters(
        &mut self,
    ) -> Result<Vec<(String, TypeExpression)>, SpannedParserError> {
        let mut parameters = Vec::new();
        self.expect(Token::LeftParen)?;

        while self.peek()?.token != Token::RightParen {
            // parse parameter name
            let (param_name, _) = self.expect_identifier()?;

            // expect colon
            self.expect(Token::Colon)?;

            // parse parameter type
            let param_type = self.parse_type_expression(0)?;
            parameters.push((param_name, param_type));

            // if next token is comma, consume it
            if self.peek()?.token == Token::Comma {
                self.advance()?;
            }
        }

        self.expect(Token::RightParen)?;
        Ok(parameters)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::expressions::{
        BinaryOperation, CallableDeclaration, CallableKind,
        DatexExpressionData, Statements,
    };
    use crate::ast::spanned::Spanned;
    use crate::ast::type_expressions::TypeExpressionData;
    use crate::global::operators::BinaryOperator;
    use crate::global::operators::binary::ArithmeticOperator;
    use crate::parser::tests::parse;

    #[test]
    fn parse_empty_function() {
        let expr = parse("function test() ()");
        assert_eq!(
            expr.data,
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name: "test".to_string(),
                kind: CallableKind::Function,
                parameters: vec![],
                return_type: None,
                body: Box::new(
                    DatexExpressionData::Statements(Statements {
                        statements: vec![],
                        is_terminated: false,
                        unbounded: None,
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_empty_procedure() {
        let expr = parse("procedure doSomething() ()");
        assert_eq!(
            expr.data,
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name: "doSomething".to_string(),
                kind: CallableKind::Procedure,
                parameters: vec![],
                return_type: None,
                body: Box::new(
                    DatexExpressionData::Statements(Statements {
                        statements: vec![],
                        is_terminated: false,
                        unbounded: None,
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_function_with_parameters_and_return_type() {
        let expr = parse("function add(a: integer, b: integer) -> integer ( )");
        assert_eq!(
            expr.data,
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name: "add".to_string(),
                kind: CallableKind::Function,
                parameters: vec![
                    (
                        "a".to_string(),
                        TypeExpressionData::Identifier("integer".to_string())
                            .with_default_span()
                    ),
                    (
                        "b".to_string(),
                        TypeExpressionData::Identifier("integer".to_string())
                            .with_default_span()
                    ),
                ],
                return_type: Some(
                    TypeExpressionData::Identifier("integer".to_string())
                        .with_default_span()
                ),
                body: Box::new(
                    DatexExpressionData::Statements(Statements {
                        statements: vec![],
                        is_terminated: false,
                        unbounded: None,
                    })
                    .with_default_span()
                ),
            })
        );
    }

    #[test]
    fn parse_function_with_parameters_and_body() {
        let expr =
            parse("function greet(name: text) -> text ( \"Hello, \" + name )");
        assert_eq!(
            expr.data,
            DatexExpressionData::CallableDeclaration(CallableDeclaration {
                name: "greet".to_string(),
                kind: CallableKind::Function,
                parameters: vec![(
                    "name".to_string(),
                    TypeExpressionData::Identifier("text".to_string())
                        .with_default_span()
                ),],
                return_type: Some(
                    TypeExpressionData::Identifier("text".to_string())
                        .with_default_span()
                ),
                body: Box::new(
                    DatexExpressionData::BinaryOperation(BinaryOperation {
                        left: Box::new(
                            DatexExpressionData::Text("Hello, ".to_string())
                                .with_default_span()
                        ),
                        operator: BinaryOperator::Arithmetic(
                            ArithmeticOperator::Add
                        ),
                        right: Box::new(
                            DatexExpressionData::Identifier("name".to_string())
                                .with_default_span()
                        ),
                        ty: None,
                    })
                    .with_default_span(),
                )
            })
        );
    }
}

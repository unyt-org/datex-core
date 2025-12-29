use datex_core::ast::spanned::Spanned;
use datex_core::ast::structs::expression::DatexExpression;
use crate::ast::lexer::{SpannedToken, Token};
use crate::ast::structs::expression::{DatexExpressionData, List};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {

    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> DatexExpression {
        self.parse_value()
    }

    fn peek(&self) -> &SpannedToken {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> SpannedToken {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: Token) {
        if self.advance().token != kind {
            panic!("Expected {:?}", kind);
        }
    }


    fn parse_value(&mut self) -> DatexExpression {
        match self.peek().token {
            Token::LeftCurly => todo!(),
            Token::LeftBracket => self.parse_list(),
            Token::True => {
                let span = self.advance().span;
                DatexExpressionData::Boolean(true).with_span(span)
            }
            Token::False => {
                let span = self.advance().span;
                DatexExpressionData::Boolean(false).with_span(span)
            }
            Token::Null => {
                let span = self.advance().span;
                DatexExpressionData::Null.with_span(span)
            }
            _ => self.parse_expr(0),
        }
    }

    fn parse_list(&mut self) -> DatexExpression {
        self.expect(Token::LeftBracket);
        let mut items = Vec::new();

        while self.peek().token != Token::RightBracket {
            items.push(self.parse_value());

            if self.peek().token == Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RightBracket);
        DatexExpressionData::List(List {
            items
        }).with_default_span()
    }

    fn parse_expr(&mut self, min_bp: u8) -> DatexExpression {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    use crate::ast::lexer::get_spanned_tokens_from_source;
    use super::*;

    fn parse(src: &str) -> DatexExpression {
        let tokens = get_spanned_tokens_from_source(src).unwrap();
        println!("{:?}", tokens);
        let mut parser = Parser::new(tokens);
        parser.parse()
    }


    #[test]
    fn parse_boolean_true() {
        let expr = parse("true");
        assert_eq!(expr.data, DatexExpressionData::Boolean(true));
    }

    #[test]
    fn parse_boolean_false() {
        let expr = parse("false");
        assert_eq!(expr.data, DatexExpressionData::Boolean(false));
    }

    #[test]
    fn parse_simple_list() {
        let expr = parse("[true, false, null]");
        assert_eq!(expr.data, DatexExpressionData::List(List {
            items: vec![
                DatexExpressionData::Boolean(true).with_default_span(),
                DatexExpressionData::Boolean(false).with_default_span(),
                DatexExpressionData::Null.with_default_span(),
            ]
        }));
    }
}

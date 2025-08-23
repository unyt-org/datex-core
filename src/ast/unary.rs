use crate::ast::utils::whitespace;
use crate::compiler::ast_parser::DatexExpression;
use crate::compiler::lexer::Token;
use chumsky::extra::Err;
use chumsky::prelude::*;

pub fn unary<'a>(
    atom: impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DatexExpression, Err<Cheap>> + Clone + 'a {
    recursive(|unary| {
        // & or &mut prefix
        just(Token::Ampersand)
            .ignore_then(just(Token::Mutable).or_not().padded_by(whitespace()))
            .then(unary)
            .map(|(mut_kw, expr)| {
                if mut_kw.is_some() {
                    DatexExpression::RefMut(Box::new(expr))
                } else {
                    DatexExpression::Ref(Box::new(expr))
                }
            })
            // could also add unary minus, not, etc. here later
            .or(atom)
    })
}

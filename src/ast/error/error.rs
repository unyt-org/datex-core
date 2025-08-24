use std::ops::Range;

use crate::{ast::TokenInput, compiler::lexer::Token};

#[derive(Debug, Clone, PartialEq)]

pub enum SpanOrToken {
    Span(Range<usize>),
    Token(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Unexpected {
        span: SpanOrToken,
        found: Option<Token>,
        expected: Vec<DefaultExpected<'static, Token>>,
    },
    UnclosedDelimiter {
        span: SpanOrToken,
        delimiter: Token,
    },
    InvalidToken {
        span: SpanOrToken,
        token: Token,
    },
    Custom {
        span: SpanOrToken,
        message: String,
    },
}
impl ParseError {
    pub fn new_invalid_token(token: Token, span: Range<usize>) -> Self {
        ParseError::InvalidToken {
            span: SpanOrToken::Span(span),
            token,
        }
    }

    fn span_or_token(&self) -> &SpanOrToken {
        match self {
            ParseError::Unexpected { span, .. } => span,
            ParseError::UnclosedDelimiter { span, .. } => span,
            ParseError::InvalidToken { span, .. } => span,
            ParseError::Custom { span, .. } => span,
        }
    }
    pub fn span(&self) -> Option<&Range<usize>> {
        match self.span_or_token() {
            SpanOrToken::Span(span) => Some(span),
            SpanOrToken::Token(_) => None,
        }
    }
    pub fn token_pos(&self) -> Option<usize> {
        match self.span_or_token() {
            SpanOrToken::Span(_) => None,
            SpanOrToken::Token(pos) => Some(*pos),
        }
    }
}
use chumsky::{
    DefaultExpected,
    error::{Error, LabelError},
    span::SimpleSpan,
    util::MaybeRef,
};

impl<'a> Error<'a, TokenInput<'a>> for ParseError {
    fn merge(mut self, mut other: Self) -> Self {
        if let (
            Self::Unexpected { expected, .. },
            Self::Unexpected {
                expected: expected_other,
                ..
            },
        ) = (&mut self, &mut other)
        {
            expected.append(expected_other);
        }
        self
    }
}

impl<'a> LabelError<'a, TokenInput<'a>, DefaultExpected<'a, Token>>
    for ParseError
{
    fn expected_found<Iter: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        expected: Iter,
        found: Option<MaybeRef<'a, Token>>,
        span: SimpleSpan<usize>,
    ) -> Self {
        Self::Unexpected {
            span: SpanOrToken::Token(span.start),
            expected: expected.into_iter().map(|e| e.into_owned()).collect(),
            found: found.as_deref().cloned(),
        }
    }
}

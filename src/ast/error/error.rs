use std::{collections::HashSet, io::Write, ops::Range};

use crate::{
    ast::{
        TokenInput,
        error::{pattern::Pattern, src::SrcId},
    },
    compiler::lexer::Token,
    values::core_values::{
        endpoint::InvalidEndpointError, error::NumberParseError,
    },
};

#[derive(Debug, Clone, PartialEq)]

pub enum SpanOrToken {
    Span(Range<usize>),
    Token(usize),
}
impl From<Range<usize>> for SpanOrToken {
    fn from(value: Range<usize>) -> Self {
        SpanOrToken::Span(value)
    }
}
impl From<usize> for SpanOrToken {
    fn from(value: usize) -> Self {
        SpanOrToken::Token(value)
    }
}
#[derive(Debug, PartialEq, Clone)]
pub enum ErrorKind {
    Custom(HashSet<String>),
    InvalidEndpoint(InvalidEndpointError),
    NumberParseError(NumberParseError),
    UnexpectedEnd,
    Unexpected {
        found: Option<Pattern>,
        expected: Vec<Pattern>,
    },
    Unclosed {
        start: Pattern,
        before_span: SpanOrToken,
        before: Option<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    kind: ErrorKind,
    span: SpanOrToken,
    context: Option<(SpanOrToken, String)>,
    note: Option<&'static str>,
}
impl From<NumberParseError> for ParseError {
    fn from(value: NumberParseError) -> Self {
        Self::new(ErrorKind::NumberParseError(value))
    }
}
impl From<InvalidEndpointError> for ParseError {
    fn from(value: InvalidEndpointError) -> Self {
        Self::new(ErrorKind::InvalidEndpoint(value))
    }
}
impl From<&str> for ParseError {
    fn from(value: &str) -> Self {
        Self::new_custom(value.to_string())
    }
}
impl From<String> for ParseError {
    fn from(value: String) -> Self {
        Self::new_custom(value)
    }
}

impl ParseError {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            span: SpanOrToken::Token(0),
            context: None,
            note: None,
        }
    }
    pub fn new_custom(message: String) -> Self {
        Self::new(ErrorKind::Custom(HashSet::from([message])))
    }
    pub fn new_unexpected_end(span: SpanOrToken) -> Self {
        Self::new(ErrorKind::UnexpectedEnd)
    }
    pub fn new_unexpected(found: Option<Pattern>) -> Self {
        Self {
            kind: ErrorKind::Unexpected {
                found,
                expected: Vec::new(),
            },
            span: SpanOrToken::Token(0),
            context: None,
            note: None,
        }
    }
    pub(crate) fn new_unexpected_with_span<T: Into<SpanOrToken>>(
        found: Option<Pattern>,
        span: T,
    ) -> Self {
        Self {
            kind: ErrorKind::Unexpected {
                found,
                expected: Vec::new(),
            },
            span: span.into(),
            context: None,
            note: None,
        }
    }
    pub fn new_unclosed(
        start: Pattern,
        before_span: SpanOrToken,
        before: Option<Pattern>,
    ) -> Self {
        Self::new(ErrorKind::Unclosed {
            start,
            before_span: before_span.clone(),
            before,
        })
    }

    pub fn with_context(
        mut self,
        span: SpanOrToken,
        context: &'static str,
    ) -> Self {
        self.context = Some((span, context.to_string()));
        self
    }

    pub fn with_note(mut self, note: &'static str) -> Self {
        self.note = Some(note);
        self
    }
}

fn expected_items_to_string(expected: &[Pattern]) -> String {
    let mut normal_items = Vec::new();
    let mut has_something_else = false;

    for expected in expected {
        match expected {
            Pattern::SomethingElse => has_something_else = true,
            _ => normal_items.push(expected.as_string()),
        }
    }
    if has_something_else {
        normal_items.push(Pattern::SomethingElse.to_string());
    }
    match normal_items.len() {
        0 => Pattern::SomethingElse.to_string(),
        1 => normal_items[0].clone(),
        2 => format!("{} or {}", normal_items[0], normal_items[1]),
        _ => {
            let last = normal_items.pop().unwrap();
            format!("{}, or {}", normal_items.join(", "), last)
        }
    }
}

impl ParseError {
    pub(crate) fn set_span(&mut self, span: Range<usize>) {
        self.span = span.into();
    }
    pub(crate) fn set_token_pos(&mut self, pos: usize) {
        self.span = pos.into();
    }
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    pub fn note(&self) -> Option<&'static str> {
        self.note
    }
    pub fn span(&self) -> Option<Range<usize>> {
        match &self.span {
            SpanOrToken::Span(span) => Some(span.clone()),
            SpanOrToken::Token(_) => None,
        }
    }
    pub fn token_pos(&self) -> Option<usize> {
        match &self.span {
            SpanOrToken::Span(_) => None,
            SpanOrToken::Token(pos) => Some(*pos),
        }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            ErrorKind::Custom(msg) => {
                msg.iter().cloned().collect::<Vec<_>>().join(" | ")
            }
            ErrorKind::NumberParseError(err) => err.to_string(),
            ErrorKind::UnexpectedEnd => "Unexpected end of input".to_string(),
            ErrorKind::Unexpected { found, expected } => {
                let mut msg = String::new();
                if let Some(found) = found {
                    msg.push_str(&format!(
                        "Unexpected {}: {}",
                        found.kind(),
                        found
                    ));
                } else {
                    msg.push_str("Unexpected end of input");
                }
                if !expected.is_empty() {
                    msg.push_str(", expected one of: ");
                    msg.push_str(&expected_items_to_string(expected));
                }
                msg
            }
            ErrorKind::Unclosed {
                start,
                before_span,
                before,
            } => {
                let mut msg = format!("Unclosed delimiter: {}", start);
                if let Some(before) = before {
                    msg.push_str(&format!(", before {}", before));
                }
                msg
            }
            ErrorKind::InvalidEndpoint(e) => {
                format!("Parsing error: {}", e)
            }
        }
    }

    pub fn label(&self) -> String {
        use ariadne::{Color, Fmt};
        match &self.kind {
            ErrorKind::NumberParseError(_) => "Number parse error".to_string(),
            ErrorKind::UnexpectedEnd => "End of input".to_string(),
            ErrorKind::Unexpected { found, .. } => {
                format!(
                    "Unexpected {}",
                    found.clone().unwrap().to_string().fg(Color::Red)
                )
            }
            ErrorKind::Unclosed { start, .. } => {
                format!("Delimiter {} is never closed", start.fg(Color::Red))
            }
            ErrorKind::Custom(_) => "Invalid syntax".to_string(),
            ErrorKind::InvalidEndpoint(_) => "Invalid endpoint".to_string(),
        }
    }

    pub fn write<C: ariadne::Cache<SrcId>>(self, cache: C, writer: impl Write) {
        use ariadne::{Color, Fmt, Label, Report, ReportKind};

        let span = (SrcId::test(), self.span().unwrap().clone());
        let mut note: String = self
            .note
            .unwrap_or("Please check the syntax and try again.")
            .to_string();
        if !note.ends_with('.') {
            note.push('.');
        }
        let mut report = Report::build(ReportKind::Error, span.clone())
            .with_code("Syntax")
            .with_message(self.message())
            .with_note(note)
            .with_label(
                Label::new(span)
                    .with_message(self.label())
                    .with_color(Color::Red),
            );
        if let Some((_, context)) = self.context {
            report = report.with_help(format!(
                "In the context of: {}",
                context.fg(Color::Yellow)
            ));
        }
        report.finish().write(cache, writer).unwrap();
    }
}

use chumsky::{
    DefaultExpected,
    error::{Error, LabelError},
    span::SimpleSpan,
    util::MaybeRef,
};
use rsa::rand_core::le;

impl<'a> Error<'a, TokenInput<'a>> for ParseError {
    fn merge(mut self, mut other: Self) -> Self {
        match (&mut self.kind, &mut other.kind) {
            (ErrorKind::Custom(msg1), ErrorKind::Custom(msg2)) => {
                msg1.extend(msg2.drain());
            }
            (ErrorKind::UnexpectedEnd, ErrorKind::UnexpectedEnd) => {}
            (
                ErrorKind::Unexpected {
                    found: found1,
                    expected: expected1,
                },
                ErrorKind::Unexpected {
                    found: found2,
                    expected: expected2,
                },
            ) => {
                if found1.is_none() {
                    *found1 = found2.take();
                }
                for exp in expected2.drain(..) {
                    if !expected1.contains(&exp) {
                        expected1.push(exp);
                    }
                }
            }
            _ => {}
        };

        //

        // if let Some((span, ctx)) = &other.context {
        //     // let new_ctx = self
        //     //     .context
        //     //     .as_ref()
        //     //     .map(|(_, c)| c.clone())
        //     //     .unwrap_or_default();
        //     // let ctx = format!("{}; {}", new_ctx, ctx);
        //     // self.context = Some((self.span.clone(), ctx.clone()));
        // }
        if other.context.is_some() {
            self.context = other.context.take();
        }

        self
    }
}

impl<'a>
    chumsky::error::LabelError<'a, TokenInput<'a>, DefaultExpected<'a, Token>>
    for ParseError
{
    fn expected_found<Iter: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        expected: Iter,
        found: Option<MaybeRef<'a, Token>>,
        span: chumsky::span::SimpleSpan<usize>,
    ) -> Self {
        let expected: Vec<Pattern> = expected
            .into_iter()
            .map(|e| match e {
                DefaultExpected::Any => Pattern::Any,
                DefaultExpected::Token(token) => {
                    Pattern::from(token.into_inner().clone())
                }
                DefaultExpected::EndOfInput => Pattern::EndOfInput,
                DefaultExpected::SomethingElse => Pattern::SomethingElse,
                _ => unreachable!("Unexpected expected variant: {:?}", e),
            })
            .collect();

        if found.is_none() {
            return ParseError::new_unexpected_end(span.start.into());
        }

        ParseError {
            kind: ErrorKind::Unexpected {
                found: found.as_deref().cloned().map(Pattern::from),
                expected,
            },
            span: SpanOrToken::Token(span.start),
            context: None,
            note: None,
        }
    }
}

impl<'a> LabelError<'a, TokenInput<'a>, Pattern> for ParseError {
    fn label_with(&mut self, label: Pattern) {
        self.context = Some((self.span.clone(), label.to_string()));
    }
    fn expected_found<Iter: IntoIterator<Item = Pattern>>(
        expected: Iter,
        found: Option<MaybeRef<'a, Token>>,
        span: SimpleSpan<usize>,
    ) -> Self {
        let expected: Vec<Pattern> = expected.into_iter().collect();
        if found.is_none() {
            return ParseError::new_unexpected_end(span.start.into());
        }

        // let context = span.context();
        ParseError {
            kind: ErrorKind::Unexpected {
                found: found.as_deref().cloned().map(Pattern::Token),
                expected,
            },
            span: SpanOrToken::Token(span.start),
            context: None,
            note: None,
        }
    }
}

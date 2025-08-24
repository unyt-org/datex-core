use std::{collections::HashSet, io::Write, ops::Range};

use crate::{
    ast::{
        TokenInput,
        error::{pattern::Pattern, src::SrcId},
    },
    compiler::lexer::Token,
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
    UnexpectedEnd,
    Unexpected {
        found: Option<Token>,
        expected: Vec<DefaultExpected<'static, Token>>,
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
    while_parsing: Option<(SpanOrToken, &'static str)>,
    label: Option<&'static str>,
}

impl ParseError {
    pub fn new(kind: ErrorKind, span: SpanOrToken) -> Self {
        Self {
            kind,
            span,
            while_parsing: None,
            label: None,
        }
    }
    pub fn new_custom(message: String, span: SpanOrToken) -> Self {
        Self::new(ErrorKind::Custom(HashSet::from([message])), span)
    }
    pub fn new_unexpected_end(span: SpanOrToken) -> Self {
        Self::new(ErrorKind::UnexpectedEnd, span)
    }
    pub fn new_unexpected<T: Into<SpanOrToken>>(
        found: Option<Token>,
        span: T,
    ) -> Self {
        Self::new(
            ErrorKind::Unexpected {
                found,
                expected: Vec::new(),
            },
            span.into(),
        )
    }
    pub fn new_unclosed(
        start: Pattern,
        before_span: SpanOrToken,
        before: Option<Pattern>,
    ) -> Self {
        Self::new(
            ErrorKind::Unclosed {
                start,
                before_span: before_span.clone(),
                before,
            },
            before_span,
        )
    }

    pub fn with_while_parsing(
        mut self,
        span: SpanOrToken,
        context: &'static str,
    ) -> Self {
        self.while_parsing = Some((span, context));
        self
    }

    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

fn expected_items_to_string(
    expected: &[DefaultExpected<'static, Token>],
) -> String {
    let mut normal_items = Vec::new();
    let mut has_something_else = false;

    for expected in expected {
        match expected {
            DefaultExpected::Any => normal_items.push("any".to_string()),
            DefaultExpected::Token(token) => {
                normal_items.push(token.as_string())
            }
            DefaultExpected::EndOfInput => {
                normal_items.push("end of input".to_string())
            }
            DefaultExpected::SomethingElse => has_something_else = true,
            e => unreachable!("Unexpected expected variant: {:?}", e),
        }
    }
    if has_something_else {
        normal_items.push("something else".to_string());
    }
    match normal_items.len() {
        0 => "something else".to_string(),
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
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    pub fn label(&self) -> Option<&'static str> {
        self.label
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
            ErrorKind::UnexpectedEnd => "Unexpected end of input".to_string(),
            ErrorKind::Unexpected { found, expected } => {
                let mut msg = String::new();
                if let Some(found) = found {
                    msg.push_str(&format!("Unexpected token: {}", found));
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
        }
    }

    pub fn write<C: ariadne::Cache<SrcId>>(self, cache: C, writer: impl Write) {
        use ariadne::{Color, Fmt, Label, Report, ReportKind};

        let span = (SrcId::test(), self.span().unwrap().clone());

        let report = Report::build(ReportKind::Error, span.clone())
            .with_code("Unexpected Token")
            .with_message(self.message())
            .with_note("Please check the syntax and try again.")
            .with_label(
                Label::new(span)
                    .with_message(match &self.kind {
                        ErrorKind::UnexpectedEnd => "End of input".to_string(),
                        ErrorKind::Unexpected { found, expected } => {
                            format!(
                                "Unexpected {}",
                                found
                                    .clone()
                                    .unwrap()
                                    .as_string()
                                    .fg(Color::Red)
                            )
                        }
                        ErrorKind::Unclosed { start, .. } => format!(
                            "Delimiter {} is never closed",
                            start.fg(Color::Red)
                        ),
                        ErrorKind::Custom(msg) => {
                            msg.iter().cloned().collect::<Vec<_>>().join(" | ")
                        }
                    })
                    .with_color(Color::Red),
            );
        report.finish().write(cache, writer).unwrap();
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
        let expected: Vec<DefaultExpected<'static, Token>> =
            expected.into_iter().map(|e| e.into_owned()).collect();
        // let context = span.context();
        ParseError {
            kind: ErrorKind::Unexpected {
                found: found.as_deref().cloned(),
                expected,
            },
            span: SpanOrToken::Token(span.start),
            while_parsing: None,
            label: None,
        }
    }
}

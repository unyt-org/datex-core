use chumsky::span::SimpleSpan;

pub(crate) trait Spanned: Sized {
    type Output;
    fn with_span(self, span: SimpleSpan) -> Self::Output;
    fn with_default_span(self) -> Self::Output;
}

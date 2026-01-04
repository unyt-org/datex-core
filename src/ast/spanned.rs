use core::ops::Range;

pub trait Spanned: Sized {
    type Output;
    fn with_span<T: Into<Range<usize>>>(self, span: T) -> Self::Output;
    fn with_default_span(self) -> Self::Output;
}

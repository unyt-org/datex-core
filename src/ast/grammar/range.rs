use crate::ast::DatexParserTrait;

pub fn range<'a>(
    inner: impl DatexParserTrait<'a>,
) -> impl DatexParserTrait<'a> {
    // panic!("OutRanged");
    inner
}

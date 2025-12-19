use crate::values::core_values::integer::Integer;
use core::fmt;
use core::ops;

#[derive(Debug)]
pub enum RangeError {
    StepOverflow,
    InvalidRange,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Range {
    // lower bound (inclusive)
    pub start: Integer,
    // upper bound (exclusive)
    pub end: Integer,
}

impl From<Range> for ops::Range<Integer> {
    fn from(range: Range) -> Self {
        range.start..range.end
    }
}

impl From<ops::Range<Integer>> for Range {
    fn from(range: ops::Range<Integer>) -> Self {
        Range {
            start: range.start,
            end: range.end,
        }
    }
}

impl Range {
    pub fn new(start: Integer, end: Integer) -> Self {
        Range { start, end }
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{}..{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_values::integer::Integer;

    fn test_helper() -> (Integer, Integer, Integer) {
        (
            Integer::from_string("11").unwrap(),
            Integer::from_string("23").unwrap(),
            Integer::from_string("3").unwrap(),
        )
    }

    #[test]
    pub fn range_from_into() {
        let (begin, ending, _) = test_helper();
        let dx_range = Range::new(begin, ending.clone());
        let std_range: ops::Range<Integer> = dx_range.clone().into();
        let other_dx_range: Range = std_range.clone().into();

        let other_std_range = ops::Range::from(other_dx_range.clone());
        let other_dx_range = Range::from(std_range.clone());
        assert_eq!(dx_range, other_dx_range);
        assert_eq!(std_range, other_std_range);
    }

    #[test]
    pub fn range_formatting() {
        let (begin, ending, _) = test_helper();
        let range = Range::new(begin, ending.clone());

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "11..23");
        assert_eq!(debugged, "Integer(11)..Integer(23)");
    }
}

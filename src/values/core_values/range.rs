use crate::values::core_values::integer::Integer;
use core::fmt;
use core::ops::Range;

#[derive(Debug)]
pub enum RangeError {
    StepOverflow,
    InvalidRange,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct RangeDefinition {
    // lower bound (inclusive)
    pub start: Integer,
    // upper bound (exclusive)
    pub end: Integer,
}

impl From<RangeDefinition> for Range<Integer> {
    fn from(range: RangeDefinition) -> Self {
        range.start..range.end
    }
}

impl From<Range<Integer>> for RangeDefinition {
    fn from(range: Range<Integer>) -> Self {
        RangeDefinition {
            start: range.start,
            end: range.end,
        }
    }
}

impl RangeDefinition {
    pub fn new(start: Integer, end: Integer) -> Self {
        RangeDefinition { start, end }
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl fmt::Debug for RangeDefinition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl fmt::Display for RangeDefinition {
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
        let dx_range = RangeDefinition::new(begin, ending.clone());
        let std_range: Range<Integer> = dx_range.clone().into();
        let other_dx_range: RangeDefinition = std_range.clone().into();

        let other_std_range = Range::from(other_dx_range.clone());
        let other_dx_range = RangeDefinition::from(std_range.clone());
        assert_eq!(dx_range, other_dx_range);
        assert_eq!(std_range, other_std_range);
    }

    #[test]
    pub fn range_formatting() {
        let (begin, ending, _) = test_helper();
        let range = RangeDefinition::new(begin, ending.clone());

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "11..23");
        assert_eq!(debugged, "Integer(11)..Integer(23)");
    }
}

use core::fmt;
use core::ops::Range;

#[derive(Debug)]
pub enum RangeError {
    StepOverflow,
    InvalidRange,
}

#[derive(Clone)]
pub struct RangeDefinition<T> {
    // lower bound (inclusive)
    start: T,
    // upper bound (exclusive)
    end: T,
}

impl<T> From<RangeDefinition<T>> for Range<T> {
    fn from(range: RangeDefinition<T>) -> Self {
        range.start..range.end
    }
}

impl<T> From<Range<T>> for RangeDefinition<T> {
    fn from(range: Range<T>) -> Self {
        RangeDefinition {
            start: range.start,
            end: range.end,
        }
    }
}

impl<T: PartialEq> PartialEq for RangeDefinition<T> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<T: PartialOrd<T>> RangeDefinition<T> {
    pub fn new(start: T, end: T) -> Self {
        RangeDefinition { start, end }
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl<T: fmt::Debug> fmt::Debug for RangeDefinition<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl<T: fmt::Display> fmt::Display for RangeDefinition<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{}..{}", self.start, self.end)
    }
}

pub struct RangeStepper<T> {
    range: RangeDefinition<T>,
    step: T,
    current: T,
}

impl<T: fmt::Debug> fmt::Debug for RangeStepper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.range, f)
    }
}

impl<T: fmt::Display> fmt::Display for RangeStepper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.range, f)
    }
}

impl<T> RangeStepper<T>
where
    T: Clone + PartialOrd,
{
    fn new(range: RangeDefinition<T>, step: T) -> Self {
        let current = range.start.clone();
        Self {
            range,
            step,
            current,
        }
    }

    pub fn start(&self) -> &T {
        &self.range.start
    }

    pub fn end(&self) -> &T {
        &self.range.end
    }
}

impl<T> Iterator for RangeStepper<T>
where
    T: Clone + PartialOrd + core::ops::Add<Output = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < *self.end() {
            let val = self.current.clone();
            self.current = self.current.clone() + self.step.clone();
            Some(val)
        } else {
            None
        }
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
        let other_dx_range: RangeDefinition<Integer> = std_range.clone().into();

        let other_std_range = Range::from(other_dx_range.clone());
        let other_dx_range = RangeDefinition::from(std_range.clone());
        assert_eq!(dx_range, other_dx_range);
        assert_eq!(std_range, other_std_range);
    }

    #[test]
    pub fn range_formatting() {
        let (begin, ending, step) = test_helper();
        let range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "11..23");
        assert_eq!(debugged, "Integer(11)..Integer(23)");
    }

    #[test]
    pub fn range_iterator() {
        let (begin, ending, step) = test_helper();
        let mut range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );
        assert!(!range.range.is_empty());

        let pre_sum = Integer::from_string("62").unwrap();
        let mut post_sum = Integer::from_string("0").unwrap();
        for i in &mut range {
            post_sum = post_sum + i;
        }
        assert_eq!(pre_sum, post_sum);

        assert!(!range.range.is_empty());
        assert!(range.next().is_none());
        assert_eq!(range.current, ending);
    }
}

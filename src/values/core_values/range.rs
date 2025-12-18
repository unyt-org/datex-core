use core::fmt;

#[derive(Debug)]
pub enum RangeError {
    StepOverflow,
    InvalidRange,
}

pub struct RangeDefinition<T> {
    // lower bound (inclusive)
    start: T,
    // upper bound (exclusive)
    end: T,
}

impl<T: PartialOrd<T>> RangeDefinition<T> {
    pub fn new(start: T, end: T) -> Self {
        RangeDefinition { start, end }
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    pub fn start(&self) -> &T {
        &self.start
    }

    pub fn end(&self) -> &T {
        &self.end
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
}

impl<T> Iterator for RangeStepper<T>
where
    T: Clone + PartialOrd + core::ops::Add<Output = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < *self.range.end() {
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

    #[test]
    pub fn integer_range() {
        let begin = Integer::from_string("11").unwrap();
        let ending = Integer::from_string("23").unwrap();
        let step = Integer::from_string("3").unwrap();

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

    #[test]
    pub fn integer_range_formatting() {
        // Integer Ranges
        let begin = Integer::from_string("11").unwrap();
        let ending = Integer::from_string("23").unwrap();
        let step = Integer::from_string("3").unwrap();

        let range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "11..23");
        assert_eq!(debugged, "Integer(11)..Integer(23)");
    }
}

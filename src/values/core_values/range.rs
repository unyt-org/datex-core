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
        core::write!(f, "{:?}...{:?}", self.start, self.end)
    }
}

impl<T: fmt::Display> fmt::Display for RangeDefinition<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        core::write!(f, "{}...{}", self.start, self.end)
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

pub struct FallibleRangeStepper<T> {
    stepper: RangeStepper<T>,
}

impl<T: PartialOrd<T> + Clone> FallibleRangeStepper<T> {
    fn new(range: RangeDefinition<T>, step: T) -> Result<Self, RangeError> {
        match range.is_empty() {
            true => Err(RangeError::InvalidRange),
            false => Ok(Self {
                stepper: RangeStepper::new(range, step),
            }),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FallibleRangeStepper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.stepper, f)
    }
}

impl<T: fmt::Display> fmt::Display for FallibleRangeStepper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.stepper, f)
    }
}

impl<T> Iterator for FallibleRangeStepper<T>
where
    T: Clone + PartialOrd + core::ops::Add<Output = Option<T>>,
{
    type Item = Result<T, RangeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stepper.current < *self.stepper.range.end() {
            let val = self.stepper.current.clone();
            match self.stepper.current.clone() + self.stepper.step.clone() {
                Some(next) => {
                    self.stepper.current = next;
                    Some(Ok(val))
                }
                None => Some(Err(RangeError::StepOverflow)),
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::core_values::decimal::Decimal;
    use crate::values::core_values::decimal::typed_decimal::{
        DecimalTypeVariant, TypedDecimal,
    };
    use crate::values::core_values::integer::Integer;
    use crate::values::core_values::integer::typed_integer::{
        IntegerTypeVariant, TypedInteger,
    };

    #[test]
    pub fn typed_integer_range() -> Result<(), RangeError> {
        // 11 + 14 + 17 + 20 = 25 + 37 = 62
        let begin = TypedInteger::from_string_with_variant(
            "11",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let ending = TypedInteger::from_string_with_variant(
            "23",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let step =
            TypedInteger::from_string_with_variant("3", IntegerTypeVariant::U8)
                .unwrap();

        let mut range = FallibleRangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        )
        .unwrap();

        assert!(!range.stepper.range.is_empty());
        let pre_sum = TypedInteger::from_string_with_variant(
            "62",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let mut post_sum =
            TypedInteger::from_string_with_variant("0", IntegerTypeVariant::U8)
                .unwrap();
        for i in &mut range {
            post_sum = (post_sum + i?).ok_or(RangeError::StepOverflow)?;
        }
        assert_eq!(pre_sum, post_sum);

        assert!(!range.stepper.range.is_empty());
        assert!(range.next().is_none());
        assert_eq!(range.stepper.current, ending);
        Ok(())
    }

    #[test]
    pub fn typed_decimal_range() {
        let begin = TypedDecimal::from_string_and_variant(
            "0.11",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let ending = TypedDecimal::from_string_and_variant(
            "0.23",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let step = TypedDecimal::from_string_and_variant(
            "0.03",
            DecimalTypeVariant::F32,
        )
        .unwrap();

        let mut range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );
        assert!(!range.range.is_empty());

        let pre_sum = TypedDecimal::from_string_and_variant(
            "0.62",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let mut post_sum =
            TypedDecimal::from_string_and_variant("0", DecimalTypeVariant::F32)
                .unwrap();
        for i in &mut range {
            post_sum += i;
        }
        assert_eq!(pre_sum, post_sum);

        assert!(!range.range.is_empty());
        assert!(range.next().is_none());
        assert_eq!(range.current, ending);
    }

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
    pub fn decimal_range() {
        let begin = Decimal::from_string("0.11").unwrap();
        let ending = Decimal::from_string("0.23").unwrap();
        let step = Decimal::from_string("0.03").unwrap();

        let mut range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );
        assert!(!range.range.is_empty());

        let pre_sum = Decimal::from_string("0.62").unwrap();
        let mut post_sum = Decimal::from_string("0").unwrap();
        for i in &mut range {
            post_sum = post_sum + i;
        }
        assert_eq!(pre_sum, post_sum);

        assert!(!range.range.is_empty());
        assert!(range.next().is_none());
        assert_eq!(range.current, ending);
    }

    #[test]
    pub fn typed_integer_range_formatting() {
        // TypedInteger Ranges
        let begin = TypedInteger::from_string_with_variant(
            "11",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let ending = TypedInteger::from_string_with_variant(
            "23",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let step =
            TypedInteger::from_string_with_variant("3", IntegerTypeVariant::U8)
                .unwrap();

        let range = FallibleRangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        )
        .unwrap();

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "11...23");
        assert_eq!(debugged, "U8(11)...U8(23)");
    }

    #[test]
    pub fn typed_decimal_range_formatting() {
        // TypedDecimal Ranges
        let begin = TypedDecimal::from_string_and_variant(
            "0.11",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let ending = TypedDecimal::from_string_and_variant(
            "0.23",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let step = TypedDecimal::from_string_and_variant(
            "0.03",
            DecimalTypeVariant::F32,
        )
        .unwrap();

        let range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );
        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "0.11...0.23");
        assert_eq!(debugged, "F32(0.11)...F32(0.23)");
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
        assert_eq!(displayed, "11...23");
        assert_eq!(debugged, "Integer(11)...Integer(23)");
    }

    #[test]
    pub fn decimal_range_formatting() {
        // Decimal ranges
        let begin = Decimal::from_string("0.11").unwrap();
        let ending = Decimal::from_string("0.23").unwrap();
        let step = Decimal::from_string("0.03").unwrap();

        let range = RangeStepper::new(
            RangeDefinition::new(begin, ending.clone()),
            step,
        );

        let displayed = format!("{}", range);
        let debugged = format!("{:?}", range);
        assert_eq!(displayed, "0.11...0.23");
        assert_eq!(
            debugged,
            "Finite(Rational { big_rational: Ratio { numer: 11, denom: 100 } })...Finite(Rational { big_rational: Ratio { numer: 23, denom: 100 } })"
        );
    }
}

pub struct RangeDefinition<T> {
    // lower bound (inclusive)
    start: T,
    // upper bound (exclusive)
    end: T,
    // Items per step (requires metric)
    step: T,
}

impl<T: PartialOrd<T>> RangeDefinition<T> {
    fn new(start: T, end: T, step: T) -> Self {
        RangeDefinition { start, end, step }
    }
    fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

impl<T> Iterator for RangeDefinition<T>
where
    T: Clone + PartialOrd + core::ops::Add<Output = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_empty() {
            let val = self.start.clone();
            self.start = self.start.clone() + self.step.clone();
            Some(val)
        } else {
            None
        }
    }
}

pub struct TypedRangeDefinition<T> {
    range: RangeDefinition<T>,
}

impl<T: PartialOrd<T>> TypedRangeDefinition<T> {
    fn new(start: T, end: T, step: T) -> Self {
        TypedRangeDefinition {
            range: RangeDefinition::new(start, end, step),
        }
    }
}

impl<T> Iterator for TypedRangeDefinition<T>
where
    T: Clone + PartialOrd + core::ops::Add<Output = Option<T>>,
{
    type Item = Option<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.range.is_empty() {
            let val = self.range.start.clone();
            self.range.start =
                (self.range.start.clone() + self.range.step.clone()).unwrap();
            Some(Some(val))
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
    pub fn typed_integer_range() {
        // 11 + 14 + 17 + 20 = 25 + 37 = 62
        let begin = TypedInteger::from_string_with_variant(
            "11",
            IntegerTypeVariant::U8,
        );
        let ending = TypedInteger::from_string_with_variant(
            "23",
            IntegerTypeVariant::U8,
        );
        let step =
            TypedInteger::from_string_with_variant("3", IntegerTypeVariant::U8);

        let range = TypedRangeDefinition::new(
            begin.unwrap(),
            ending.unwrap(),
            step.unwrap(),
        );

        assert!(!range.range.is_empty());

        let pre_sum = TypedInteger::from_string_with_variant(
            "62",
            IntegerTypeVariant::U8,
        )
        .unwrap();
        let mut post_sum =
            TypedInteger::from_string_with_variant("0", IntegerTypeVariant::U8)
                .unwrap();
        for i in range {
            post_sum = (post_sum + i.unwrap()).unwrap();
        }
        assert_eq!(pre_sum, post_sum);
    }

    #[test]
    pub fn typed_decimal_range() {
        // 11 + 14 + 17 + 20 = 25 + 37 = 62
        let begin = TypedDecimal::from_string_and_variant(
            "0.11",
            DecimalTypeVariant::F32,
        );
        let ending = TypedDecimal::from_string_and_variant(
            "0.23",
            DecimalTypeVariant::F32,
        );
        let step = TypedDecimal::from_string_and_variant(
            "0.03",
            DecimalTypeVariant::F32,
        );

        let range = RangeDefinition::new(
            begin.unwrap(),
            ending.unwrap(),
            step.unwrap(),
        );

        assert!(!range.is_empty());

        let pre_sum = TypedDecimal::from_string_and_variant(
            "0.62",
            DecimalTypeVariant::F32,
        )
        .unwrap();
        let mut post_sum =
            TypedDecimal::from_string_and_variant("0", DecimalTypeVariant::F32)
                .unwrap();
        for i in range {
            post_sum += i;
        }
        assert_eq!(pre_sum, post_sum);
    }

    #[test]
    pub fn integer_range() {
        let begin = Integer::from_string("11");
        let ending = Integer::from_string("23");
        let step = Integer::from_string("3");

        let range = RangeDefinition::new(
            begin.unwrap(),
            ending.unwrap(),
            step.unwrap(),
        );

        assert!(!range.is_empty());

        let pre_sum = Integer::from_string("62").unwrap();
        let mut post_sum = Integer::from_string("0").unwrap();
        for i in range {
            post_sum = post_sum + i;
        }
        assert_eq!(pre_sum, post_sum);
    }

    #[test]
    pub fn decimal_range() {
        let begin = Decimal::from_string("0.11");
        let ending = Decimal::from_string("0.23");
        let step = Decimal::from_string("0.03");

        let range = RangeDefinition::new(
            begin.unwrap(),
            ending.unwrap(),
            step.unwrap(),
        );

        assert!(!range.is_empty());

        let pre_sum = Decimal::from_string("0.62").unwrap();
        let mut post_sum = Decimal::from_string("0").unwrap();
        for i in range {
            post_sum = post_sum + i;
        }
        assert_eq!(pre_sum, post_sum);
    }
}

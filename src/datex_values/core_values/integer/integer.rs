use std::{
    fmt::Display,
    hash::Hash,
    ops::{Add, Sub},
};

use crate::datex_values::{
    core_values::integer::{
        typed_integer::TypedInteger,
        utils::{smallest_fitting_signed, smallest_fitting_unsigned},
    },
    traits::soft_eq::SoftEq,
};

#[derive(Debug, Clone, Eq, Copy)]
pub struct Integer(pub TypedInteger);
impl Integer {
    pub fn to_smallest_fitting(&self) -> TypedInteger {
        self.0.to_smallest_fitting()
    }
    
    pub fn from_string(s: &str) -> Result<Self, String> {
        Integer::from_string_radix(s, 10)
    }
    
    pub fn from_string_radix(s: &str, radix: u32) -> Result<Self, String> {
        // remove all underscores
        let s = &s.replace('_', "");
        match i128::from_str_radix(s, radix) {
            Ok(value) => Ok(Integer(TypedInteger::I128(value))),
            Err(_) => match s.parse::<u128>() {
                Ok(value) => Ok(Integer(TypedInteger::U128(value))),
                Err(_) => Err(format!("Failed to parse integer from string with radix {radix}: {s}")),
            },
        }
    }
}

impl SoftEq for Integer {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0.soft_eq(&other.0)
    }
}

impl<T: Into<TypedInteger>> From<T> for Integer {
    fn from(value: T) -> Self {
        Integer(value.into())
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// FIXME use integer i32 by default and switch automaticially if a calculation provoces one of the values to get out of bounds
impl Add for Integer {
    type Output = Option<Integer>;

    fn add(self, rhs: Self) -> Self::Output {
        let a = self.0;
        let b = rhs.0;
        if a.is_unsigned() && b.is_unsigned() {
            Some(Integer(smallest_fitting_unsigned(
                a.as_u128().checked_add(b.as_u128())?,
            )))
        } else {
            Some(Integer(smallest_fitting_signed(
                a.as_i128()?.checked_add(b.as_i128()?)?,
            )))
        }
    }
}
impl Add for &Integer {
    type Output = Option<Integer>;

    fn add(self, rhs: Self) -> Self::Output {
        Integer::add(*self, *rhs)
    }
}

impl Sub for Integer {
    type Output = Option<Integer>;

    fn sub(self, rhs: Self) -> Self::Output {
        let a = self.0;
        let b = rhs.0;
        Some(Integer(smallest_fitting_signed(
            a.as_i128()?.checked_sub(b.as_i128()?)?,
        )))
    }
}

impl Sub for &Integer {
    type Output = Option<Integer>;

    fn sub(self, rhs: Self) -> Self::Output {
        Integer::sub(*self, *rhs)
    }
}

impl PartialEq for Integer {
    fn eq(&self, other: &Self) -> bool {
        self.soft_eq(other)
    }
}

impl Hash for Integer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.0.is_signed() {
            self.0.as_i128().hash(state);
        } else {
            self.0.as_u128().hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_integer_addition() {
        let a = TypedInteger::I8(10);
        let b = TypedInteger::I8(20);

        let result = a + b;
        assert_eq!(result, Some(TypedInteger::I8(30)));

        let c = TypedInteger::U8(10);
        let result = a + c;
        assert_eq!(result, Some(TypedInteger::I8(20)));

        let result = c + a;
        assert_eq!(result, Some(TypedInteger::U8(20)));

        // out of bounds
        let d = TypedInteger::I8(100);
        let e = TypedInteger::I8(50);
        let result = d + e;
        assert_eq!(result, None);
    }

    #[test]
    fn test_typed_integer_subtraction() {
        let a = TypedInteger::I8(30);
        let b = TypedInteger::I8(20);
        let result = a - b;
        assert_eq!(result, Some(TypedInteger::I8(10)));

        // negative result
        let c = TypedInteger::I8(20);
        let d = TypedInteger::I8(30);
        let result = c - d;
        assert_eq!(result, Some(TypedInteger::I8(-10)));

        // out of bounds
        let e = TypedInteger::I8(-100);
        let f = TypedInteger::I8(50);
        let result = e - f;
        assert_eq!(result, None);

        let g = TypedInteger::U8(30);
        let h = TypedInteger::I8(30);
        let result = g - h;
        assert_eq!(result, Some(TypedInteger::U8(0)));

        let h = TypedInteger::U8(30);
        let i = TypedInteger::I8(31);

        let result = h - i;
        assert_eq!(result, None);
    }

    #[test]
    fn test_integer_addition() {
        let a = Integer::from(10_i8);
        let b = Integer::from(20_i8);
        let result = a + b;
        assert_eq!(result, Some(Integer(TypedInteger::I8(30))));

        // let c = Integer::from(10 as u8);
        // let result = a + c;
        // assert_eq!(result, Some(Integer(TypedInteger::I8(20))));

        // let result = c + a;
        // assert_eq!(result, Some(Integer(TypedInteger::U8(20))));

        // // out of bounds
        // let d = Integer::from(100 as i8);
        // let e = Integer::from(50 as i8);
        // let result = d + e;
        // assert_eq!(result, None);
    }

    #[test]
    fn test_integer() {
        let a = Integer::from(1_i8);
        assert_eq!(a.0, TypedInteger::I8(1));

        let b = Integer::from(1_u8);
        assert_eq!(b.0, TypedInteger::U8(1));

        let c = Integer::from(1_i16);
        assert_eq!(c.0, TypedInteger::I16(1));

        let d = Integer::from(1_u16);
        assert_eq!(d.0, TypedInteger::U16(1));

        let e = Integer::from(1_i32);
        assert_eq!(e.0, TypedInteger::I32(1));

        let f = Integer::from(1_u32);
        assert_eq!(f.0, TypedInteger::U32(1));

        let g = Integer::from(1_i64);
        assert_eq!(g.0, TypedInteger::I64(1));

        let h = Integer::from(1_u64);
        assert_eq!(h.0, TypedInteger::U64(1));

        let i = Integer::from(1_i128);
        assert_eq!(i.0, TypedInteger::I128(1));

        let j = Integer::from(1_u128);
        assert_eq!(j.0, TypedInteger::U128(1));

        assert_eq!(a.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(b.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(c.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(d.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(e.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(f.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(g.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(h.to_smallest_fitting(), TypedInteger::U8(1));
        assert_eq!(i.to_smallest_fitting(), TypedInteger::I8(1));
        assert_eq!(j.to_smallest_fitting(), TypedInteger::U8(1));
    }
}

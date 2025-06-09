use crate::datex_values::core_values::decimal::big_decimal::ExtendedBigDecimal;
use crate::datex_values::core_values::decimal::typed_decimal::TypedDecimal;
use crate::datex_values::traits::soft_eq::SoftEq;
use std::hash::Hash;
use std::ops::{Deref, Neg, Sub};
use std::{fmt::Display, ops::Add};

#[derive(Debug, Clone, Eq)]
pub struct Decimal(pub TypedDecimal);
impl SoftEq for Decimal {
    fn soft_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Deref for Decimal {
    type Target = TypedDecimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Into<TypedDecimal>> From<T> for Decimal {
    fn from(value: T) -> Self {
        let typed = value.into();
        match typed {
            TypedDecimal::Big(_) => Decimal(typed),
            _ => Decimal(TypedDecimal::Big(ExtendedBigDecimal::from(
                typed.as_f64(),
            ))),
        }
    }
}

impl From<&str> for Decimal {
    fn from(value: &str) -> Self {
        Decimal(TypedDecimal::Big(
            ExtendedBigDecimal::from_string(value)
                .unwrap_or(ExtendedBigDecimal::NaN),
        ))
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        Decimal(self.0 + rhs.0)
    }
}

impl Add for &Decimal {
    type Output = Decimal;

    fn add(self, rhs: Self) -> Self::Output {
        Decimal::add(self.clone(), rhs.clone())
    }
}

impl Sub for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self.0, rhs.0) {
            (TypedDecimal::Big(a), TypedDecimal::Big(b)) => {
                Decimal(TypedDecimal::Big(a.add(b.neg())))
            }
            _ => unreachable!("Subtraction of Decimal should only be used with TypedDecimal::Big"),
        }
    }
}

impl Sub for &Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Self) -> Self::Output {
        Decimal::sub(self.clone(), rhs.clone())
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Hash for Decimal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[cfg(test)]
mod tests {
    

    use crate::logger::init_logger;

    use super::*;

    #[test]
    fn test_zero() {
        let a = Decimal::from("0.0");
        let b = Decimal::from("0.0");
        matches!(a.0, TypedDecimal::Big(_));
        matches!(b.0, TypedDecimal::Big(_));
        assert!(a.is_zero());
        assert!(b.is_zero());
        assert_eq!(a, b);
    }

    #[test]
    fn test_neg_zero() {
        let a = Decimal::from("-0.0");
        let b = Decimal::from("-0.0");
        matches!(a.0, TypedDecimal::Big(_));
        matches!(b.0, TypedDecimal::Big(_));
        assert!(a.is_zero());
        assert!(b.is_zero());
        assert_eq!(a, b);
    }

    #[test]
    fn test_nan_eq() {
        // implicit big decimal NaN
        let a = Decimal::from("nan");
        let b = Decimal::from("nan");
        assert_ne!(a, b);
        assert!(!a.soft_eq(&b));

        // explicit big decimal NaN
        let c = Decimal(TypedDecimal::Big(
            ExtendedBigDecimal::from_string("nan").unwrap(),
        ));
        let d = Decimal(TypedDecimal::Big(
            ExtendedBigDecimal::from_string("nan").unwrap(),
        ));
        assert_ne!(c, d);
        assert!(!c.soft_eq(&d));

        // f32 NaN
        let e = Decimal(TypedDecimal::F32(f32::NAN.into()));
        let f = Decimal(TypedDecimal::F32(f32::NAN.into()));
        assert_ne!(e, f);
        assert!(!e.soft_eq(&f));

        // f64 NaN
        let g = Decimal(TypedDecimal::F64(f64::NAN.into()));
        let h = Decimal(TypedDecimal::F64(f64::NAN.into()));
        assert_ne!(g, h);
        assert!(!g.soft_eq(&h));

        // eq
        assert_ne!(a, c);
        assert_ne!(a, e);
        assert_ne!(a, g);
        assert_ne!(a, h);
        assert_ne!(b, c);
        assert_ne!(b, e);
        assert_ne!(b, g);
        assert_ne!(b, h);
        assert_ne!(c, e);
        assert_ne!(c, g);
        assert_ne!(c, h);
        assert_ne!(d, e);
        assert_ne!(d, g);
        assert_ne!(d, h);
        assert_ne!(e, g);
        assert_ne!(e, h);
        assert_ne!(f, g);
        assert_ne!(f, h);
        assert_ne!(g, h);

        // soft_eq
        assert!(!a.soft_eq(&c));
        assert!(!a.soft_eq(&e));
        assert!(!a.soft_eq(&g));
        assert!(!a.soft_eq(&h));
        assert!(!b.soft_eq(&c));
        assert!(!b.soft_eq(&e));
        assert!(!b.soft_eq(&g));
        assert!(!b.soft_eq(&h));
        assert!(!c.soft_eq(&e));
        assert!(!c.soft_eq(&g));
        assert!(!c.soft_eq(&h));
        assert!(!d.soft_eq(&e));
        assert!(!d.soft_eq(&g));
        assert!(!d.soft_eq(&h));
        assert!(!e.soft_eq(&g));
        assert!(!e.soft_eq(&h));
        assert!(!f.soft_eq(&g));
        assert!(!f.soft_eq(&h));
        assert!(!g.soft_eq(&h));
    }

    #[test]
    fn test_zero_eq() {
        init_logger();
        let a = Decimal::from("0.0");
        let b = Decimal::from("0.0");
        let c = Decimal::from("-0.0");

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_equality() {
        let a = Decimal::from("1.0");
        let b = Decimal::from("1.0");
        let c = Decimal::from("2.0");
        assert!(a.soft_eq(&b));
        assert!(!a.soft_eq(&c));
        assert!(!b.soft_eq(&c));

        let d = Decimal::from("infinity");
        let e = Decimal::from("-infinity");
        assert!(d.soft_eq(&Decimal::from("infinity")));
        assert!(e.soft_eq(&Decimal::from("-infinity")));
    }

    #[test]
    fn test_decimal_addition() {
        let a = Decimal::from("1.0");
        let b = Decimal::from("2.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("3.0"));

        let c = Decimal::from("1.5");
        let d = Decimal::from("2.5");
        let result2 = c + d;
        assert_eq!(result2, Decimal::from(4.0));

        let e = Decimal::from("0.1");
        let f = Decimal::from("0.2");
        let result3 = &e + &f;
        assert_eq!(result3, Decimal::from("0.3"));
    }

    #[test]
    fn test_infinity_calculations() {
        let a = Decimal::from("1.0");
        let b = Decimal::from("infinity");
        let result = a + b;
        assert_eq!(result, Decimal::from("infinity"));

        let a = Decimal::from("infinity");
        let b = Decimal::from("-infinity");
        let result = a + b;
        assert_eq!(result, Decimal::from("nan"));

        let a = Decimal::from("infinity");
        let b = Decimal::from("-0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("infinity"));

        let a = Decimal::from("-infinity");
        let b = Decimal::from("0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("-infinity"));

        let a = Decimal::from("0.0");
        let b = Decimal::from("-0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("0.0"));

        let a = Decimal::from("-0.0");
        let b = Decimal::from("0.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("0.0"));

        let a = Decimal::from("nan");
        let b = Decimal::from("1.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("nan"));

        let a = Decimal::from("1.0");
        let b = Decimal::from("nan");
        let result = a + b;
        assert_eq!(result, Decimal::from("nan"));

        let a = Decimal::from("nan");
        let b = Decimal::from("nan");
        let result = a + b;
        assert_eq!(result, Decimal::from("nan"));

        let a = Decimal::from("-nan");
        let b = Decimal::from("1.0");
        let result = a + b;
        assert_eq!(result, Decimal::from("nan"));
    }

    #[test]
    fn test_large_decimal_addition() {
        let a = Decimal::from("100000000000000000000.00000000000000000001");
        let b = Decimal::from("100000000000000000000.00000000000000000001");
        let result = a + b;
        assert_eq!(
            result,
            Decimal::from("200000000000000000000.00000000000000000002")
        );
    }

    #[test]
    fn test_e_notation_decimal_addition() {
        let a = Decimal::from("1e10");
        let b = Decimal::from("2e10");
        let result = a + b;
        assert_eq!(result, Decimal::from("3e10"));

        let c = Decimal::from("1.5e10");
        let d = Decimal::from("2.5e10");
        let result2 = c + d;
        assert_eq!(result2, Decimal::from("4e10"));
    }
}

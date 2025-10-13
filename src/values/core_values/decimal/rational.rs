use num::BigRational;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use pad::PadStr;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Add, Neg};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rational {
    big_rational: BigRational,
}

impl Serialize for Rational {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.rational_to_string())
    }
}

impl<'de> Deserialize<'de> for Rational {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Try to parse as BigRational
        if let Ok(big_rational) = s.parse::<BigRational>() {
            return Ok(Rational { big_rational });
        }

        Err(serde::de::Error::custom(format!(
            "Failed to parse '{}' as Rational",
            s
        )))
    }
}

impl Rational {
    pub(crate) fn is_integer(&self) -> bool {
        self.big_rational.is_integer()
    }
    pub(crate) fn to_i16(&self) -> Option<i16> {
        self.big_rational.to_i16()
    }
    pub(crate) fn to_i32(&self) -> Option<i32> {
        self.big_rational.to_i32()
    }
    pub(crate) fn to_i64(&self) -> Option<i64> {
        self.big_rational.to_i64()
    }
    pub(crate) fn to_f32(&self) -> f32 {
        self.big_rational.to_f32().unwrap_or(f32::NAN)
    }

    pub(crate) fn is_positive(&self) -> bool {
        self.big_rational.is_positive()
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.big_rational.is_negative()
    }

    pub(crate) fn to_f64(&self) -> f64 {
        self.big_rational.to_f64().unwrap_or(f64::NAN)
    }

    pub(crate) fn from_big_rational(big_rational: BigRational) -> Self {
        Rational { big_rational }
    }
    pub(crate) fn new(numerator: BigInt, denominator: BigInt) -> Self {
        if denominator.is_zero() {
            panic!("Denominator cannot be zero");
        }
        let big_rational = BigRational::new(numerator, denominator);
        Rational { big_rational }
    }

    // TODO #128: support e-notation for large numbers
    // FIXME #341: Improve this, pass args as reference and non mutable
    pub(crate) fn finite_fraction_to_decimal_string(
        mut numerator: BigInt,
        denominator: BigInt,
    ) -> String {
        let mut shift = denominator.to_string().len() as u32; // absolute value

        // TODO #129 more efficient algorithm for this?

        let numerator_is_neg = numerator.is_negative();
        if numerator_is_neg {
            numerator = numerator.abs();
        }
        // get next higher denominator with power of 10

        if !Rational::is_power_of_10(denominator.clone()) {
            let mut found = false;
            for _ in 0..10000 {
                // only try 10000 iterations, works in first iteration in most cases

                // d % 10^x = 0 => solve s

                let new_denominator = BigInt::from(10).pow(shift); // new possible base 10 denominator

                // is integer factor, can use as new denominator
                if &new_denominator % &denominator == BigInt::from(0) {
                    numerator *= &new_denominator / &denominator;
                    found = true;
                    break;
                }
                // try higher denominator
                else {
                    shift += 1;
                }
            }

            if !found {
                return "invalid".to_string();
            }
        } else {
            shift -= 1;
        }

        let string = numerator.to_string().pad(
            shift as usize,
            '0',
            pad::Alignment::Right,
            false,
        );
        let comma_shift = string.len() - shift as usize;
        let p1 = &string[0..comma_shift];
        let p2 = &string[comma_shift..];

        // self.sign
        format!(
            "{}{p1}{}{p2}",
            if numerator_is_neg { "-" } else { "" },
            if p2.is_empty() {
                ".0"
            } else if p1.is_empty() {
                "0."
            } else {
                "."
            }
        )
    }

    pub(crate) fn has_finite_decimal_rep(mut denominator: BigInt) -> bool {
        while denominator
            .mod_floor(&BigInt::from(2u8))
            .eq(&BigInt::from(0u8))
        {
            denominator /= BigInt::from(2u8);
        }
        let i = &mut BigInt::from(3u8);
        while i.pow(2) <= denominator {
            while denominator.mod_floor(i).eq(&BigInt::from(0u8)) {
                denominator /= BigInt::clone(i);
                if (*i).ne(&BigInt::from(2u8)) && (*i).ne(&BigInt::from(5u8)) {
                    return false; // not allowed
                }
            }
            *i += BigInt::from(2u8);
        }
        if denominator > BigInt::from(2u8) {
            // for primes larger than 2
            if denominator.ne(&BigInt::from(2u8))
                && denominator.ne(&BigInt::from(5u8))
            {
                return false; // not allowed
            }
        }

        true
    }

    pub(crate) fn is_power_of_10(mut n: BigInt) -> bool {
        let zero = &BigInt::from(0u8);
        let one = &BigInt::from(1u8);
        let ten = &BigInt::from(10u8);
        while &n > one && &(&n % ten) == zero {
            n /= ten;
        }
        &n == one
    }

    pub(crate) fn rational_to_string(&self) -> String {
        let rational = &self.big_rational;
        if rational.is_zero() {
            return "0.0".to_string();
        }

        let numerator = rational.numer();
        let denominator = rational.denom();

        if Rational::has_finite_decimal_rep(denominator.clone()) {
            // finite decimal representation
            Rational::finite_fraction_to_decimal_string(
                numerator.clone(),
                denominator.clone(),
            )
        } else {
            // fractional representation
            rational.to_string()
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.big_rational.is_zero()
    }

    pub(crate) fn numer(&self) -> BigInt {
        self.big_rational.numer().clone()
    }

    pub(crate) fn denom(&self) -> BigInt {
        self.big_rational.denom().clone()
    }
}

impl Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Rational::from_big_rational(self.big_rational + rhs.big_rational)
    }
}

impl Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rational_to_string())
    }
}

impl Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Rational::from_big_rational(-self.big_rational)
    }
}

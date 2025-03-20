use crate::stdlib::ops::Div;
use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use pad::PadStr;
use std::collections::HashMap; // FIXME no-std

static EXPONENT_MIN: i8 = -128;

static EXPONENT_MAX: i8 = 127;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseUnit {
    // SI base units 0x00 - 0x0f
    SECOND = 0x00,
    METRE = 0x01,
    GRAM = 0x02,
    AMPERE = 0x03,
    KELVIN = 0x04,
    MOLE = 0x05,
    CANDELA = 0x06,

    // Currency units with ISO codes 0xa0 - 0xdf
    EUR = 0xa0,
    USD = 0xa1,
    GBP = 0xa2,
    RUB = 0xa3,
    CNY = 0xa4,
    JPY = 0xa5,

    CMO = 0xc0, // calendar month

    DIMENSIONLESS = 0xff,
}

type UnitFactor = (BaseUnit, i8);
type Unit = Vec<UnitFactor>;

lazy_static! {
    static ref UNIT_SYMBOLS: HashMap<BaseUnit, &'static str> = [
        (BaseUnit::SECOND, "s"),
        (BaseUnit::METRE, "m"),
        (BaseUnit::GRAM, "g"),
        (BaseUnit::AMPERE, "A"),
        (BaseUnit::KELVIN, "K"),
        (BaseUnit::MOLE, "mol"),
        (BaseUnit::CANDELA, "cd"),
        (BaseUnit::EUR, "EUR"),
        (BaseUnit::USD, "USD"),
        (BaseUnit::GBP, "GBP"),
        (BaseUnit::RUB, "RUB"),
        (BaseUnit::CNY, "CNY"),
        (BaseUnit::JPY, "JPY"),
        (BaseUnit::CMO, "mo"),
        (BaseUnit::DIMENSIONLESS, "x"),
    ]
    .iter()
    .copied()
    .collect();
}

#[derive(Clone)]
pub struct Quantity {
    pub sign: bool, // true = positive, false = negative
    pub numerator: BigUint,
    pub denominator: BigUint,

    pub short_divisor: BigUint,

    pub unit: Unit,
}

impl Quantity {
    pub fn to_string(&self, _colorized: bool) -> String {
        self.value_to_string(true, None) + &self.get_unit_string()
    }

    pub fn value_to_string(
        &self,
        alias_factor: bool,
        decimals: Option<u8>,
    ) -> String {
        let (mut numerator, mut denominator) = (
            BigInt::from_biguint(Sign::Plus, self.numerator.clone()),
            BigInt::from_biguint(Sign::Plus, self.denominator.clone()),
        );

        // divide by short_divisor to match alias factor
        if alias_factor {
            (numerator, denominator) = Quantity::normalize_fraction(
                numerator,
                denominator
                    * BigInt::from_biguint(
                        Sign::Plus,
                        self.short_divisor.clone(),
                    ),
            );
        }

        // fixed decimals
        if decimals.is_some() {
            // return numerator/denominator as fixed decimal string
            numerator.div(denominator).to_string()
        }
        // finite decimal representation
        else if Quantity::has_finite_decimal_rep(
            &mut denominator.to_biguint().unwrap(),
        ) {
            // this.#finiteFractionToDecimalString(numerator, denominator)
            self.finite_fraction_to_decimal_string(numerator, denominator)
        }
        // fraction
        else {
            format!("{}/{}", numerator, denominator)
        }
    }

    fn finite_fraction_to_decimal_string(
        &self,
        mut numerator: BigInt,
        denominator: BigInt,
    ) -> String {
        let mut shift = denominator.to_string().len() as u32; // absolute value

        // TODO more efficient algorithm for this?

        // get next higher denominator with power of 10

        if !Quantity::is_power_of_10(denominator.clone()) {
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

        let string = (numerator).to_string().pad(
            shift as usize,
            '0',
            pad::Alignment::Right,
            false,
        );
        let comma_shift = string.len() - shift as usize;
        let p1 = &string[0..comma_shift];
        let p2 = &string[comma_shift..];

        (if self.sign { "" } else { "-" }).to_owned()
            + p1
            + (if p2.len() > 0 {
                if p1.len() > 0 {
                    "."
                } else {
                    "0."
                }
            } else {
                ""
            })
            + p2
    }

    pub fn get_unit_string(&self) -> String {
        let mut formatted = String::new();
        let mut is_first = true;
        let _format_divisor = 1;

        for encoded in &self.unit {
            if is_first {
                formatted += if encoded.1 < 0 { "x/" } else { "" }
            } else {
                formatted += if encoded.1 < 0 { "/" } else { "*" }
            }
            formatted += &Quantity::get_unit_factor_string(encoded, true);

            is_first = false;
        }

        formatted
    }

    fn get_unit_factor_string(unit: &UnitFactor, abs_exponent: bool) -> String {
        let exponent = if abs_exponent { unit.1.abs() } else { unit.1 };
        if exponent == 1 {
            Quantity::get_base_unit_string(unit.0)
        } else {
            format!("{}^{}", Quantity::get_base_unit_string(unit.0), exponent)
        }
    }

    fn get_base_unit_string(base_unit: BaseUnit) -> String {
        UNIT_SYMBOLS.get(&base_unit).unwrap().to_string()
    }

    fn has_finite_decimal_rep(denominator: &mut BigUint) -> bool {
        while denominator
            .mod_floor(&BigUint::from(2u8))
            .eq(&BigUint::from(0u8))
        {
            *denominator /= BigUint::from(2u8);
        }
        let i = &mut BigUint::from(3u8);
        while i.pow(2) <= *denominator {
            while denominator.mod_floor(i).eq(&BigUint::from(0u8)) {
                *denominator /= BigUint::clone(i);
                if (*i).ne(&BigUint::from(2u8)) && (*i).ne(&BigUint::from(5u8))
                {
                    return false; // not allowed
                }
            }
            *i += BigUint::from(2u8);
        }
        if *denominator > BigUint::from(2u8) {
            // for primes larger than 2
            if (*denominator).ne(&BigUint::from(2u8))
                && (*denominator).ne(&BigUint::from(5u8))
            {
                return false; // not allowed
            }
        }

        true
    }

    fn raise_unit_to_power(unit: Unit, power: usize) -> Unit {
        let mut new_exp: Unit = vec![];
        for u in &unit {
            new_exp.push((u.0, u.1 * power as i8));
        }
        new_exp
    }

    fn has_same_dimension(&self, other: &Quantity) -> bool {
        // check if units are equal
        for i in 0..self.unit.len() {
            if self.unit[i].0 != other.unit[i].0
                || self.unit[i].1 != other.unit[i].1
            {
                return false;
            }
        }
        true
    }

    fn equals(&self, other: &Quantity) -> bool {
        return self.has_same_dimension(other)
            && self.denominator == self.denominator
            && self.numerator == other.numerator
            && self.sign == other.sign;
    }

    fn normalize_fraction<'a>(
        mut numerator: BigInt,
        mut denominator: BigInt,
    ) -> (BigInt, BigInt) {
        let zero = &BigInt::from(0u8);
        let one = &BigInt::from(1u8);
        let minus_one = &BigInt::from(-1i8);

        // denominator always positive, numerator has sign
        if &numerator < zero && &denominator < zero {
            numerator = numerator * minus_one;
            denominator = denominator * minus_one;
        } else if &numerator >= zero && &denominator < zero {
            numerator = numerator * BigInt::from(-1i8);
            denominator = denominator * BigInt::from(-1i8);
        }

        // reduce to lowest terms
        let gcd = Quantity::gcd(numerator.clone(), denominator.clone());

        if &gcd > one {
            numerator = numerator / &gcd;
            denominator = denominator / &gcd;
        } else if &gcd < minus_one {
            let gcd2 = gcd * minus_one;
            numerator = numerator / &gcd2;
            denominator = denominator / &gcd2;
        }

        (numerator, denominator)
    }

    // greates common divisor (Euclidian algorithm)
    fn gcd(mut n1: BigInt, mut n2: BigInt) -> BigInt {
        let zero = &BigInt::from(0u8);

        while &n2 != zero {
            let t = n2;
            n2 = &n1 % &t;
            n1 = t;
        }
        n1
    }

    // least common multiple
    fn lcm(n1: BigInt, n2: BigInt) -> BigInt {
        let prod = &n1 * &n2;
        prod / Quantity::gcd(n1, n2)
    }

    fn is_power_of_10(mut n: BigInt) -> bool {
        let zero = &BigInt::from(0u8);
        let one = &BigInt::from(1u8);
        let ten = &BigInt::from(10u8);
        while &n > one && &(&n % ten) == zero {
            n = n / ten;
        }
        &n == one
    }
}

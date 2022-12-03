use std::ops::Div;

use num_bigint::BigUint;
use num_integer::Integer;

static EXPONENT_MIN:i8 = -128;
static EXPONENT_MAX:i8 = 127;

type Unit = Vec<(u8, i8)>;

#[derive(Clone)]
pub struct Quantity {
	pub sign: bool,
	pub numerator: BigUint,
	pub denominator: BigUint,

	pub unit: Unit
}

impl Quantity {
	fn has_finite_decimal_rep(mut denominator:&mut BigUint) -> bool {
        while denominator.mod_floor(&BigUint::from(2u8)).eq(&BigUint::from(0u8)) {
            *denominator /= BigUint::from(2u8);
        }
		let i = &mut BigUint::from(3u8);
        while i.pow(2) <= *denominator {
            while denominator.mod_floor(i).eq(&BigUint::from(0u8)) {
                *denominator /= BigUint::clone(i);
                if (*i).ne(&BigUint::from(2u8)) && (*i).ne(&BigUint::from(5u8)) {
                    return false // not allowed
                }
            }
			*i += BigUint::from(2u8);
        }
        if *denominator > BigUint::from(2u8) { // for primes larger than 2
            if (*denominator).ne(&BigUint::from(2u8)) && (*denominator).ne(&BigUint::from(5u8)) {
                return false // not allowed
            }
        }

        return true;
    }

    fn raise_unit_to_power(unit:Unit, power:usize) -> Unit {
        let mut new_exp:Unit = vec![];
        for u in &unit {
            new_exp.push((u.0, u.1 * power as i8));
        }
        return new_exp;
    }

    fn has_same_dimension(&self, other:&Quantity) -> bool {
        // check if units are equal
        for i in 0..self.unit.len() {
            if self.unit[i].0 != other.unit[i].0 || self.unit[i].1 != other.unit[i].1 {return false}
        }
        return true;
    }


    fn equals(&self, other:&Quantity) -> bool {
        return self.has_same_dimension(other) && self.denominator == self.denominator && self.numerator == other.numerator && self.sign == other.sign;
    }
}
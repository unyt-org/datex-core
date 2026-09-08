use core::ops::Add;

use crate::values::core_values::decimal::rational::Rational;

impl Add for &Rational {
    type Output = Rational;

    fn add(self, rhs: &Rational) -> Self::Output {
        Rational::from_big_rational(&self.big_rational + &rhs.big_rational)
    }
}

impl Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        (&self).add(&rhs)
    }
}
